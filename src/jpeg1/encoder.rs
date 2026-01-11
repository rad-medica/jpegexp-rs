//! JPEG 1 Baseline and Extended Sequential Encoder orchestration.

use crate::error::JpeglsError;
use crate::jpeg1::dct::fdct_8x8;
use crate::jpeg1::huffman::{
    generate_optimal_huffman_table, HuffmanEncoder, HuffmanTable, JpegBitWriter, SymbolFrequencies,
    STD_LUMINANCE_DC_LENGTHS, STD_LUMINANCE_DC_VALUES,
};
use crate::jpeg1::quantization::{
    STD_CHROMINANCE_QUANT_TABLE, STD_LUMINANCE_QUANT_TABLE,
};
use crate::jpeg_stream_writer::JpegStreamWriter;
use crate::FrameInfo;

/// Zigzag scan pattern for 8x8 blocks.
pub const ZIGZAG_ORDER: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

/// Downsample chroma component for 4:2:0 subsampling (half width, half height).
/// Averages 2x2 pixel blocks.
fn downsample_chroma_420(full_res: &[f32], width: usize, height: usize) -> Vec<f32> {
    let sub_width = (width + 1) / 2;
    let sub_height = (height + 1) / 2;
    let mut result = vec![0.0f32; sub_width * sub_height];
    
    for y in 0..sub_height {
        for x in 0..sub_width {
            let x0 = x * 2;
            let y0 = y * 2;
            let x1 = (x0 + 1).min(width - 1);
            let y1 = (y0 + 1).min(height - 1);
            
            let sum = full_res[y0 * width + x0]
                    + full_res[y0 * width + x1]
                    + full_res[y1 * width + x0]
                    + full_res[y1 * width + x1];
            
            result[y * sub_width + x] = sum * 0.25;
        }
    }
    result
}

/// Downsample chroma component for 4:2:2 subsampling (half width, full height).
/// Averages 2x1 pixel blocks horizontally.
fn downsample_chroma_422(full_res: &[f32], width: usize, height: usize) -> Vec<f32> {
    let sub_width = (width + 1) / 2;
    let mut result = vec![0.0f32; sub_width * height];
    
    for y in 0..height {
        for x in 0..sub_width {
            let x0 = x * 2;
            let x1 = (x0 + 1).min(width - 1);
            
            let sum = full_res[y * width + x0]
                    + full_res[y * width + x1];
            
            result[y * sub_width + x] = sum * 0.5;
        }
    }
    result
}

pub struct Jpeg1Encoder {
    huffman: HuffmanEncoder,
    dc_table_lum: HuffmanTable,
    ac_table_lum: HuffmanTable,
    dc_table_chrom: HuffmanTable,
    ac_table_chrom: HuffmanTable,
    pub quantization_table_lum: [u16; 64],
    pub quantization_table_chrom: [u16; 64],
    pub restart_interval: u16,
    pub quality: u8,
    pub bits_per_sample: u8,
    lossless_mode: bool,
    lossless_predictor: u8,
    /// Horizontal sampling factor for Y component (1-4)
    h_samp_y: u8,
    /// Vertical sampling factor for Y component (1-4)
    v_samp_y: u8,
    /// Horizontal sampling factor for Cb/Cr components (1-4)
    h_samp_chroma: u8,
    /// Vertical sampling factor for Cb/Cr components (1-4)
    v_samp_chroma: u8,
    /// Enable optimized Huffman table generation (two-pass encoding)
    optimize_huffman: bool,
}

impl Default for Jpeg1Encoder {
    fn default() -> Self {
        let mut lum = [0u16; 64];
        let mut chrom = [0u16; 64];
        for i in 0..64 {
            lum[i] = STD_LUMINANCE_QUANT_TABLE[i] as u16;
            chrom[i] = STD_CHROMINANCE_QUANT_TABLE[i] as u16;
        }
        Jpeg1Encoder {
            huffman: HuffmanEncoder::new(),
            dc_table_lum: HuffmanTable::standard_luminance_dc(),
            ac_table_lum: HuffmanTable::standard_luminance_ac(),
            dc_table_chrom: HuffmanTable::standard_chrominance_dc(),
            ac_table_chrom: HuffmanTable::standard_chrominance_ac(),
            quantization_table_lum: lum,
            quantization_table_chrom: chrom,
            restart_interval: 0,
            quality: 75,
            bits_per_sample: 8,
            lossless_mode: false,
            lossless_predictor: 1,
            h_samp_y: 1,
            v_samp_y: 1,
            h_samp_chroma: 1,
            v_samp_chroma: 1,
            optimize_huffman: false,
        }
    }
}

impl Jpeg1Encoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_bits_per_sample(&mut self, bits: u8) {
        self.bits_per_sample = bits.clamp(8, 16);
        if self.bits_per_sample > 8 {
            self.dc_table_lum = HuffmanTable::extended_luminance_dc();
            self.dc_table_chrom = HuffmanTable::extended_luminance_dc();
        } else {
            self.dc_table_lum = HuffmanTable::standard_luminance_dc();
            self.dc_table_chrom = HuffmanTable::standard_chrominance_dc();
        }
    }

    pub fn set_restart_interval(&mut self, interval: u16) {
        self.restart_interval = interval;
    }

    pub fn set_quality(&mut self, quality: u8) {
        self.quality = quality.clamp(1, 100);
        let scale = if self.quality < 50 {
            5000.0 / self.quality as f32
        } else {
            200.0 - 2.0 * self.quality as f32
        };

        for i in 0..64 {
            let lum_val =
                ((STD_LUMINANCE_QUANT_TABLE[i] as f32 * scale / 100.0).round() as u16).clamp(1, 65535);
            let chrom_val = ((STD_CHROMINANCE_QUANT_TABLE[i] as f32 * scale / 100.0).round() as u16)
                .clamp(1, 65535);
            self.quantization_table_lum[i] = lum_val;
            self.quantization_table_chrom[i] = chrom_val;
        }
    }

    /// Enable lossless encoding mode with the specified predictor (1-7).
    pub fn set_lossless(&mut self, predictor: u8) {
        self.lossless_mode = true;
        self.lossless_predictor = predictor.clamp(1, 7);
    }

    /// Set chroma subsampling mode.
    /// - `(2, 2, 1, 1)`: 4:2:0 (half resolution horizontally and vertically)
    /// - `(2, 1, 1, 1)`: 4:2:2 (half resolution horizontally only)
    /// - `(1, 1, 1, 1)`: 4:4:4 (no subsampling, default)
    pub fn set_subsampling(&mut self, h_y: u8, v_y: u8, h_chroma: u8, v_chroma: u8) {
        self.h_samp_y = h_y.clamp(1, 4);
        self.v_samp_y = v_y.clamp(1, 4);
        self.h_samp_chroma = h_chroma.clamp(1, 4);
        self.v_samp_chroma = v_chroma.clamp(1, 4);
    }

    /// Convenience method: Set 4:2:0 subsampling (most common for web/photography).
    pub fn set_subsampling_420(&mut self) {
        self.set_subsampling(2, 2, 1, 1);
    }

    /// Convenience method: Set 4:2:2 subsampling (common for video).
    pub fn set_subsampling_422(&mut self) {
        self.set_subsampling(2, 1, 1, 1);
    }

    /// Convenience method: Set 4:4:4 (no subsampling, highest quality).
    pub fn set_subsampling_444(&mut self) {
        self.set_subsampling(1, 1, 1, 1);
    }

    /// Enable optimized Huffman table generation (two-pass encoding).
    /// This provides 5-15% file size reduction but requires encoding the image twice.
    pub fn set_optimize_huffman(&mut self, enable: bool) {
        self.optimize_huffman = enable;
    }

    pub fn encode(
        &mut self,
        source: &[u8],
        frame_info: &FrameInfo,
        destination: &mut [u8],
    ) -> Result<usize, JpeglsError> {
        if self.lossless_mode {
            return self.encode_lossless(source, frame_info, destination);
        }
        let mut writer = JpegStreamWriter::new(destination);
        let components_count = frame_info.component_count as usize;

        // === OPTIMIZED HUFFMAN PASS 1 ===
        if self.optimize_huffman && !self.lossless_mode {
            let mut lum_dc_freqs = SymbolFrequencies::new();
            let mut lum_ac_freqs = SymbolFrequencies::new();
            let mut chrom_dc_freqs = SymbolFrequencies::new();
            let mut chrom_ac_freqs = SymbolFrequencies::new();
            let mut dc_prev = [0i16; 3]; // 0: Y, 1: Cb, 2: Cr
            let level_shift = (1 << (frame_info.bits_per_sample - 1)) as f32;
            let width = frame_info.width as usize;
            let height = frame_info.height as usize;
            let mut mcus_encoded = 0;
            let total_mcus = if components_count == 1 {
                 ((height + 7) / 8) * ((width + 7) / 8)
            } else {
                 let mcu_width = 8 * self.h_samp_y as usize;
                 let mcu_height = 8 * self.v_samp_y as usize;
                 ((height + mcu_height - 1) / mcu_height) * ((width + mcu_width - 1) / mcu_width)
            };

            if components_count == 1 {
                for block_y in (0..height).step_by(8) {
                    for block_x in (0..width).step_by(8) {
                        // Handle restarts
                        if self.restart_interval > 0 && mcus_encoded > 0 && (mcus_encoded % self.restart_interval as usize == 0) && mcus_encoded < total_mcus {
                            dc_prev[0] = 0;
                        }

                        let mut block_data = [0.0f32; 64];
                        for y in 0..8 {
                            for x in 0..8 {
                                let py = block_y + y;
                                let px = block_x + x;
                                if py < height && px < width {
                                    block_data[y * 8 + x] = source[py * width + px] as f32 - level_shift;
                                }
                            }
                        }
                        Self::collect_block_statistics(&block_data, &self.quantization_table_lum, &mut lum_dc_freqs, &mut lum_ac_freqs, &mut dc_prev[0]);
                        mcus_encoded += 1;
                    }
                }
            } else {
                // RGB/YCbCr
                let mut y_plane = vec![0.0f32; width * height];
                let mut cb_plane = vec![0.0f32; width * height];
                let mut cr_plane = vec![0.0f32; width * height];
                
                for py in 0..height {
                    for px in 0..width {
                        let idx = (py * width + px) * 3;
                        let r = source[idx] as f32;
                        let g = source[idx + 1] as f32;
                        let b = source[idx + 2] as f32;
                        let luma = 0.299 * r + 0.587 * g + 0.114 * b;
                        let cb = -0.1687 * r - 0.3313 * g + 0.5 * b + 128.0;
                        let cr = 0.5 * r - 0.4187 * g - 0.0813 * b + 128.0;
                        y_plane[py * width + px] = luma - 128.0;
                        cb_plane[py * width + px] = cb - 128.0;
                        cr_plane[py * width + px] = cr - 128.0;
                    }
                }

                // Downsample logic (copied)
                let (cb_downsampled, cb_width, cb_height) = if self.h_samp_y > self.h_samp_chroma || self.v_samp_y > self.v_samp_chroma {
                    if self.h_samp_y == 2 && self.v_samp_y == 2 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                        (downsample_chroma_420(&cb_plane, width, height), (width + 1) / 2, (height + 1) / 2)
                    } else if self.h_samp_y == 2 && self.v_samp_y == 1 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                        (downsample_chroma_422(&cb_plane, width, height), (width + 1) / 2, height)
                    } else {
                        (cb_plane, width, height)
                    }
                } else {
                    (cb_plane, width, height)
                };
                
                let (cr_downsampled, _, _) = if self.h_samp_y > self.h_samp_chroma || self.v_samp_y > self.v_samp_chroma {
                    if self.h_samp_y == 2 && self.v_samp_y == 2 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                        (downsample_chroma_420(&cr_plane, width, height), (width + 1) / 2, (height + 1) / 2)
                    } else if self.h_samp_y == 2 && self.v_samp_y == 1 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                        (downsample_chroma_422(&cr_plane, width, height), (width + 1) / 2, height)
                    } else {
                        (cr_plane, width, height)
                    }
                } else {
                    (cr_plane, width, height)
                };

                let mcu_width = 8 * self.h_samp_y as usize;
                let mcu_height = 8 * self.v_samp_y as usize;
                let mcu_cols = (width + mcu_width - 1) / mcu_width;
                let mcu_rows = (height + mcu_height - 1) / mcu_height;

                for mcu_row in 0..mcu_rows {
                    for mcu_col in 0..mcu_cols {
                        if self.restart_interval > 0 && mcus_encoded > 0 && (mcus_encoded % self.restart_interval as usize == 0) && mcus_encoded < total_mcus {
                            dc_prev = [0; 3];
                        }

                        // Y blocks
                        for v in 0..self.v_samp_y {
                            for h in 0..self.h_samp_y {
                                let block_x = mcu_col * mcu_width + h as usize * 8;
                                let block_y = mcu_row * mcu_height + v as usize * 8;
                                let mut block_data = [0.0f32; 64];
                                for y in 0..8 {
                                    for x in 0..8 {
                                        let px = block_x + x;
                                        let py = block_y + y;
                                        if py < height && px < width {
                                            block_data[y * 8 + x] = y_plane[py * width + px];
                                        }
                                    }
                                }
                                Self::collect_block_statistics(&block_data, &self.quantization_table_lum, &mut lum_dc_freqs, &mut lum_ac_freqs, &mut dc_prev[0]);
                            }
                        }

                        // Cb blocks
                        for v in 0..self.v_samp_chroma {
                            for h in 0..self.h_samp_chroma {
                                let block_x = mcu_col * (mcu_width / (self.h_samp_y / self.h_samp_chroma) as usize) + h as usize * 8;
                                let block_y = mcu_row * (mcu_height / (self.v_samp_y / self.v_samp_chroma) as usize) + v as usize * 8;
                                let mut block_data = [0.0f32; 64];
                                for y in 0..8 {
                                    for x in 0..8 {
                                        let px = block_x + x;
                                        let py = block_y + y;
                                        if py < cb_height && px < cb_width {
                                            block_data[y * 8 + x] = cb_downsampled[py * cb_width + px];
                                        }
                                    }
                                }
                                Self::collect_block_statistics(&block_data, &self.quantization_table_chrom, &mut chrom_dc_freqs, &mut chrom_ac_freqs, &mut dc_prev[1]);
                            }
                        }

                        // Cr blocks
                        for v in 0..self.v_samp_chroma {
                            for h in 0..self.h_samp_chroma {
                                let block_x = mcu_col * (mcu_width / (self.h_samp_y / self.h_samp_chroma) as usize) + h as usize * 8;
                                let block_y = mcu_row * (mcu_height / (self.v_samp_y / self.v_samp_chroma) as usize) + v as usize * 8;
                                let mut block_data = [0.0f32; 64];
                                for y in 0..8 {
                                    for x in 0..8 {
                                        let px = block_x + x;
                                        let py = block_y + y;
                                        if py < cb_height && px < cb_width {
                                            block_data[y * 8 + x] = cr_downsampled[py * cb_width + px];
                                        }
                                    }
                                }
                                Self::collect_block_statistics(&block_data, &self.quantization_table_chrom, &mut chrom_dc_freqs, &mut chrom_ac_freqs, &mut dc_prev[2]);
                            }
                        }
                        mcus_encoded += 1;
                    }
                }
            }

            // Generate optimal tables
            let (dc_lum_lens_vec, dc_lum_vals) = generate_optimal_huffman_table(&lum_dc_freqs.dc_freqs, 16);
            let (ac_lum_lens_vec, ac_lum_vals) = generate_optimal_huffman_table(&lum_ac_freqs.ac_freqs, 256);
            let (dc_chrom_lens_vec, dc_chrom_vals) = generate_optimal_huffman_table(&chrom_dc_freqs.dc_freqs, 16);
            let (ac_chrom_lens_vec, ac_chrom_vals) = generate_optimal_huffman_table(&chrom_ac_freqs.ac_freqs, 256);

            let mut dc_lum_lens = [0u8; 16];
            for (i, &len) in dc_lum_lens_vec.iter().enumerate().take(16) { dc_lum_lens[i] = len; }
            let mut ac_lum_lens = [0u8; 16];
            for (i, &len) in ac_lum_lens_vec.iter().enumerate().take(16) { ac_lum_lens[i] = len; }
            let mut dc_chrom_lens = [0u8; 16];
            for (i, &len) in dc_chrom_lens_vec.iter().enumerate().take(16) { dc_chrom_lens[i] = len; }
            let mut ac_chrom_lens = [0u8; 16];
            for (i, &len) in ac_chrom_lens_vec.iter().enumerate().take(16) { ac_chrom_lens[i] = len; }

            self.dc_table_lum = HuffmanTable::build_from_dht(&dc_lum_lens, &dc_lum_vals);
            self.ac_table_lum = HuffmanTable::build_from_dht(&ac_lum_lens, &ac_lum_vals);
            self.dc_table_chrom = HuffmanTable::build_from_dht(&dc_chrom_lens, &dc_chrom_vals);
            self.ac_table_chrom = HuffmanTable::build_from_dht(&ac_chrom_lens, &ac_chrom_vals);
        }

        writer.write_start_of_image()?;

        if components_count == 1 {
            writer.write_dqt_u16(0, &self.quantization_table_lum)?;
        } else {
            writer.write_dqt_u16(0, &self.quantization_table_lum)?;
            writer.write_dqt_u16(1, &self.quantization_table_chrom)?;
        }

        // Write Huffman Tables (Luminance)
        if self.bits_per_sample > 8 {
            writer.write_dht(0, 0, &self.dc_table_lum.lengths, &self.dc_table_lum.values)?;
        } else {
            writer.write_dht(0, 0, &self.dc_table_lum.lengths, &self.dc_table_lum.values)?;
        }
        writer.write_dht(1, 0, &self.ac_table_lum.lengths, &self.ac_table_lum.values)?;

        if components_count > 1 {
            writer.write_dht(0, 1, &self.dc_table_chrom.lengths, &self.dc_table_chrom.values)?;
            writer.write_dht(1, 1, &self.ac_table_chrom.lengths, &self.ac_table_chrom.values)?;
        }

        if self.restart_interval > 0 {
            writer.write_dri(self.restart_interval)?;
        }

        // Write SOF segment with sampling factors
        let sampling_factors = if components_count > 1 {
            vec![
                (self.h_samp_y, self.v_samp_y),
                (self.h_samp_chroma, self.v_samp_chroma),
                (self.h_samp_chroma, self.v_samp_chroma),
            ]
        } else {
            vec![(1, 1)]
        };
        
        if self.bits_per_sample > 8 {
            writer.write_sof1_segment_with_sampling(frame_info, &sampling_factors)?;
        } else {
            writer.write_sof0_segment_with_sampling(frame_info, &sampling_factors)?;
        }
        writer.write_sos_segment(frame_info.component_count as u8)?;

        let mut bit_writer_opt = Some(JpegBitWriter::new(writer.remaining_slice()));
        let width = frame_info.width as usize;
        let height = frame_info.height as usize;
        let mut mcus_encoded = 0;
        let mut next_restart_index = 0;

        self.huffman.dc_previous_value = [0; 4];
        let level_shift = (1 << (frame_info.bits_per_sample - 1)) as f32;

        if components_count == 1 {
            // Grayscale encoding (unchanged)
            let total_mcus = ((height + 7) / 8) * ((width + 7) / 8);
            
            for block_y in (0..height).step_by(8) {
                for block_x in (0..width).step_by(8) {
                    if self.restart_interval > 0 && mcus_encoded > 0 && (mcus_encoded % self.restart_interval as usize == 0) && mcus_encoded < total_mcus {
                        let bw = bit_writer_opt.as_mut().ok_or(JpeglsError::InvalidOperation)?;
                        bw.flush()?;
                        let len = bw.len();
                        let _ = bit_writer_opt.take();
                        writer.advance(len);
                        let marker = crate::jpeg_marker_code::JpegMarkerCode::try_from(0xD0 + (next_restart_index % 8)).map_err(|_| JpeglsError::InvalidOperation)?;
                        writer.write_marker(marker)?;
                        next_restart_index += 1;
                        bit_writer_opt = Some(JpegBitWriter::new(writer.remaining_slice()));
                        self.huffman.dc_previous_value = [0; 4];
                    }

                    let bit_writer = bit_writer_opt.as_mut().ok_or(JpeglsError::InvalidOperation)?;
                    let mut block_data = [0.0f32; 64];
                    for y in 0..8 {
                        for x in 0..8 {
                            let py = block_y + y;
                            let px = block_x + x;
                            if py < height && px < width {
                                block_data[y * 8 + x] = source[py * width + px] as f32 - level_shift;
                            }
                        }
                    }
                    Self::encode_block_internal(&mut self.huffman, &block_data, bit_writer, &self.dc_table_lum, &self.ac_table_lum, &self.quantization_table_lum, 0)?;
                    mcus_encoded += 1;
                }
            }
        } else {
            // RGB/YCbCr encoding with potential subsampling
            // Step 1: Convert RGB to YCbCr planar format
            let mut y_plane = vec![0.0f32; width * height];
            let mut cb_plane = vec![0.0f32; width * height];
            let mut cr_plane = vec![0.0f32; width * height];
            
            for py in 0..height {
                for px in 0..width {
                    let idx = (py * width + px) * 3;
                    let r = source[idx] as f32;
                    let g = source[idx + 1] as f32;
                    let b = source[idx + 2] as f32;
                    let luma = 0.299 * r + 0.587 * g + 0.114 * b;
                    let cb = -0.1687 * r - 0.3313 * g + 0.5 * b + 128.0;
                    let cr = 0.5 * r - 0.4187 * g - 0.0813 * b + 128.0;
                    y_plane[py * width + px] = luma - 128.0;
                    cb_plane[py * width + px] = cb - 128.0;
                    cr_plane[py * width + px] = cr - 128.0;
                }
            }
            
            // Step 2: Downsample chroma if needed
            let (cb_downsampled, cb_width, cb_height) = if self.h_samp_y > self.h_samp_chroma || self.v_samp_y > self.v_samp_chroma {
                if self.h_samp_y == 2 && self.v_samp_y == 2 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                    // 4:2:0
                    (downsample_chroma_420(&cb_plane, width, height), (width + 1) / 2, (height + 1) / 2)
                } else if self.h_samp_y == 2 && self.v_samp_y == 1 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                    // 4:2:2
                    (downsample_chroma_422(&cb_plane, width, height), (width + 1) / 2, height)
                } else {
                    // No subsampling or unsupported mode - use 4:4:4
                    (cb_plane.clone(), width, height)
                }
            } else {
                (cb_plane.clone(), width, height)
            };
            
            let (cr_downsampled, _, _) = if self.h_samp_y > self.h_samp_chroma || self.v_samp_y > self.v_samp_chroma {
                if self.h_samp_y == 2 && self.v_samp_y == 2 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                    // 4:2:0
                    (downsample_chroma_420(&cr_plane, width, height), (width + 1) / 2, (height + 1) / 2)
                } else if self.h_samp_y == 2 && self.v_samp_y == 1 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                    // 4:2:2
                    (downsample_chroma_422(&cr_plane, width, height), (width + 1) / 2, height)
                } else {
                    (cr_plane.clone(), width, height)
                }
            } else {
                (cr_plane.clone(), width, height)
            };
            
            // Step 3: Calculate MCU dimensions
            let mcu_width = 8 * self.h_samp_y as usize;
            let mcu_height = 8 * self.v_samp_y as usize;
            let mcu_cols = (width + mcu_width - 1) / mcu_width;
            let mcu_rows = (height + mcu_height - 1) / mcu_height;
            let total_mcus = mcu_rows * mcu_cols;
            
            // Step 4: Encode MCUs
            for mcu_row in 0..mcu_rows {
                for mcu_col in 0..mcu_cols {
                    if self.restart_interval > 0 && mcus_encoded > 0 && (mcus_encoded % self.restart_interval as usize == 0) && mcus_encoded < total_mcus {
                        let bw = bit_writer_opt.as_mut().ok_or(JpeglsError::InvalidOperation)?;
                        bw.flush()?;
                        let len = bw.len();
                        let _ = bit_writer_opt.take();
                        writer.advance(len);
                        let marker = crate::jpeg_marker_code::JpegMarkerCode::try_from(0xD0 + (next_restart_index % 8)).map_err(|_| JpeglsError::InvalidOperation)?;
                        writer.write_marker(marker)?;
                        next_restart_index += 1;
                        bit_writer_opt = Some(JpegBitWriter::new(writer.remaining_slice()));
                        self.huffman.dc_previous_value = [0; 4];
                    }

                    let bit_writer = bit_writer_opt.as_mut().ok_or(JpeglsError::InvalidOperation)?;
                    
                    // Encode Y blocks (h_samp_y * v_samp_y blocks per MCU)
                    for v in 0..self.v_samp_y {
                        for h in 0..self.h_samp_y {
                            let block_x = mcu_col * mcu_width + h as usize * 8;
                            let block_y = mcu_row * mcu_height + v as usize * 8;
                            let mut block_data = [0.0f32; 64];
                            
                            for y in 0..8 {
                                for x in 0..8 {
                                    let px = block_x + x;
                                    let py = block_y + y;
                                    if py < height && px < width {
                                        block_data[y * 8 + x] = y_plane[py * width + px];
                                    }
                                }
                            }
                            Self::encode_block_internal(&mut self.huffman, &block_data, bit_writer, &self.dc_table_lum, &self.ac_table_lum, &self.quantization_table_lum, 0)?;
                        }
                    }
                    
                    // Encode Cb blocks (h_samp_chroma * v_samp_chroma blocks per MCU)
                    for v in 0..self.v_samp_chroma {
                        for h in 0..self.h_samp_chroma {
                            let block_x = mcu_col * (mcu_width / (self.h_samp_y / self.h_samp_chroma) as usize) + h as usize * 8;
                            let block_y = mcu_row * (mcu_height / (self.v_samp_y / self.v_samp_chroma) as usize) + v as usize * 8;
                            let mut block_data = [0.0f32; 64];
                            
                            for y in 0..8 {
                                for x in 0..8 {
                                    let px = block_x + x;
                                    let py = block_y + y;
                                    if py < cb_height && px < cb_width {
                                        block_data[y * 8 + x] = cb_downsampled[py * cb_width + px];
                                    }
                                }
                            }
                            Self::encode_block_internal(&mut self.huffman, &block_data, bit_writer, &self.dc_table_chrom, &self.ac_table_chrom, &self.quantization_table_chrom, 1)?;
                        }
                    }
                    
                    // Encode Cr blocks (h_samp_chroma * v_samp_chroma blocks per MCU)
                    for v in 0..self.v_samp_chroma {
                        for h in 0..self.h_samp_chroma {
                            let block_x = mcu_col * (mcu_width / (self.h_samp_y / self.h_samp_chroma) as usize) + h as usize * 8;
                            let block_y = mcu_row * (mcu_height / (self.v_samp_y / self.v_samp_chroma) as usize) + v as usize * 8;
                            let mut block_data = [0.0f32; 64];
                            
                            for y in 0..8 {
                                for x in 0..8 {
                                    let px = block_x + x;
                                    let py = block_y + y;
                                    if py < cb_height && px < cb_width {
                                        block_data[y * 8 + x] = cr_downsampled[py * cb_width + px];
                                    }
                                }
                            }
                            Self::encode_block_internal(&mut self.huffman, &block_data, bit_writer, &self.dc_table_chrom, &self.ac_table_chrom, &self.quantization_table_chrom, 2)?;
                        }
                    }
                    
                    mcus_encoded += 1;
                }
            }
        }

        let mut bw = bit_writer_opt.take().ok_or(JpeglsError::InvalidOperation)?;
        bw.flush()?;
        let encoded_len = bw.len();
        writer.advance(encoded_len);
        writer.write_end_of_image()?;
        Ok(writer.len())
    }

    pub fn encode_u16(
        &mut self,
        source: &[u16],
        frame_info: &FrameInfo,
        destination: &mut [u8],
    ) -> Result<usize, JpeglsError> {
        if self.lossless_mode {
            return self.encode_lossless_u16(source, frame_info, destination);
        }
        let mut writer = JpegStreamWriter::new(destination);
        let components_count = frame_info.component_count as usize;

        // === OPTIMIZED HUFFMAN PASS 1 (u16) ===
        if self.optimize_huffman && !self.lossless_mode {
            let mut lum_dc_freqs = SymbolFrequencies::new();
            let mut lum_ac_freqs = SymbolFrequencies::new();
            let mut chrom_dc_freqs = SymbolFrequencies::new();
            let mut chrom_ac_freqs = SymbolFrequencies::new();
            let mut dc_prev = [0i16; 3];
            let level_shift = (1 << (frame_info.bits_per_sample - 1)) as f32;
            let width = frame_info.width as usize;
            let height = frame_info.height as usize;
            let mut mcus_encoded = 0;
            let total_mcus = if components_count == 1 {
                 ((height + 7) / 8) * ((width + 7) / 8)
            } else {
                 let mcu_width = 8 * self.h_samp_y as usize;
                 let mcu_height = 8 * self.v_samp_y as usize;
                 ((height + mcu_height - 1) / mcu_height) * ((width + mcu_width - 1) / mcu_width)
            };

            if components_count == 1 {
                for block_y in (0..height).step_by(8) {
                    for block_x in (0..width).step_by(8) {
                        if self.restart_interval > 0 && mcus_encoded > 0 && (mcus_encoded % self.restart_interval as usize == 0) && mcus_encoded < total_mcus {
                            dc_prev[0] = 0;
                        }

                        let mut block_data = [0.0f32; 64];
                        for y in 0..8 {
                            for x in 0..8 {
                                let py = block_y + y;
                                let px = block_x + x;
                                if py < height && px < width {
                                    block_data[y * 8 + x] = source[py * width + px] as f32 - level_shift;
                                }
                            }
                        }
                        Self::collect_block_statistics(&block_data, &self.quantization_table_lum, &mut lum_dc_freqs, &mut lum_ac_freqs, &mut dc_prev[0]);
                        mcus_encoded += 1;
                    }
                }
            } else {
                // RGB/YCbCr
                let mut y_plane = vec![0.0f32; width * height];
                let mut cb_plane = vec![0.0f32; width * height];
                let mut cr_plane = vec![0.0f32; width * height];
                
                for py in 0..height {
                    for px in 0..width {
                        let idx = (py * width + px) * components_count;
                        let r = source[idx] as f32;
                        let g = source[idx + 1] as f32;
                        let b = source[idx + 2] as f32;
                        let luma = 0.299 * r + 0.587 * g + 0.114 * b;
                        let cb = -0.1687 * r - 0.3313 * g + 0.5 * b + level_shift;
                        let cr = 0.5 * r - 0.4187 * g - 0.0813 * b + level_shift;
                        y_plane[py * width + px] = luma - level_shift;
                        cb_plane[py * width + px] = cb - level_shift;
                        cr_plane[py * width + px] = cr - level_shift;
                    }
                }

                // Downsample logic
                let (cb_downsampled, cb_width, cb_height) = if self.h_samp_y > self.h_samp_chroma || self.v_samp_y > self.v_samp_chroma {
                    if self.h_samp_y == 2 && self.v_samp_y == 2 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                        (downsample_chroma_420(&cb_plane, width, height), (width + 1) / 2, (height + 1) / 2)
                    } else if self.h_samp_y == 2 && self.v_samp_y == 1 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                        (downsample_chroma_422(&cb_plane, width, height), (width + 1) / 2, height)
                    } else {
                        (cb_plane, width, height)
                    }
                } else {
                    (cb_plane, width, height)
                };
                
                let (cr_downsampled, _, _) = if self.h_samp_y > self.h_samp_chroma || self.v_samp_y > self.v_samp_chroma {
                    if self.h_samp_y == 2 && self.v_samp_y == 2 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                        (downsample_chroma_420(&cr_plane, width, height), (width + 1) / 2, (height + 1) / 2)
                    } else if self.h_samp_y == 2 && self.v_samp_y == 1 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                        (downsample_chroma_422(&cr_plane, width, height), (width + 1) / 2, height)
                    } else {
                        (cr_plane, width, height)
                    }
                } else {
                    (cr_plane, width, height)
                };

                let mcu_width = 8 * self.h_samp_y as usize;
                let mcu_height = 8 * self.v_samp_y as usize;
                let mcu_cols = (width + mcu_width - 1) / mcu_width;
                let mcu_rows = (height + mcu_height - 1) / mcu_height;

                for mcu_row in 0..mcu_rows {
                    for mcu_col in 0..mcu_cols {
                        if self.restart_interval > 0 && mcus_encoded > 0 && (mcus_encoded % self.restart_interval as usize == 0) && mcus_encoded < total_mcus {
                            dc_prev = [0; 3];
                        }

                        // Y blocks
                        for v in 0..self.v_samp_y {
                            for h in 0..self.h_samp_y {
                                let block_x = mcu_col * mcu_width + h as usize * 8;
                                let block_y = mcu_row * mcu_height + v as usize * 8;
                                let mut block_data = [0.0f32; 64];
                                for y in 0..8 {
                                    for x in 0..8 {
                                        let px = block_x + x;
                                        let py = block_y + y;
                                        if py < height && px < width {
                                            block_data[y * 8 + x] = y_plane[py * width + px];
                                        }
                                    }
                                }
                                Self::collect_block_statistics(&block_data, &self.quantization_table_lum, &mut lum_dc_freqs, &mut lum_ac_freqs, &mut dc_prev[0]);
                            }
                        }

                        // Cb blocks
                        for v in 0..self.v_samp_chroma {
                            for h in 0..self.h_samp_chroma {
                                let block_x = mcu_col * (mcu_width / (self.h_samp_y / self.h_samp_chroma) as usize) + h as usize * 8;
                                let block_y = mcu_row * (mcu_height / (self.v_samp_y / self.v_samp_chroma) as usize) + v as usize * 8;
                                let mut block_data = [0.0f32; 64];
                                for y in 0..8 {
                                    for x in 0..8 {
                                        let px = block_x + x;
                                        let py = block_y + y;
                                        if py < cb_height && px < cb_width {
                                            block_data[y * 8 + x] = cb_downsampled[py * cb_width + px];
                                        }
                                    }
                                }
                                Self::collect_block_statistics(&block_data, &self.quantization_table_chrom, &mut chrom_dc_freqs, &mut chrom_ac_freqs, &mut dc_prev[1]);
                            }
                        }

                        // Cr blocks
                        for v in 0..self.v_samp_chroma {
                            for h in 0..self.h_samp_chroma {
                                let block_x = mcu_col * (mcu_width / (self.h_samp_y / self.h_samp_chroma) as usize) + h as usize * 8;
                                let block_y = mcu_row * (mcu_height / (self.v_samp_y / self.v_samp_chroma) as usize) + v as usize * 8;
                                let mut block_data = [0.0f32; 64];
                                for y in 0..8 {
                                    for x in 0..8 {
                                        let px = block_x + x;
                                        let py = block_y + y;
                                        if py < cb_height && px < cb_width {
                                            block_data[y * 8 + x] = cr_downsampled[py * cb_width + px];
                                        }
                                    }
                                }
                                Self::collect_block_statistics(&block_data, &self.quantization_table_chrom, &mut chrom_dc_freqs, &mut chrom_ac_freqs, &mut dc_prev[2]);
                            }
                        }
                        mcus_encoded += 1;
                    }
                }
            }

            // Generate optimal tables
            let (dc_lum_lens_vec, dc_lum_vals) = generate_optimal_huffman_table(&lum_dc_freqs.dc_freqs, 16);
            let (ac_lum_lens_vec, ac_lum_vals) = generate_optimal_huffman_table(&lum_ac_freqs.ac_freqs, 256);
            let (dc_chrom_lens_vec, dc_chrom_vals) = generate_optimal_huffman_table(&chrom_dc_freqs.dc_freqs, 16);
            let (ac_chrom_lens_vec, ac_chrom_vals) = generate_optimal_huffman_table(&chrom_ac_freqs.ac_freqs, 256);

            let mut dc_lum_lens = [0u8; 16];
            for (i, &len) in dc_lum_lens_vec.iter().enumerate().take(16) { dc_lum_lens[i] = len; }
            let mut ac_lum_lens = [0u8; 16];
            for (i, &len) in ac_lum_lens_vec.iter().enumerate().take(16) { ac_lum_lens[i] = len; }
            let mut dc_chrom_lens = [0u8; 16];
            for (i, &len) in dc_chrom_lens_vec.iter().enumerate().take(16) { dc_chrom_lens[i] = len; }
            let mut ac_chrom_lens = [0u8; 16];
            for (i, &len) in ac_chrom_lens_vec.iter().enumerate().take(16) { ac_chrom_lens[i] = len; }

            self.dc_table_lum = HuffmanTable::build_from_dht(&dc_lum_lens, &dc_lum_vals);
            self.ac_table_lum = HuffmanTable::build_from_dht(&ac_lum_lens, &ac_lum_vals);
            self.dc_table_chrom = HuffmanTable::build_from_dht(&dc_chrom_lens, &dc_chrom_vals);
            self.ac_table_chrom = HuffmanTable::build_from_dht(&ac_chrom_lens, &ac_chrom_vals);
        }

        writer.write_start_of_image()?;

        if components_count == 1 {
            writer.write_dqt_u16(0, &self.quantization_table_lum)?;
        } else {
            writer.write_dqt_u16(0, &self.quantization_table_lum)?;
            writer.write_dqt_u16(1, &self.quantization_table_chrom)?;
        }

        // Write Huffman Tables (Luminance)
        if self.bits_per_sample > 8 {
            writer.write_dht(0, 0, &self.dc_table_lum.lengths, &self.dc_table_lum.values)?;
        } else {
            writer.write_dht(0, 0, &self.dc_table_lum.lengths, &self.dc_table_lum.values)?;
        }
        writer.write_dht(1, 0, &self.ac_table_lum.lengths, &self.ac_table_lum.values)?;

        if components_count > 1 {
            writer.write_dht(0, 1, &self.dc_table_chrom.lengths, &self.dc_table_chrom.values)?;
            writer.write_dht(1, 1, &self.ac_table_chrom.lengths, &self.ac_table_chrom.values)?;
        }

        if self.restart_interval > 0 {
            writer.write_dri(self.restart_interval)?;
        }

        // Write SOF segment with sampling factors
        let sampling_factors = if components_count > 1 {
            vec![
                (self.h_samp_y, self.v_samp_y),
                (self.h_samp_chroma, self.v_samp_chroma),
                (self.h_samp_chroma, self.v_samp_chroma),
            ]
        } else {
            vec![(1, 1)]
        };
        
        if frame_info.bits_per_sample > 8 {
            writer.write_sof1_segment_with_sampling(frame_info, &sampling_factors)?;
        } else {
            writer.write_sof0_segment_with_sampling(frame_info, &sampling_factors)?;
        }
        writer.write_sos_segment(frame_info.component_count as u8)?;

        let mut bit_writer_opt = Some(JpegBitWriter::new(writer.remaining_slice()));
        let width = frame_info.width as usize;
        let height = frame_info.height as usize;
        let mut mcus_encoded = 0;
        let mut next_restart_index = 0;

        self.huffman.dc_previous_value = [0; 4];
        let level_shift = (1 << (frame_info.bits_per_sample - 1)) as f32;

        if components_count == 1 {
            // Grayscale encoding (unchanged)
            let total_mcus = ((height + 7) / 8) * ((width + 7) / 8);
            
            for block_y in (0..height).step_by(8) {
                for block_x in (0..width).step_by(8) {
                    if self.restart_interval > 0 && mcus_encoded > 0 && (mcus_encoded % self.restart_interval as usize == 0) && mcus_encoded < total_mcus {
                        let bw = bit_writer_opt.as_mut().ok_or(JpeglsError::InvalidOperation)?;
                        bw.flush()?;
                        let len = bw.len();
                        let _ = bit_writer_opt.take();
                        writer.advance(len);
                        let marker = crate::jpeg_marker_code::JpegMarkerCode::try_from(0xD0 + (next_restart_index % 8)).map_err(|_| JpeglsError::InvalidOperation)?;
                        writer.write_marker(marker)?;
                        next_restart_index += 1;
                        bit_writer_opt = Some(JpegBitWriter::new(writer.remaining_slice()));
                        self.huffman.dc_previous_value = [0; 4];
                    }

                    let bit_writer = bit_writer_opt.as_mut().ok_or(JpeglsError::InvalidOperation)?;
                    let mut block_data = [0.0f32; 64];
                    for y in 0..8 {
                        for x in 0..8 {
                            let py = block_y + y;
                            let px = block_x + x;
                            if py < height && px < width {
                                block_data[y * 8 + x] = source[py * width + px] as f32 - level_shift;
                            }
                        }
                    }
                    Self::encode_block_internal(&mut self.huffman, &block_data, bit_writer, &self.dc_table_lum, &self.ac_table_lum, &self.quantization_table_lum, 0)?;
                    mcus_encoded += 1;
                }
            }
        } else {
            // RGB/YCbCr encoding with potential subsampling
            // Step 1: Convert RGB to YCbCr planar format
            let mut y_plane = vec![0.0f32; width * height];
            let mut cb_plane = vec![0.0f32; width * height];
            let mut cr_plane = vec![0.0f32; width * height];
            
            for py in 0..height {
                for px in 0..width {
                    let idx = (py * width + px) * components_count;
                    let r = source[idx] as f32;
                    let g = source[idx + 1] as f32;
                    let b = source[idx + 2] as f32;
                    let luma = 0.299 * r + 0.587 * g + 0.114 * b;
                    let cb = -0.1687 * r - 0.3313 * g + 0.5 * b + level_shift;
                    let cr = 0.5 * r - 0.4187 * g - 0.0813 * b + level_shift;
                    y_plane[py * width + px] = luma - level_shift;
                    cb_plane[py * width + px] = cb - level_shift;
                    cr_plane[py * width + px] = cr - level_shift;
                }
            }
            
            // Step 2: Downsample chroma if needed
            let (cb_downsampled, cb_width, cb_height) = if self.h_samp_y > self.h_samp_chroma || self.v_samp_y > self.v_samp_chroma {
                if self.h_samp_y == 2 && self.v_samp_y == 2 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                    // 4:2:0
                    (downsample_chroma_420(&cb_plane, width, height), (width + 1) / 2, (height + 1) / 2)
                } else if self.h_samp_y == 2 && self.v_samp_y == 1 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                    // 4:2:2
                    (downsample_chroma_422(&cb_plane, width, height), (width + 1) / 2, height)
                } else {
                    // No subsampling or unsupported mode - use 4:4:4
                    (cb_plane.clone(), width, height)
                }
            } else {
                (cb_plane.clone(), width, height)
            };
            
            let (cr_downsampled, _, _) = if self.h_samp_y > self.h_samp_chroma || self.v_samp_y > self.v_samp_chroma {
                if self.h_samp_y == 2 && self.v_samp_y == 2 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                    // 4:2:0
                    (downsample_chroma_420(&cr_plane, width, height), (width + 1) / 2, (height + 1) / 2)
                } else if self.h_samp_y == 2 && self.v_samp_y == 1 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                    // 4:2:2
                    (downsample_chroma_422(&cr_plane, width, height), (width + 1) / 2, height)
                } else {
                    (cr_plane.clone(), width, height)
                }
            } else {
                (cr_plane.clone(), width, height)
            };
            
            // Step 3: Calculate MCU dimensions
            let mcu_width = 8 * self.h_samp_y as usize;
            let mcu_height = 8 * self.v_samp_y as usize;
            let mcu_cols = (width + mcu_width - 1) / mcu_width;
            let mcu_rows = (height + mcu_height - 1) / mcu_height;
            let total_mcus = mcu_rows * mcu_cols;
            
            // Step 4: Encode MCUs
            for mcu_row in 0..mcu_rows {
                for mcu_col in 0..mcu_cols {
                    if self.restart_interval > 0 && mcus_encoded > 0 && (mcus_encoded % self.restart_interval as usize == 0) && mcus_encoded < total_mcus {
                        let bw = bit_writer_opt.as_mut().ok_or(JpeglsError::InvalidOperation)?;
                        bw.flush()?;
                        let len = bw.len();
                        let _ = bit_writer_opt.take();
                        writer.advance(len);
                        let marker = crate::jpeg_marker_code::JpegMarkerCode::try_from(0xD0 + (next_restart_index % 8)).map_err(|_| JpeglsError::InvalidOperation)?;
                        writer.write_marker(marker)?;
                        next_restart_index += 1;
                        bit_writer_opt = Some(JpegBitWriter::new(writer.remaining_slice()));
                        self.huffman.dc_previous_value = [0; 4];
                    }

                    let bit_writer = bit_writer_opt.as_mut().ok_or(JpeglsError::InvalidOperation)?;
                    
                    // Encode Y blocks (h_samp_y * v_samp_y blocks per MCU)
                    for v in 0..self.v_samp_y {
                        for h in 0..self.h_samp_y {
                            let block_x = mcu_col * mcu_width + h as usize * 8;
                            let block_y = mcu_row * mcu_height + v as usize * 8;
                            let mut block_data = [0.0f32; 64];
                            
                            for y in 0..8 {
                                for x in 0..8 {
                                    let px = block_x + x;
                                    let py = block_y + y;
                                    if py < height && px < width {
                                        block_data[y * 8 + x] = y_plane[py * width + px];
                                    }
                                }
                            }
                            Self::encode_block_internal(&mut self.huffman, &block_data, bit_writer, &self.dc_table_lum, &self.ac_table_lum, &self.quantization_table_lum, 0)?;
                        }
                    }
                    
                    // Encode Cb blocks (h_samp_chroma * v_samp_chroma blocks per MCU)
                    for v in 0..self.v_samp_chroma {
                        for h in 0..self.h_samp_chroma {
                            let block_x = mcu_col * (mcu_width / (self.h_samp_y / self.h_samp_chroma) as usize) + h as usize * 8;
                            let block_y = mcu_row * (mcu_height / (self.v_samp_y / self.v_samp_chroma) as usize) + v as usize * 8;
                            let mut block_data = [0.0f32; 64];
                            
                            for y in 0..8 {
                                for x in 0..8 {
                                    let px = block_x + x;
                                    let py = block_y + y;
                                    if py < cb_height && px < cb_width {
                                        block_data[y * 8 + x] = cb_downsampled[py * cb_width + px];
                                    }
                                }
                            }
                            Self::encode_block_internal(&mut self.huffman, &block_data, bit_writer, &self.dc_table_chrom, &self.ac_table_chrom, &self.quantization_table_chrom, 1)?;
                        }
                    }
                    
                    // Encode Cr blocks (h_samp_chroma * v_samp_chroma blocks per MCU)
                    for v in 0..self.v_samp_chroma {
                        for h in 0..self.h_samp_chroma {
                            let block_x = mcu_col * (mcu_width / (self.h_samp_y / self.h_samp_chroma) as usize) + h as usize * 8;
                            let block_y = mcu_row * (mcu_height / (self.v_samp_y / self.v_samp_chroma) as usize) + v as usize * 8;
                            let mut block_data = [0.0f32; 64];
                            
                            for y in 0..8 {
                                for x in 0..8 {
                                    let px = block_x + x;
                                    let py = block_y + y;
                                    if py < cb_height && px < cb_width {
                                        block_data[y * 8 + x] = cr_downsampled[py * cb_width + px];
                                    }
                                }
                            }
                            Self::encode_block_internal(&mut self.huffman, &block_data, bit_writer, &self.dc_table_chrom, &self.ac_table_chrom, &self.quantization_table_chrom, 2)?;
                        }
                    }
                    
                    mcus_encoded += 1;
                }
            }
        }

        let mut bw = bit_writer_opt.take().ok_or(JpeglsError::InvalidOperation)?;
        bw.flush()?;
        let encoded_len = bw.len();
        writer.advance(encoded_len);
        writer.write_end_of_image()?;
        Ok(writer.len())
    }

    pub fn encode_planar(
        &mut self,
        source: &[u8],
        frame_info: &FrameInfo,
        destination: &mut [u8],
    ) -> Result<usize, JpeglsError> {
        let mut writer = JpegStreamWriter::new(destination);
        let components_count = frame_info.component_count as usize;

        // === OPTIMIZED HUFFMAN PASS 1 ===
        if self.optimize_huffman && !self.lossless_mode {
            let mut lum_dc_freqs = SymbolFrequencies::new();
            let mut lum_ac_freqs = SymbolFrequencies::new();
            let mut chrom_dc_freqs = SymbolFrequencies::new();
            let mut chrom_ac_freqs = SymbolFrequencies::new();
            let mut dc_prev = [0i16; 3]; // 0: Y, 1: Cb, 2: Cr
            let level_shift = (1 << (frame_info.bits_per_sample - 1)) as f32;
            let width = frame_info.width as usize;
            let height = frame_info.height as usize;
            let mut mcus_encoded = 0;
            let total_mcus = if components_count == 1 {
                 ((height + 7) / 8) * ((width + 7) / 8)
            } else {
                 let mcu_width = 8 * self.h_samp_y as usize;
                 let mcu_height = 8 * self.v_samp_y as usize;
                 ((height + mcu_height - 1) / mcu_height) * ((width + mcu_width - 1) / mcu_width)
            };

            if components_count == 1 {
                for block_y in (0..height).step_by(8) {
                    for block_x in (0..width).step_by(8) {
                        // Handle restarts
                        if self.restart_interval > 0 && mcus_encoded > 0 && (mcus_encoded % self.restart_interval as usize == 0) && mcus_encoded < total_mcus {
                            dc_prev[0] = 0;
                        }

                        let mut block_data = [0.0f32; 64];
                        for y in 0..8 {
                            for x in 0..8 {
                                let py = block_y + y;
                                let px = block_x + x;
                                if py < height && px < width {
                                    block_data[y * 8 + x] = source[py * width + px] as f32 - level_shift;
                                }
                            }
                        }
                        Self::collect_block_statistics(&block_data, &self.quantization_table_lum, &mut lum_dc_freqs, &mut lum_ac_freqs, &mut dc_prev[0]);
                        mcus_encoded += 1;
                    }
                }
            } else {
                // RGB/YCbCr
                let mut y_plane = vec![0.0f32; width * height];
                let mut cb_plane = vec![0.0f32; width * height];
                let mut cr_plane = vec![0.0f32; width * height];
                
                for py in 0..height {
                    for px in 0..width {
                        let idx = (py * width + px) * 3;
                        let r = source[idx] as f32;
                        let g = source[idx + 1] as f32;
                        let b = source[idx + 2] as f32;
                        let luma = 0.299 * r + 0.587 * g + 0.114 * b;
                        let cb = -0.1687 * r - 0.3313 * g + 0.5 * b + 128.0;
                        let cr = 0.5 * r - 0.4187 * g - 0.0813 * b + 128.0;
                        y_plane[py * width + px] = luma - 128.0;
                        cb_plane[py * width + px] = cb - 128.0;
                        cr_plane[py * width + px] = cr - 128.0;
                    }
                }

                // Downsample logic (copied)
                let (cb_downsampled, cb_width, cb_height) = if self.h_samp_y > self.h_samp_chroma || self.v_samp_y > self.v_samp_chroma {
                    if self.h_samp_y == 2 && self.v_samp_y == 2 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                        (downsample_chroma_420(&cb_plane, width, height), (width + 1) / 2, (height + 1) / 2)
                    } else if self.h_samp_y == 2 && self.v_samp_y == 1 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                        (downsample_chroma_422(&cb_plane, width, height), (width + 1) / 2, height)
                    } else {
                        (cb_plane, width, height)
                    }
                } else {
                    (cb_plane, width, height)
                };
                
                let (cr_downsampled, _, _) = if self.h_samp_y > self.h_samp_chroma || self.v_samp_y > self.v_samp_chroma {
                    if self.h_samp_y == 2 && self.v_samp_y == 2 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                        (downsample_chroma_420(&cr_plane, width, height), (width + 1) / 2, (height + 1) / 2)
                    } else if self.h_samp_y == 2 && self.v_samp_y == 1 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                        (downsample_chroma_422(&cr_plane, width, height), (width + 1) / 2, height)
                    } else {
                        (cr_plane, width, height)
                    }
                } else {
                    (cr_plane, width, height)
                };

                let mcu_width = 8 * self.h_samp_y as usize;
                let mcu_height = 8 * self.v_samp_y as usize;
                let mcu_cols = (width + mcu_width - 1) / mcu_width;
                let mcu_rows = (height + mcu_height - 1) / mcu_height;

                for mcu_row in 0..mcu_rows {
                    for mcu_col in 0..mcu_cols {
                        if self.restart_interval > 0 && mcus_encoded > 0 && (mcus_encoded % self.restart_interval as usize == 0) && mcus_encoded < total_mcus {
                            dc_prev = [0; 3];
                        }

                        // Y blocks
                        for v in 0..self.v_samp_y {
                            for h in 0..self.h_samp_y {
                                let block_x = mcu_col * mcu_width + h as usize * 8;
                                let block_y = mcu_row * mcu_height + v as usize * 8;
                                let mut block_data = [0.0f32; 64];
                                for y in 0..8 {
                                    for x in 0..8 {
                                        let px = block_x + x;
                                        let py = block_y + y;
                                        if py < height && px < width {
                                            block_data[y * 8 + x] = y_plane[py * width + px];
                                        }
                                    }
                                }
                                Self::collect_block_statistics(&block_data, &self.quantization_table_lum, &mut lum_dc_freqs, &mut lum_ac_freqs, &mut dc_prev[0]);
                            }
                        }

                        // Cb blocks
                        for v in 0..self.v_samp_chroma {
                            for h in 0..self.h_samp_chroma {
                                let block_x = mcu_col * (mcu_width / (self.h_samp_y / self.h_samp_chroma) as usize) + h as usize * 8;
                                let block_y = mcu_row * (mcu_height / (self.v_samp_y / self.v_samp_chroma) as usize) + v as usize * 8;
                                let mut block_data = [0.0f32; 64];
                                for y in 0..8 {
                                    for x in 0..8 {
                                        let px = block_x + x;
                                        let py = block_y + y;
                                        if py < cb_height && px < cb_width {
                                            block_data[y * 8 + x] = cb_downsampled[py * cb_width + px];
                                        }
                                    }
                                }
                                Self::collect_block_statistics(&block_data, &self.quantization_table_chrom, &mut chrom_dc_freqs, &mut chrom_ac_freqs, &mut dc_prev[1]);
                            }
                        }

                        // Cr blocks
                        for v in 0..self.v_samp_chroma {
                            for h in 0..self.h_samp_chroma {
                                let block_x = mcu_col * (mcu_width / (self.h_samp_y / self.h_samp_chroma) as usize) + h as usize * 8;
                                let block_y = mcu_row * (mcu_height / (self.v_samp_y / self.v_samp_chroma) as usize) + v as usize * 8;
                                let mut block_data = [0.0f32; 64];
                                for y in 0..8 {
                                    for x in 0..8 {
                                        let px = block_x + x;
                                        let py = block_y + y;
                                        if py < cb_height && px < cb_width {
                                            block_data[y * 8 + x] = cr_downsampled[py * cb_width + px];
                                        }
                                    }
                                }
                                Self::collect_block_statistics(&block_data, &self.quantization_table_chrom, &mut chrom_dc_freqs, &mut chrom_ac_freqs, &mut dc_prev[2]);
                            }
                        }
                        mcus_encoded += 1;
                    }
                }
            }

            // Generate optimal tables
            let (dc_lum_lens_vec, dc_lum_vals) = generate_optimal_huffman_table(&lum_dc_freqs.dc_freqs, 16);
            let (ac_lum_lens_vec, ac_lum_vals) = generate_optimal_huffman_table(&lum_ac_freqs.ac_freqs, 256);
            let (dc_chrom_lens_vec, dc_chrom_vals) = generate_optimal_huffman_table(&chrom_dc_freqs.dc_freqs, 16);
            let (ac_chrom_lens_vec, ac_chrom_vals) = generate_optimal_huffman_table(&chrom_ac_freqs.ac_freqs, 256);

            let mut dc_lum_lens = [0u8; 16];
            for (i, &len) in dc_lum_lens_vec.iter().enumerate().take(16) { dc_lum_lens[i] = len; }
            let mut ac_lum_lens = [0u8; 16];
            for (i, &len) in ac_lum_lens_vec.iter().enumerate().take(16) { ac_lum_lens[i] = len; }
            let mut dc_chrom_lens = [0u8; 16];
            for (i, &len) in dc_chrom_lens_vec.iter().enumerate().take(16) { dc_chrom_lens[i] = len; }
            let mut ac_chrom_lens = [0u8; 16];
            for (i, &len) in ac_chrom_lens_vec.iter().enumerate().take(16) { ac_chrom_lens[i] = len; }

            self.dc_table_lum = HuffmanTable::build_from_dht(&dc_lum_lens, &dc_lum_vals);
            self.ac_table_lum = HuffmanTable::build_from_dht(&ac_lum_lens, &ac_lum_vals);
            self.dc_table_chrom = HuffmanTable::build_from_dht(&dc_chrom_lens, &dc_chrom_vals);
            self.ac_table_chrom = HuffmanTable::build_from_dht(&ac_chrom_lens, &ac_chrom_vals);
        }

        writer.write_start_of_image()?;

        if components_count == 1 {
            writer.write_dqt_u16(0, &self.quantization_table_lum)?;
        } else {
            writer.write_dqt_u16(0, &self.quantization_table_lum)?;
            writer.write_dqt_u16(1, &self.quantization_table_chrom)?;
        }

        // Write Huffman Tables (Luminance)
        if self.bits_per_sample > 8 {
            writer.write_dht(0, 0, &self.dc_table_lum.lengths, &self.dc_table_lum.values)?;
        } else {
            // println!("Writing Lum DC: {} vals", self.dc_table_lum.values.len());
            writer.write_dht(0, 0, &self.dc_table_lum.lengths, &self.dc_table_lum.values)?;
        }
        // println!("Writing Lum AC: {} vals", self.ac_table_lum.values.len());
        writer.write_dht(1, 0, &self.ac_table_lum.lengths, &self.ac_table_lum.values)?;
        if components_count > 1 {
            writer.write_dht(0, 1, &crate::jpeg1::huffman::STD_CHROMINANCE_DC_LENGTHS, &crate::jpeg1::huffman::STD_CHROMINANCE_DC_VALUES)?;
            writer.write_dht(1, 1, &self.ac_table_chrom.lengths, &self.ac_table_chrom.values)?;
        }

        if self.restart_interval > 0 {
            writer.write_dri(self.restart_interval)?;
        }

        if self.bits_per_sample > 8 {
            writer.write_sof1_segment(frame_info)?;
        } else {
            writer.write_sof0_segment(frame_info)?;
        }

        let width = frame_info.width as usize;
        let height = frame_info.height as usize;

        for comp_idx in 0..components_count {
            writer.write_marker(crate::jpeg_marker_code::JpegMarkerCode::StartOfScan)?;
            let length = 2 + 1 + 2 + 3;
            writer.write_u16(length as u16)?;
            writer.write_byte(1)?;
            writer.write_byte((comp_idx + 1) as u8)?;

            let (dc_table_id, ac_table_id, quant_table, pred_idx) = if comp_idx == 0 {
                (0x00, 0x00, &self.quantization_table_lum, 0)
            } else {
                (0x11, 0x11, &self.quantization_table_chrom, comp_idx)
            };

            let table_sel = (dc_table_id & 0xF0) | (ac_table_id & 0x0F);
            writer.write_byte(table_sel)?;
            writer.write_byte(0)?;
            writer.write_byte(63)?;
            writer.write_byte(0)?;

            let mut bit_writer_opt = Some(JpegBitWriter::new(writer.remaining_slice()));
            let mut bit_writer = bit_writer_opt.as_mut().ok_or(JpeglsError::InvalidOperation)?;

            self.huffman.dc_previous_value[pred_idx] = 0;
            let mut mcus_encoded = 0;
            let mut next_restart_index = 0;
            let total_blocks = ((height + 7) / 8) * ((width + 7) / 8);

            for block_y in (0..height).step_by(8) {
                for block_x in (0..width).step_by(8) {
                    if self.restart_interval > 0 && mcus_encoded > 0 && (mcus_encoded % self.restart_interval as usize == 0) && mcus_encoded < total_blocks {
                        bit_writer.flush()?;
                        let len = bit_writer.len();
                        let _ = bit_writer_opt.take();
                        writer.advance(len);
                        let marker = crate::jpeg_marker_code::JpegMarkerCode::try_from(0xD0 + (next_restart_index % 8)).map_err(|_| JpeglsError::InvalidOperation)?;
                        writer.write_marker(marker)?;
                        next_restart_index += 1;
                        bit_writer_opt = Some(JpegBitWriter::new(writer.remaining_slice()));
                        bit_writer = bit_writer_opt.as_mut().ok_or(JpeglsError::InvalidOperation)?;
                        self.huffman.dc_previous_value[pred_idx] = 0;
                    }

                    let mut block_data = [0.0f32; 64];
                    for y in 0..8 {
                        for x in 0..8 {
                            let py = block_y + y;
                            let px = block_x + x;
                            if py < height && px < width {
                                if components_count == 1 {
                                    block_data[y * 8 + x] = source[py * width + px] as f32 - 128.0;
                                } else {
                                    let idx = (py * width + px) * 3;
                                    let r = source[idx] as f32;
                                    let g = source[idx + 1] as f32;
                                    let b = source[idx + 2] as f32;
                                    if comp_idx == 0 {
                                        block_data[y * 8 + x] = (0.299 * r + 0.587 * g + 0.114 * b) - 128.0;
                                    } else if comp_idx == 1 {
                                        block_data[y * 8 + x] = (-0.1687 * r - 0.3313 * g + 0.5 * b + 128.0) - 128.0;
                                    } else {
                                        block_data[y * 8 + x] = (0.5 * r - 0.4187 * g - 0.0813 * b + 128.0) - 128.0;
                                    }
                                }
                            }
                        }
                    }

                    let ref_dc = if comp_idx == 0 { &self.dc_table_lum } else { &self.dc_table_chrom };
                    let ref_ac = if comp_idx == 0 { &self.ac_table_lum } else { &self.ac_table_chrom };

                    Self::encode_block_internal(&mut self.huffman, &block_data, bit_writer, ref_dc, ref_ac, quant_table, pred_idx)?;
                    mcus_encoded += 1;
                }
            }
            bit_writer.flush()?;
            let encoded_len = bit_writer.len();
            let _ = bit_writer_opt.take();
            writer.advance(encoded_len);
        }

        writer.write_end_of_image()?;
        Ok(writer.len())
    }

    fn encode_block_internal(
        huffman: &mut HuffmanEncoder,
        block: &[f32; 64],
        bit_writer: &mut JpegBitWriter,
        dc_table: &HuffmanTable,
        ac_table: &HuffmanTable,
        quant_table: &[u16; 64],
        dc_pred_idx: usize,
    ) -> Result<(), JpeglsError> {
        let mut dct_coeffs = [0.0f32; 64];
        fdct_8x8(block, &mut dct_coeffs);

        let mut quant_coeffs = [0i16; 64];
        crate::jpeg1::quantization::quantize_block_u16(&dct_coeffs, quant_table, &mut quant_coeffs);

        let mut zigzag_coeffs = [0i16; 64];
        for i in 0..64 {
            zigzag_coeffs[i] = quant_coeffs[ZIGZAG_ORDER[i]];
        }

        let dc_val = zigzag_coeffs[0];
        let diff = dc_val - huffman.dc_previous_value[dc_pred_idx];
        huffman.dc_previous_value[dc_pred_idx] = dc_val;

        let dc_category = HuffmanEncoder::get_category(diff);
        let dc_code = dc_table.codes[dc_category as usize];
        if dc_code.length == 0 && dc_category > 0 {
             return Err(JpeglsError::InvalidData);
        }
        bit_writer.write_bits(dc_code.value, dc_code.length)?;
        let (dc_bits, dc_bit_len) = HuffmanEncoder::get_diff_bits(diff, dc_category);
        bit_writer.write_bits(dc_bits, dc_bit_len)?;

        let mut run = 0;
        for i in 1..64 {
            let ac_val = zigzag_coeffs[i];
            if ac_val == 0 {
                run += 1;
            } else {
                while run > 15 {
                    let zrl_code = ac_table.codes[0xF0];
                    bit_writer.write_bits(zrl_code.value, zrl_code.length)?;
                    run -= 16;
                }
                let category = HuffmanEncoder::get_category(ac_val);
                let symbol = (run << 4) | (category as usize);
                let ac_code = ac_table.codes[symbol];
                if ac_code.length == 0 {
                    return Err(JpeglsError::InvalidData);
                }
                bit_writer.write_bits(ac_code.value, ac_code.length)?;
                let (ac_bits, ac_bit_len) = HuffmanEncoder::get_diff_bits(ac_val, category);
                bit_writer.write_bits(ac_bits, ac_bit_len)?;
                run = 0;
            }
        }
        if run > 0 {
            let eob_code = ac_table.codes[0x00];
            if eob_code.length == 0 {
                return Err(JpeglsError::InvalidData);
            }
            bit_writer.write_bits(eob_code.value, eob_code.length)?;
        }
        Ok(())
    }

    /// Collect Huffman statistics from a block without encoding
    fn collect_block_statistics(
        block: &[f32; 64],
        quant_table: &[u16; 64],
        dc_freqs: &mut SymbolFrequencies,
        ac_freqs: &mut SymbolFrequencies,
        dc_prev: &mut i16,
    ) {
        // if rand::random::<u8>() < 1 { println!("collecting stats"); } // Debug
        // 1. Perform DCT
        let mut dct_coeffs = [0.0f32; 64];
        fdct_8x8(block, &mut dct_coeffs);

        // 2. Quantize coefficients
        let mut quant_coeffs = [0i16; 64];
        crate::jpeg1::quantization::quantize_block_u16(&dct_coeffs, quant_table, &mut quant_coeffs);

        // 3. Apply zigzag ordering
        let mut zigzag = [0i16; 64];
        for i in 0..64 {
            zigzag[i] = quant_coeffs[ZIGZAG_ORDER[i]];
        }

        // 4. Record DC symbol
        let dc_diff = zigzag[0] - *dc_prev;
        *dc_prev = zigzag[0];
        let dc_category = HuffmanEncoder::get_category(dc_diff);
        dc_freqs.record_dc(dc_category);

        // 5. Record AC symbols (using run-length encoding logic)
        let mut run = 0u8;
        for &coef in &zigzag[1..] {
            if coef == 0 {
                run += 1;
                if run == 16 {
                    ac_freqs.record_ac(0xF0); // ZRL
                    run = 0;
                }
            } else {
                while run >= 16 {
                    ac_freqs.record_ac(0xF0);
                    run -= 16;
                }
                let category = HuffmanEncoder::get_category(coef);
                let symbol = (run << 4) | category;
                ac_freqs.record_ac(symbol);
                run = 0;
            }
        }

        // 6. Record EOB if needed
        if run > 0 {
            ac_freqs.record_ac(0x00); // EOB
        }
    }

    fn encode_lossless(
        &mut self,
        source: &[u8],
        frame_info: &FrameInfo,
        destination: &mut [u8],
    ) -> Result<usize, JpeglsError> {
        use crate::jpeg1::lossless::Jpeg1LosslessEncoder;
        
        let mut writer = JpegStreamWriter::new(destination);
        let components_count = frame_info.component_count as usize;
        let width = frame_info.width as usize;
        let height = frame_info.height as usize;

        writer.write_start_of_image()?;

        // Write Huffman Tables (DC only for lossless)
        if self.bits_per_sample > 8 {
            writer.write_dht(0, 0, &crate::jpeg1::huffman::EXT_LUMINANCE_DC_LENGTHS, &crate::jpeg1::huffman::EXT_LUMINANCE_DC_VALUES)?;
        } else {
            writer.write_dht(0, 0, &STD_LUMINANCE_DC_LENGTHS, &STD_LUMINANCE_DC_VALUES)?;
        }

        if components_count > 1 {
            if self.bits_per_sample > 8 {
                writer.write_dht(0, 1, &crate::jpeg1::huffman::EXT_LUMINANCE_DC_LENGTHS, &crate::jpeg1::huffman::EXT_LUMINANCE_DC_VALUES)?;
            } else {
                writer.write_dht(0, 1, &crate::jpeg1::huffman::STD_CHROMINANCE_DC_LENGTHS, &crate::jpeg1::huffman::STD_CHROMINANCE_DC_VALUES)?;
            }
        }

        if self.restart_interval > 0 {
            writer.write_dri(self.restart_interval)?;
        }

        writer.write_sof3_segment(frame_info, self.lossless_predictor)?;
        writer.write_sos_segment_lossless(frame_info.component_count as u8, self.lossless_predictor)?;

        let mut bit_writer = JpegBitWriter::new(writer.remaining_slice());
        self.huffman.dc_previous_value = [0; 4];

        if components_count == 1 {
            // Grayscale lossless
            let mut pixels = vec![0i32; width * height];
            for i in 0..pixels.len() {
                pixels[i] = source[i] as i32;
            }
            Jpeg1LosslessEncoder::encode_component(
                self.lossless_predictor,
                width,
                height,
                frame_info.bits_per_sample as u8,
                &pixels,
                &mut bit_writer,
                &mut self.huffman,
                &self.dc_table_lum,
                0,
            )?;
        } else {
            // RGB lossless - encode each component directly (no color conversion)
            let mut r_pixels = vec![0i32; width * height];
            let mut g_pixels = vec![0i32; width * height];
            let mut b_pixels = vec![0i32; width * height];
            
            for i in 0..(width * height) {
                let idx = i * 3;
                r_pixels[i] = source[idx] as i32;
                g_pixels[i] = source[idx + 1] as i32;
                b_pixels[i] = source[idx + 2] as i32;
            }
            
            Jpeg1LosslessEncoder::encode_component(
                self.lossless_predictor,
                width,
                height,
                frame_info.bits_per_sample as u8,
                &r_pixels,
                &mut bit_writer,
                &mut self.huffman,
                &self.dc_table_lum,
                0,
            )?;
            
            Jpeg1LosslessEncoder::encode_component(
                self.lossless_predictor,
                width,
                height,
                frame_info.bits_per_sample as u8,
                &g_pixels,
                &mut bit_writer,
                &mut self.huffman,
                &self.dc_table_chrom,
                1,
            )?;
            
            Jpeg1LosslessEncoder::encode_component(
                self.lossless_predictor,
                width,
                height,
                frame_info.bits_per_sample as u8,
                &b_pixels,
                &mut bit_writer,
                &mut self.huffman,
                &self.dc_table_chrom,
                2,
            )?;
        }

        bit_writer.flush()?;
        let encoded_len = bit_writer.len();
        writer.advance(encoded_len);
        writer.write_end_of_image()?;
        Ok(writer.len())
    }

    fn encode_lossless_u16(
        &mut self,
        source: &[u16],
        frame_info: &FrameInfo,
        destination: &mut [u8],
    ) -> Result<usize, JpeglsError> {
        use crate::jpeg1::lossless::Jpeg1LosslessEncoder;
        
        let mut writer = JpegStreamWriter::new(destination);
        let components_count = frame_info.component_count as usize;
        let width = frame_info.width as usize;
        let height = frame_info.height as usize;

        writer.write_start_of_image()?;

        // Write Huffman Tables (DC only for lossless)
        if self.bits_per_sample > 8 {
            writer.write_dht(0, 0, &crate::jpeg1::huffman::EXT_LUMINANCE_DC_LENGTHS, &crate::jpeg1::huffman::EXT_LUMINANCE_DC_VALUES)?;
        } else {
            writer.write_dht(0, 0, &STD_LUMINANCE_DC_LENGTHS, &STD_LUMINANCE_DC_VALUES)?;
        }

        if components_count > 1 {
            if self.bits_per_sample > 8 {
                writer.write_dht(0, 1, &crate::jpeg1::huffman::EXT_LUMINANCE_DC_LENGTHS, &crate::jpeg1::huffman::EXT_LUMINANCE_DC_VALUES)?;
            } else {
                writer.write_dht(0, 1, &crate::jpeg1::huffman::STD_CHROMINANCE_DC_LENGTHS, &crate::jpeg1::huffman::STD_CHROMINANCE_DC_VALUES)?;
            }
        }

        if self.restart_interval > 0 {
            writer.write_dri(self.restart_interval)?;
        }

        writer.write_sof3_segment(frame_info, self.lossless_predictor)?;
        writer.write_sos_segment_lossless(frame_info.component_count as u8, self.lossless_predictor)?;

        let mut bit_writer = JpegBitWriter::new(writer.remaining_slice());
        self.huffman.dc_previous_value = [0; 4];

        if components_count == 1 {
            // Grayscale lossless
            let mut pixels = vec![0i32; width * height];
            for i in 0..pixels.len() {
                pixels[i] = source[i] as i32;
            }
            Jpeg1LosslessEncoder::encode_component(
                self.lossless_predictor,
                width,
                height,
                frame_info.bits_per_sample as u8,
                &pixels,
                &mut bit_writer,
                &mut self.huffman,
                &self.dc_table_lum,
                0,
            )?;
        } else {
            // RGB lossless - encode each component directly (no color conversion)
            let mut r_pixels = vec![0i32; width * height];
            let mut g_pixels = vec![0i32; width * height];
            let mut b_pixels = vec![0i32; width * height];
            
            for i in 0..(width * height) {
                let idx = i * components_count;
                r_pixels[i] = source[idx] as i32;
                g_pixels[i] = source[idx + 1] as i32;
                b_pixels[i] = source[idx + 2] as i32;
            }
            
            Jpeg1LosslessEncoder::encode_component(
                self.lossless_predictor,
                width,
                height,
                frame_info.bits_per_sample as u8,
                &r_pixels,
                &mut bit_writer,
                &mut self.huffman,
                &self.dc_table_lum,
                0,
            )?;
            
            Jpeg1LosslessEncoder::encode_component(
                self.lossless_predictor,
                width,
                height,
                frame_info.bits_per_sample as u8,
                &g_pixels,
                &mut bit_writer,
                &mut self.huffman,
                &self.dc_table_chrom,
                1,
            )?;
            
            Jpeg1LosslessEncoder::encode_component(
                self.lossless_predictor,
                width,
                height,
                frame_info.bits_per_sample as u8,
                &b_pixels,
                &mut bit_writer,
                &mut self.huffman,
                &self.dc_table_chrom,
                2,
            )?;
        }

        bit_writer.flush()?;
        let encoded_len = bit_writer.len();
        writer.advance(encoded_len);
        writer.write_end_of_image()?;
        Ok(writer.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip_grayscale() {
        let width = 16;
        let height = 16;
        let mut source = vec![0u8; width * height];
        for i in 0..source.len() {
            source[i] = (i % 256) as u8;
        }

        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 1,
        };

        let mut encoder = Jpeg1Encoder::new();
        let mut encoded = vec![0u8; 10000];
        let enc_len = encoder
            .encode(&source, &frame_info, &mut encoded)
            .expect("Encode failed");

        let mut decoder = crate::jpeg1::decoder::Jpeg1Decoder::new(&encoded[..enc_len]);
        decoder.read_header().expect("Read header failed");

        let mut decoded = vec![0u8; width * height];
        decoder.decode(&mut decoded).expect("Decode failed");

        for i in 0..source.len() {
            let diff = (source[i] as i32 - decoded[i] as i32).abs();
            assert!(diff < 20, "Mismatch at index {}: src={} dec={} diff={}", i, source[i], decoded[i], diff);
        }
    }

    #[test]
    fn test_encode_decode_roundtrip_color() {
        let width = 16;
        let height = 16;
        let mut source = vec![0u8; width * height * 3];
        for i in 0..(width * height) {
            source[i * 3 + 0] = (i % 256) as u8;
            source[i * 3 + 1] = ((i * 2) % 256) as u8;
            source[i * 3 + 2] = ((255 - i) % 256) as u8;
        }

        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 3,
        };

        let mut encoder = Jpeg1Encoder::new();
        let mut encoded = vec![0u8; 10000];
        let enc_len = encoder
            .encode(&source, &frame_info, &mut encoded)
            .expect("Encode failed");

        let mut decoder = crate::jpeg1::decoder::Jpeg1Decoder::new(&encoded[..enc_len]);
        decoder.read_header().expect("Read header failed");

        let mut decoded = vec![0u8; width * height * 3];
        decoder.decode(&mut decoded).expect("Decode failed");

        let tolerance = 25;
        for i in 0..source.len() {
            let diff = (source[i] as i32 - decoded[i] as i32).abs();
            assert!(diff < tolerance, "Mismatch at index {}: src={} dec={} diff={}", i, source[i], decoded[i], diff);
        }
    }
}
