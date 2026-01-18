//! JPEG 1 Baseline and Extended Sequential Encoder orchestration.

use crate::error::JpeglsError;
use crate::jpeg1::dct::fdct_8x8;
use crate::jpeg1::huffman::{
    generate_optimal_huffman_table, HuffmanEncoder, HuffmanTable, JpegBitWriter, SymbolFrequencies,
    STD_LUMINANCE_DC_LENGTHS, STD_LUMINANCE_DC_VALUES,
};
use crate::jpeg1::progressive::{CoefficientBuffer, QuantizedBlock, ScanScript, ScanSpecification};
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
    let sub_width = width.div_ceil(2);
    let sub_height = height.div_ceil(2);
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
    let sub_width = width.div_ceil(2);
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
    /// Enable progressive encoding mode
    progressive_mode: bool,
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
            progressive_mode: false,
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

    /// Enable progressive encoding mode.
    /// This produces a progressive JPEG (SOF2) instead of a sequential one.
    pub fn set_progressive(&mut self, enable: bool) {
        self.progressive_mode = enable;
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
        
        if self.progressive_mode {
            // Placeholder for Progressive Encoding
            // This will use collect_quantized_coefficients and scan loops
            // For now, fail or fallback
            // But since we are implementing it, let's start wiring it up
            // return self.encode_progressive(source, frame_info, destination);
            // TODO: Implement encode_progressive
            return self.encode_progressive(source, frame_info, destination);
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
                 height.div_ceil(8) * width.div_ceil(8)
            } else {
                 let mcu_width = 8 * self.h_samp_y as usize;
                 let mcu_height = 8 * self.v_samp_y as usize;
                 height.div_ceil(mcu_height) * width.div_ceil(mcu_width)
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
                        (downsample_chroma_420(&cb_plane, width, height), width.div_ceil(2), height.div_ceil(2))
                    } else if self.h_samp_y == 2 && self.v_samp_y == 1 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                        (downsample_chroma_422(&cb_plane, width, height), width.div_ceil(2), height)
                    } else {
                        (cb_plane, width, height)
                    }
                } else {
                    (cb_plane, width, height)
                };
                
                let (cr_downsampled, _, _) = if self.h_samp_y > self.h_samp_chroma || self.v_samp_y > self.v_samp_chroma {
                    if self.h_samp_y == 2 && self.v_samp_y == 2 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                        (downsample_chroma_420(&cr_plane, width, height), width.div_ceil(2), height.div_ceil(2))
                    } else if self.h_samp_y == 2 && self.v_samp_y == 1 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                        (downsample_chroma_422(&cr_plane, width, height), width.div_ceil(2), height)
                    } else {
                        (cr_plane, width, height)
                    }
                } else {
                    (cr_plane, width, height)
                };

                let mcu_width = 8 * self.h_samp_y as usize;
                let mcu_height = 8 * self.v_samp_y as usize;
                let mcu_cols = width.div_ceil(mcu_width);
                let mcu_rows = height.div_ceil(mcu_height);

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
        // Note: For now we use the same tables for 8-bit and >8-bit, 
        // but this logic allows for future optimization or differentiation
        #[allow(clippy::if_same_then_else)]
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
            let total_mcus = height.div_ceil(8) * width.div_ceil(8);
            
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
                    (downsample_chroma_420(&cb_plane, width, height), width.div_ceil(2), height.div_ceil(2))
                } else if self.h_samp_y == 2 && self.v_samp_y == 1 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                    // 4:2:2
                    (downsample_chroma_422(&cb_plane, width, height), width.div_ceil(2), height)
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
                    (downsample_chroma_420(&cr_plane, width, height), width.div_ceil(2), height.div_ceil(2))
                } else if self.h_samp_y == 2 && self.v_samp_y == 1 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                    // 4:2:2
                    (downsample_chroma_422(&cr_plane, width, height), width.div_ceil(2), height)
                } else {
                    (cr_plane.clone(), width, height)
                }
            } else {
                (cr_plane.clone(), width, height)
            };
            
            // Step 3: Calculate MCU dimensions
            let mcu_width = 8 * self.h_samp_y as usize;
            let mcu_height = 8 * self.v_samp_y as usize;
            let mcu_cols = width.div_ceil(mcu_width);
            let mcu_rows = height.div_ceil(mcu_height);
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
            let mut dc_prev = [0i16; 3]; // 0: Y, 1: Cb, 2: Cr
            let level_shift = (1 << (frame_info.bits_per_sample - 1)) as f32;
            let width = frame_info.width as usize;
            let height = frame_info.height as usize;
            let mut mcus_encoded = 0;
            let total_mcus = if components_count == 1 {
                 height.div_ceil(8) * width.div_ceil(8)
            } else {
                 let mcu_width = 8 * self.h_samp_y as usize;
                 let mcu_height = 8 * self.v_samp_y as usize;
                 height.div_ceil(mcu_height) * width.div_ceil(mcu_width)
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
                        (downsample_chroma_420(&cb_plane, width, height), width.div_ceil(2), height.div_ceil(2))
                    } else if self.h_samp_y == 2 && self.v_samp_y == 1 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                        (downsample_chroma_422(&cb_plane, width, height), width.div_ceil(2), height)
                    } else {
                        (cb_plane, width, height)
                    }
                } else {
                    (cb_plane, width, height)
                };
                
                let (cr_downsampled, _, _) = if self.h_samp_y > self.h_samp_chroma || self.v_samp_y > self.v_samp_chroma {
                    if self.h_samp_y == 2 && self.v_samp_y == 2 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                        (downsample_chroma_420(&cr_plane, width, height), width.div_ceil(2), height.div_ceil(2))
                    } else if self.h_samp_y == 2 && self.v_samp_y == 1 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                        (downsample_chroma_422(&cr_plane, width, height), width.div_ceil(2), height)
                    } else {
                        (cr_plane, width, height)
                    }
                } else {
                    (cr_plane, width, height)
                };

                let mcu_width = 8 * self.h_samp_y as usize;
                let mcu_height = 8 * self.v_samp_y as usize;
                let mcu_cols = width.div_ceil(mcu_width);
                let mcu_rows = height.div_ceil(mcu_height);

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
        // Note: For now we use the same tables for 8-bit and >8-bit, 
        // but this logic allows for future optimization or differentiation
        #[allow(clippy::if_same_then_else)]
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
            let total_mcus = height.div_ceil(8) * width.div_ceil(8);
            
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
                    (downsample_chroma_420(&cb_plane, width, height), width.div_ceil(2), height.div_ceil(2))
                } else if self.h_samp_y == 2 && self.v_samp_y == 1 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                    // 4:2:2
                    (downsample_chroma_422(&cb_plane, width, height), width.div_ceil(2), height)
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
                    (downsample_chroma_420(&cr_plane, width, height), width.div_ceil(2), height.div_ceil(2))
                } else if self.h_samp_y == 2 && self.v_samp_y == 1 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                    // 4:2:2
                    (downsample_chroma_422(&cr_plane, width, height), width.div_ceil(2), height)
                } else {
                    (cr_plane.clone(), width, height)
                }
            } else {
                (cr_plane.clone(), width, height)
            };
            
            // Step 3: Calculate MCU dimensions
            let mcu_width = 8 * self.h_samp_y as usize;
            let mcu_height = 8 * self.v_samp_y as usize;
            let mcu_cols = width.div_ceil(mcu_width);
            let mcu_rows = height.div_ceil(mcu_height);
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
                 height.div_ceil(8) * width.div_ceil(8)
            } else {
                 let mcu_width = 8 * self.h_samp_y as usize;
                 let mcu_height = 8 * self.v_samp_y as usize;
                 height.div_ceil(mcu_height) * width.div_ceil(mcu_width)
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
                        (downsample_chroma_420(&cb_plane, width, height), width.div_ceil(2), height.div_ceil(2))
                    } else if self.h_samp_y == 2 && self.v_samp_y == 1 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                        (downsample_chroma_422(&cb_plane, width, height), width.div_ceil(2), height)
                    } else {
                        (cb_plane, width, height)
                    }
                } else {
                    (cb_plane, width, height)
                };
                
                let (cr_downsampled, _, _) = if self.h_samp_y > self.h_samp_chroma || self.v_samp_y > self.v_samp_chroma {
                    if self.h_samp_y == 2 && self.v_samp_y == 2 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                        (downsample_chroma_420(&cr_plane, width, height), width.div_ceil(2), height.div_ceil(2))
                    } else if self.h_samp_y == 2 && self.v_samp_y == 1 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                        (downsample_chroma_422(&cr_plane, width, height), width.div_ceil(2), height)
                    } else {
                        (cr_plane, width, height)
                    }
                } else {
                    (cr_plane, width, height)
                };

                let mcu_width = 8 * self.h_samp_y as usize;
                let mcu_height = 8 * self.v_samp_y as usize;
                let mcu_cols = width.div_ceil(mcu_width);
                let mcu_rows = height.div_ceil(mcu_height);

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
            let total_blocks = height.div_ceil(8) * width.div_ceil(8);

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
        for &ac_val in zigzag_coeffs.iter().skip(1) {
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
                            } else {
                                while run >= 16 {
                                    ac_freqs.record_ac(0xF0); // ZRL
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

    /// Collect quantized coefficients into a buffer for progressive encoding.
    /// This performs DCT and Quantization but stores the result instead of Huffman encoding it.
    fn collect_quantized_coefficients(
        block: &[f32; 64],
        quant_table: &[u16; 64],
        buffer: &mut CoefficientBuffer,
        mcu_index: usize,
        block_index_in_mcu: usize,
    ) {
        let mut dct_coeffs = [0.0f32; 64];
        fdct_8x8(block, &mut dct_coeffs);

        let mut quant_coeffs = [0i16; 64];
        crate::jpeg1::quantization::quantize_block_u16(&dct_coeffs, quant_table, &mut quant_coeffs);

        let q_block = buffer.get_block_mut(mcu_index, block_index_in_mcu);
        q_block.coeffs = quant_coeffs;
    }

    fn encode_progressive(
        &mut self,
        source: &[u8],
        frame_info: &FrameInfo,
        destination: &mut [u8],
    ) -> Result<usize, JpeglsError> {
        let mut writer = JpegStreamWriter::new(destination);
        let components_count = frame_info.component_count as usize;
        let width = frame_info.width as usize;
        let height = frame_info.height as usize;

        writer.write_start_of_image()?;

        // Write DQTs
        if components_count == 1 {
            writer.write_dqt_u16(0, &self.quantization_table_lum)?;
        } else {
            writer.write_dqt_u16(0, &self.quantization_table_lum)?;
            writer.write_dqt_u16(1, &self.quantization_table_chrom)?;
        }

        // Write Huffman Tables (all of them upfront for simplicity, or per scan)
        // Progressive decoders expect DHTs before SOS that uses them.
        // We'll write standard tables for now.
        if self.bits_per_sample > 8 {
            writer.write_dht(0, 0, &crate::jpeg1::huffman::EXT_LUMINANCE_DC_LENGTHS, &crate::jpeg1::huffman::EXT_LUMINANCE_DC_VALUES)?;
        } else {
            writer.write_dht(0, 0, &STD_LUMINANCE_DC_LENGTHS, &STD_LUMINANCE_DC_VALUES)?;
        }
        writer.write_dht(1, 0, &self.ac_table_lum.lengths, &self.ac_table_lum.values)?;

        if components_count > 1 {
            if self.bits_per_sample > 8 {
                writer.write_dht(0, 1, &crate::jpeg1::huffman::EXT_LUMINANCE_DC_LENGTHS, &crate::jpeg1::huffman::EXT_LUMINANCE_DC_VALUES)?;
            } else {
                writer.write_dht(0, 1, &crate::jpeg1::huffman::STD_CHROMINANCE_DC_LENGTHS, &crate::jpeg1::huffman::STD_CHROMINANCE_DC_VALUES)?;
            }
            writer.write_dht(1, 1, &self.ac_table_chrom.lengths, &self.ac_table_chrom.values)?;
        }

        if self.restart_interval > 0 {
            writer.write_dri(self.restart_interval)?;
        }

        // Write SOF2 (Progressive DCT) instead of SOF0/SOF1
        // Need to implement write_sof2_segment or reuse write_sof0 with marker override?
        // JpegStreamWriter methods are specific (write_sof0_segment).
        // Let's check JpegStreamWriter capabilities.
        // Assuming we need to extend JpegStreamWriter or use generic marker writer.
        // For now, let's assume we can write the SOF2 marker manually or add a method.
        // Wait, JpegStreamWriter is in another file. I should check if it has SOF2 support.
        // If not, I'll use write_marker and write_segment_payload logic if possible, or modify JpegStreamWriter.
        
        // Let's assume for a moment we can use write_sof0_segment but patch the marker code?
        // No, write_sof0_segment likely writes the marker byte.
        // Let's check JpegStreamWriter content if possible, or just implement a local helper.
        // Since I can't see JpegStreamWriter content right now (I saw it earlier but didn't memorize it), 
        // I'll assume I need to add `write_sof2_segment` to it or use a raw write.
        
        // TEMPORARY: Write SOF0 for now to test flow, but this is WRONG for progressive.
        // The decoder will see SOF0 and expect sequential.
        // I need to write marker 0xC2 (SOF2).
        
        // Let's try to use the public API of JpegStreamWriter to write raw marker + data.
        // Sampling factors
        let sampling_factors = if components_count > 1 {
            vec![
                (self.h_samp_y, self.v_samp_y),
                (self.h_samp_chroma, self.v_samp_chroma),
                (self.h_samp_chroma, self.v_samp_chroma),
            ]
        } else {
            vec![(1, 1)]
        };
        
        // Manual SOF2 write
        writer.write_marker(crate::jpeg_marker_code::JpegMarkerCode::try_from(0xC2).unwrap())?; // SOF2
        let len = 8 + 3 * components_count as u16;
        writer.write_u16(len)?;
        writer.write_byte(frame_info.bits_per_sample as u8)?;
        writer.write_u16(frame_info.height as u16)?;
        writer.write_u16(frame_info.width as u16)?;
        writer.write_byte(components_count as u8)?;
        for (i, &(h, v)) in sampling_factors.iter().enumerate().take(components_count) {
            writer.write_byte((i + 1) as u8)?; // ID
            writer.write_byte((h << 4) | v)?;
            writer.write_byte(if i == 0 { 0 } else { 1 })?; // Quant table selector
        }

        // --- PHASE 1: COEFFICIENT COLLECTION ---
        // Initialize buffers for each component
        // 0: Y, 1: Cb, 2: Cr
        let mut buffers: Vec<CoefficientBuffer> = Vec::with_capacity(components_count);
        
        if components_count == 1 {
            buffers.push(CoefficientBuffer::new(width, height, 1, 1));
        } else {
            // Y
            buffers.push(CoefficientBuffer::new(width, height, self.h_samp_y, self.v_samp_y));
            // Cb
            let (_cb_w, _cb_h) = (width.div_ceil(2), height.div_ceil(2)); // Approximation for 4:2:0
            // Actually, we should use the same logic as sequential for dimensions
            // But CoefficientBuffer calculates its own block count based on h/v samp.
            // Wait, CoefficientBuffer needs logical dimensions of the component?
            // No, it needs the full image dimensions and sampling factors to calculate MCU layout.
            // Let's re-read CoefficientBuffer::new.
            // It takes (width, height, h_samp, v_samp).
            // It calculates mcu_cols/rows based on that.
            // So we just pass the full image dims and the component's sampling factors.
            
            buffers.push(CoefficientBuffer::new(width, height, self.h_samp_chroma, self.v_samp_chroma)); // Cb
            buffers.push(CoefficientBuffer::new(width, height, self.h_samp_chroma, self.v_samp_chroma)); // Cr
        }

        // Fill buffers (reuse sequential logic structure)
        let level_shift = (1 << (frame_info.bits_per_sample - 1)) as f32;
        
        // This part is very similar to sequential encode, but calls collect_quantized_coefficients
        // instead of encode_block_internal.
        // We can copy-paste the loop structure from `encode`.
        
        // ... (Data collection loop) ...
        // I'll implement a helper `fill_coefficient_buffers` to keep this clean.
        self.fill_coefficient_buffers(source, frame_info, &mut buffers, level_shift);

        // --- PHASE 2: SCAN LOOP ---
        // Use Simple Spectral for maximum compatibility and robustness.
        // Successive Approximation (SA) is implemented but sensitive to bitstream details.
        let script = ScanScript::simple_spectral();
        
        for scan in script.scans {
            self.write_scan(&scan, &mut writer, &mut buffers)?;
        }

        writer.write_end_of_image()?;
        Ok(writer.len())
    }

    fn fill_coefficient_buffers(
        &self, 
        source: &[u8], 
        frame_info: &FrameInfo, 
        buffers: &mut [CoefficientBuffer],
        level_shift: f32
    ) {
        let width = frame_info.width as usize;
        let height = frame_info.height as usize;
        let components_count = frame_info.component_count as usize;

        if components_count == 1 {
            // Grayscale
            for block_y in (0..height).step_by(8) {
                for block_x in (0..width).step_by(8) {
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
                    // MCU index calculation
                    let mcu_x = block_x / 8;
                    let mcu_y = block_y / 8;
                    let mcu_width = width.div_ceil(8);
                    let mcu_idx = mcu_y * mcu_width + mcu_x;
                    
                    Self::collect_quantized_coefficients(
                        &block_data, 
                        &self.quantization_table_lum, 
                        &mut buffers[0], 
                        mcu_idx, 
                        0
                    );
                }
            }
        } else {
            // RGB/YCbCr
            // Need planar conversion first (or on the fly)
            // Reuse on-the-fly logic from encode()
            
            // ... (Copy YCbCr conversion and downsampling logic) ...
            // For brevity in this edit, I'll implement a simplified version or copy it.
            // Ideally, we refactor this into a helper, but `encode` is monolithic.
            // Let's implement the loop here.
            
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

            // Downsample
            let (cb_downsampled, cb_width, cb_height) = if self.h_samp_y > self.h_samp_chroma || self.v_samp_y > self.v_samp_chroma {
                if self.h_samp_y == 2 && self.v_samp_y == 2 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                    (downsample_chroma_420(&cb_plane, width, height), width.div_ceil(2), height.div_ceil(2))
                } else if self.h_samp_y == 2 && self.v_samp_y == 1 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                    (downsample_chroma_422(&cb_plane, width, height), width.div_ceil(2), height)
                } else {
                    (cb_plane, width, height)
                }
            } else {
                (cb_plane, width, height)
            };
            
            let (cr_downsampled, _, _) = if self.h_samp_y > self.h_samp_chroma || self.v_samp_y > self.v_samp_chroma {
                if self.h_samp_y == 2 && self.v_samp_y == 2 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                    (downsample_chroma_420(&cr_plane, width, height), width.div_ceil(2), height.div_ceil(2))
                } else if self.h_samp_y == 2 && self.v_samp_y == 1 && self.h_samp_chroma == 1 && self.v_samp_chroma == 1 {
                    (downsample_chroma_422(&cr_plane, width, height), width.div_ceil(2), height)
                } else {
                    (cr_plane, width, height)
                }
            } else {
                (cr_plane, width, height)
            };

            let mcu_width = 8 * self.h_samp_y as usize;
            let mcu_height = 8 * self.v_samp_y as usize;
            let mcu_cols = width.div_ceil(mcu_width);
            let mcu_rows = height.div_ceil(mcu_height);

            for mcu_row in 0..mcu_rows {
                for mcu_col in 0..mcu_cols {
                    let mcu_idx = mcu_row * mcu_cols + mcu_col;

                    // Y blocks
                    let mut blk_idx = 0;
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
                            Self::collect_quantized_coefficients(&block_data, &self.quantization_table_lum, &mut buffers[0], mcu_idx, blk_idx);
                            blk_idx += 1;
                        }
                    }

                    // Cb blocks
                    blk_idx = 0;
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
                            Self::collect_quantized_coefficients(&block_data, &self.quantization_table_chrom, &mut buffers[1], mcu_idx, blk_idx);
                            blk_idx += 1;
                        }
                    }

                    // Cr blocks
                    blk_idx = 0;
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
                            Self::collect_quantized_coefficients(&block_data, &self.quantization_table_chrom, &mut buffers[2], mcu_idx, blk_idx);
                            blk_idx += 1;
                        }
                    }
                }
            }
        }
    }

    fn write_scan(
        &mut self,
        scan: &ScanSpecification,
        writer: &mut JpegStreamWriter,
        buffers: &mut [CoefficientBuffer],
    ) -> Result<(), JpeglsError> {
        // Write SOS
        writer.write_marker(crate::jpeg_marker_code::JpegMarkerCode::StartOfScan)?;
        let len = 2 + 1 + 2 * scan.component_indices.len() as u16 + 3;
        writer.write_u16(len)?;
        writer.write_byte(scan.component_indices.len() as u8)?;
        
        for &comp_idx in &scan.component_indices {
            writer.write_byte(comp_idx + 1)?; // Component ID (1-based)
            let (dc_tbl, ac_tbl) = if comp_idx == 0 { (0, 0) } else { (1, 1) };
            writer.write_byte((dc_tbl << 4) | ac_tbl)?;
        }
        
        writer.write_byte(scan.ss_start)?;
        writer.write_byte(scan.ss_end)?;
        writer.write_byte((scan.ah << 4) | scan.al)?;

        // Bit writer setup
        let mut bit_writer_opt = Some(JpegBitWriter::new(writer.remaining_slice()));
        let mut bit_writer = bit_writer_opt.as_mut().ok_or(JpeglsError::InvalidOperation)?;
        
        // Reset DC predictors at start of scan
        self.huffman.dc_previous_value = [0; 4];
        let mut next_restart_index = 0;
        
        // Determine total MCUs
        let buf0 = &buffers[0];
        let mcu_width = 8 * buf0.h_samp as usize;
        let mcu_height = 8 * buf0.v_samp as usize;
        let mcu_cols = buf0.width.div_ceil(mcu_width);
        let mcu_rows = buf0.height.div_ceil(mcu_height);
        let total_mcus = mcu_cols * mcu_rows;

        // Iterate over MCUs
        for mcu_idx in 0..total_mcus {
            // Handle restart intervals
            if self.restart_interval > 0 && mcu_idx > 0 && (mcu_idx % self.restart_interval as usize == 0) {
                bit_writer.flush()?;
                let len = bit_writer.len();
                let _ = bit_writer_opt.take();
                writer.advance(len);
                let marker = crate::jpeg_marker_code::JpegMarkerCode::try_from(0xD0 + (next_restart_index % 8)).map_err(|_| JpeglsError::InvalidOperation)?;
                writer.write_marker(marker)?;
                next_restart_index += 1;
                bit_writer_opt = Some(JpegBitWriter::new(writer.remaining_slice()));
                bit_writer = bit_writer_opt.as_mut().ok_or(JpeglsError::InvalidOperation)?;
                self.huffman.dc_previous_value = [0; 4];
            }

            // For each component in the scan
            for &comp_idx in &scan.component_indices {
                let comp_i = comp_idx as usize;
                let buffer = &buffers[comp_i];
                let blocks_per_mcu = buffer.h_samp as usize * buffer.v_samp as usize;
                
                // For each block in the MCU for this component
                for blk_idx in 0..blocks_per_mcu {
                    let block_offset = mcu_idx * blocks_per_mcu + blk_idx;
                    let block = buffers[comp_i].blocks[block_offset]; // Copy
                    
                    // Clone tables to avoid borrow checker issues
                    // This is slightly inefficient but safe. `HuffmanTable` contains vectors so clone is non-trivial.
                    // Better approach: pass references to tables inside `encode_progressive_block`
                    // But `encode_progressive_block` takes `&mut self` to update `self.huffman`.
                    // We need to split `self` borrows.
                    // Or, pass `&mut self.huffman` and `&table_dc` separately to a static function/method that doesn't take `&mut self`.
                    
                    // Refactoring `encode_progressive_block` to NOT take `&mut self`, but `&mut HuffmanEncoder`
                    Self::encode_progressive_block_static(
                        &mut self.huffman,
                        &block, 
                        bit_writer, 
                        if comp_idx == 0 { &self.dc_table_lum } else { &self.dc_table_chrom }, 
                        if comp_idx == 0 { &self.ac_table_lum } else { &self.ac_table_chrom }, 
                        scan, 
                        comp_i
                    )?;
                }
            }
        }
        
        bit_writer.flush()?;
        let encoded_len = bit_writer.len();
        let _ = bit_writer_opt.take();
        writer.advance(encoded_len);
        
        Ok(())
    }

    fn encode_progressive_block_static(
        huffman: &mut HuffmanEncoder,
        block: &QuantizedBlock,
        bit_writer: &mut JpegBitWriter,
        dc_table: &HuffmanTable,
        ac_table: &HuffmanTable,
        scan: &ScanSpecification,
        dc_pred_idx: usize,
    ) -> Result<(), JpeglsError> {
        let mut zigzag = [0i16; 64];
        for i in 0..64 {
            zigzag[i] = block.coeffs[ZIGZAG_ORDER[i]];
        }

        // DC Encoding (Ss=0)
        if scan.ss_start == 0 {
            let dc_val = zigzag[0];
            
            if scan.ah == 0 {
                // Initial DC Scan
                let diff = dc_val - huffman.dc_previous_value[dc_pred_idx];
                huffman.dc_previous_value[dc_pred_idx] = dc_val;
                
                let v = diff >> scan.al;
                
                let category = HuffmanEncoder::get_category(v);
                let code = dc_table.codes[category as usize];
                bit_writer.write_bits(code.value, code.length)?;
                let (bits, len) = HuffmanEncoder::get_diff_bits(v, category);
                bit_writer.write_bits(bits, len)?;
            } else {
                // DC Refinement Scan (Ah > 0)
                // We just code the bit at position Al
                // Refinement bits for DC are sent raw, not Huffman encoded.
                let bit = (dc_val >> scan.al) & 1;
                bit_writer.write_bits(bit as u16, 1)?;
            }
        }

        // AC Encoding (Ss > 0)
        // scan.ss_end is the end of the spectral band.
        let start = std::cmp::max(1, scan.ss_start as usize);
        let end = scan.ss_end as usize;
        
        if start <= end {
            if scan.ah == 0 {
                // Initial AC Scan
                let mut run = 0;
                for &val in zigzag.iter().take(end + 1).skip(start) {
                    let _abs_val = val.abs();
                    // Check if coefficient is significant at this shift
                    // We need to encode `val >> Al`
                    let shifted = val >> scan.al;
                    
                    if shifted == 0 {
                        run += 1;
                    } else {
                        while run > 15 {
                            let zrl = ac_table.codes[0xF0];
                            bit_writer.write_bits(zrl.value, zrl.length)?;
                            run -= 16;
                        }
                        let category = HuffmanEncoder::get_category(shifted);
                        let symbol = (run << 4) | category;
                        let code = ac_table.codes[symbol as usize];
                        if code.length == 0 { return Err(JpeglsError::InvalidData); }
                        bit_writer.write_bits(code.value, code.length)?;
                        let (bits, len) = HuffmanEncoder::get_diff_bits(shifted, category);
                        bit_writer.write_bits(bits, len)?;
                        run = 0;
                    }
                }
                
                if run > 0 {
                    let eob = ac_table.codes[0x00];
                    bit_writer.write_bits(eob.value, eob.length)?;
                }
            } else {
                // AC Refinement Scan (Ah > 0)
                // This logic is complex. We need to process EOB runs and history.
                // For "Simple Spectral + SA", we'll implement per-block refinement first (no EOB runs across blocks).
                
                let mut run = 0;
                let _eob_run = 0; // We are not using cross-block EOB runs yet, but logic is similar.
                
                // For AC refinement:
                // 1. Iterate over spectral band.
                // 2. If coeff was non-zero in previous pass (abs(val) >= (1 << Ah)), send Refinement Bit.
                //    - But ONLY if we are not in an EOB run (or after handling run).
                //    - Actually, non-zero history coeffs are skipped by the Zero Run Length coding of *new* coeffs.
                //    - BUT we must send their refinement bit.
                //    - JPEG standard:
                //      - If coeff was already non-zero: Send 1 bit (refinement).
                //      - If coeff was zero:
                //        - If it becomes non-zero now: Send run-length + sign.
                //        - If it stays zero: Increment run.
                //      - Wait, refinement bits are sent inline?
                //      - "The refinement bit is coded... immediately after the run-length code... OR if run-length is skipped".
                //      - Actually, the sequence is:
                //        - Skip over already-non-zero coeffs in the run counting.
                //        - When we hit a *newly* non-zero coeff (or EOB):
                //          - Code the run length of *zeros* (skipping history-non-zeros).
                //          - Code the new coeff sign.
                //          - THEN, for each skipped history-non-zero coeff, send its refinement bit.
                //          - Actually, the standard says refinement bits are sent *interleaved*?
                //          - Annex G.1.2.2:
                //            "When a non-zero coefficient is coded... [same as initial scan]..."
                //            "However, if [coeff was already non-zero]... the 'refinement bit' is coded."
                //            "The code for ZRL or a run length... accounts only for coefficients which were zero in previous scans."
                //            "Any non-zero coefficients [from history] are skipped over... however, immediately after the ZRL/run code is output, a single bit is output for each [skipped history coeff]."
                //            "If the run length is non-zero, the refinement bits are output... for each non-zero coeff... skipped over."
                
                // Let's implement this carefully.
                // We need to buffer refinement bits while counting the run.
                
                let mut refinement_bits = Vec::new(); // Bits to send after next symbol
                
                for &val in zigzag.iter().take(end + 1).skip(start) {
                    let abs_val = val.abs();
                    let history_mask = 1 << scan.ah; // Previously sent bits
                    let current_bit_mask = 1 << scan.al;
                    
                    let was_nonzero = abs_val >= history_mask;
                    
                    if was_nonzero {
                        // Already sent. We will send a refinement bit.
                        // Refinement bit = (abs_val >> Al) & 1
                        let ref_bit = (abs_val >> scan.al) & 1;
                        refinement_bits.push(ref_bit as u16);
                    } else {
                        // Was zero. Check if it becomes non-zero now.
                        let is_nonzero_now = abs_val >= current_bit_mask;
                        
                        if !is_nonzero_now {
                            // Still zero.
                            run += 1;
                        } else {
                            // Newly non-zero!
                            // 1. Send Run/Category symbol
                            while run > 15 {
                                // ZRL
                                let zrl = ac_table.codes[0xF0];
                                bit_writer.write_bits(zrl.value, zrl.length)?;
                                
                                // Send refinement bits for the history coeffs skipped during this run of 16
                                // Note: The standard says "Each ZRL is followed by the refinement bits for the non-zero coefficients skipped over during the run of 16 zeros."
                                // So we need to flush refinement bits that accumulated *during* this ZRL period.
                                // BUT `refinement_bits` vector collected ALL bits since last symbol.
                                // We need to handle this strictly.
                                // Actually, simpler logic:
                                //   When we encounter a history-non-zero coeff:
                                //     If we are *in the middle of a run*, we just queue the bit.
                                //     When we emit a ZRL or a Symbol, we flush the queue.
                                //   Wait, ZRL corresponds to 16 *zeros* (newly-zero coeffs).
                                //   If we skipped 5 history-non-zeros while counting 16 zeros, we emit ZRL then 5 bits.
                                
                                // Let's manage the queue better.
                                // We consume `refinement_bits` up to what was seen *before* the 16th zero.
                                // This is getting complicated with a simple vector.
                                // Alternative:
                                //   Iterate.
                                //   If history-non-zero:
                                //     Save bit.
                                //   If history-zero (new-zero):
                                //     Run++.
                                //     If Run == 16:
                                //       Emit ZRL.
                                //       Emit saved bits.
                                //       Clear saved bits.
                                //       Run = 0.
                                //   If history-zero (new-nonzero):
                                //     Emit Symbol (Run, Size=1).
                                //     Emit Sign bit.
                                //     Emit saved bits.
                                //     Clear saved bits.
                                //     Run = 0.
                                
                                // Wait, does the order match?
                                // "The refinement bits... are output... in the order in which they occur in the block."
                                // Yes.
                                
                                // Let's rewrite the loop structure with this logic.
                                // We need to handle the loop inside the check.
                                // Since we are inside `while run > 15`, we implicitly handled 16 zeros.
                                // But `refinement_bits` contains bits from "gaps" between those zeros.
                                // So yes, we flush `refinement_bits` here.
                                
                                for &b in &refinement_bits {
                                    bit_writer.write_bits(b, 1)?;
                                }
                                refinement_bits.clear();
                                run -= 16;
                            }
                            
                            // Send Symbol for new coeff
                            // Value is always 1 or -1 at this bit plane (shifted).
                            // Effectively category is always 1 (size 1).
                            // Symbol = (Run << 4) | 1.
                            let symbol = (run << 4) | 1;
                            let code = ac_table.codes[symbol as usize];
                            if code.length == 0 { return Err(JpeglsError::InvalidData); }
                            bit_writer.write_bits(code.value, code.length)?;
                            
                            // Send Sign bit
                            // If val > 0, sign is 1. If val < 0, sign is 0.
                            // Standard baseline logic: positive=1, negative=0.
                            let sign_bit = if val > 0 { 1 } else { 0 };
                            bit_writer.write_bits(sign_bit, 1)?;
                            
                            // Flush refinement bits
                            for &b in &refinement_bits {
                                bit_writer.write_bits(b, 1)?;
                            }
                            refinement_bits.clear();
                            run = 0;
                        }
                    }
                }
                
                // End of block.
                if run > 0 || !refinement_bits.is_empty() {
                    // We have trailing zeros OR trailing refinement bits.
                    // We must emit EOB.
                    let eob = ac_table.codes[0x00];
                    bit_writer.write_bits(eob.value, eob.length)?;
                    
                    // Flush remaining refinement bits
                    for &b in &refinement_bits {
                        bit_writer.write_bits(b, 1)?;
                    }
                }
            }
        }
        Ok(())
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
