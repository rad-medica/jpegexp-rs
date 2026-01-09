//! JPEG 1 Baseline and Extended Sequential Encoder orchestration.

use crate::error::JpeglsError;
use crate::jpeg1::dct::fdct_8x8;
use crate::jpeg1::huffman::{
    HuffmanEncoder, HuffmanTable, JpegBitWriter, STD_LUMINANCE_DC_LENGTHS, STD_LUMINANCE_DC_VALUES,
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
        }
    }
}

impl Jpeg1Encoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_bits_per_sample(&mut self, bits: u8) {
        self.bits_per_sample = bits.clamp(8, 12);
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

    pub fn encode(
        &mut self,
        source: &[u8],
        frame_info: &FrameInfo,
        destination: &mut [u8],
    ) -> Result<usize, JpeglsError> {
        let mut writer = JpegStreamWriter::new(destination);
        let components_count = frame_info.component_count as usize;

        writer.write_start_of_image()?;

        if components_count == 1 {
            writer.write_dqt_u16(0, &self.quantization_table_lum)?;
        } else {
            writer.write_dqt_u16(0, &self.quantization_table_lum)?;
            writer.write_dqt_u16(1, &self.quantization_table_chrom)?;
        }

        // Write Huffman Tables (Luminance)
        if self.bits_per_sample > 8 {
            writer.write_dht(0, 0, &crate::jpeg1::huffman::EXT_LUMINANCE_DC_LENGTHS, &crate::jpeg1::huffman::EXT_LUMINANCE_DC_VALUES)?;
        } else {
            writer.write_dht(0, 0, &STD_LUMINANCE_DC_LENGTHS, &STD_LUMINANCE_DC_VALUES)?;
        }
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
        writer.write_sos_segment(frame_info.component_count as u8)?;

        let mut bit_writer_opt = Some(JpegBitWriter::new(writer.remaining_slice()));
        let width = frame_info.width as usize;
        let height = frame_info.height as usize;
        let mut mcus_encoded = 0;
        let mut next_restart_index = 0;
        let total_mcus = ((height + 7) / 8) * ((width + 7) / 8);

        self.huffman.dc_previous_value = [0; 4];
        let level_shift = (1 << (frame_info.bits_per_sample - 1)) as f32;

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

                if components_count == 1 {
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
                } else {
                    let mut block_y_data = [0.0f32; 64];
                    let mut block_cb_data = [0.0f32; 64];
                    let mut block_cr_data = [0.0f32; 64];
                    for y in 0..8 {
                        for x in 0..8 {
                            let py = block_y + y;
                            let px = block_x + x;
                            if py < height && px < width {
                                let idx = (py * width + px) * 3;
                                let r = source[idx] as f32;
                                let g = source[idx + 1] as f32;
                                let b = source[idx + 2] as f32;
                                let luma = 0.299 * r + 0.587 * g + 0.114 * b;
                                let cb = -0.1687 * r - 0.3313 * g + 0.5 * b + 128.0;
                                let cr = 0.5 * r - 0.4187 * g - 0.0813 * b + 128.0;
                                block_y_data[y * 8 + x] = luma - 128.0;
                                block_cb_data[y * 8 + x] = cb - 128.0;
                                block_cr_data[y * 8 + x] = cr - 128.0;
                            }
                        }
                    }
                    Self::encode_block_internal(&mut self.huffman, &block_y_data, bit_writer, &self.dc_table_lum, &self.ac_table_lum, &self.quantization_table_lum, 0)?;
                    Self::encode_block_internal(&mut self.huffman, &block_cb_data, bit_writer, &self.dc_table_chrom, &self.ac_table_chrom, &self.quantization_table_chrom, 1)?;
                    Self::encode_block_internal(&mut self.huffman, &block_cr_data, bit_writer, &self.dc_table_chrom, &self.ac_table_chrom, &self.quantization_table_chrom, 2)?;
                }
                mcus_encoded += 1;
            }
        }

        let mut bw = bit_writer_opt.unwrap();
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
        let mut writer = JpegStreamWriter::new(destination);
        let components_count = frame_info.component_count as usize;

        writer.write_start_of_image()?;

        if components_count == 1 {
            writer.write_dqt_u16(0, &self.quantization_table_lum)?;
        } else {
            writer.write_dqt_u16(0, &self.quantization_table_lum)?;
            writer.write_dqt_u16(1, &self.quantization_table_chrom)?;
        }

        // Write Huffman Tables (Luminance)
        if self.bits_per_sample > 8 {
            writer.write_dht(0, 0, &crate::jpeg1::huffman::EXT_LUMINANCE_DC_LENGTHS, &crate::jpeg1::huffman::EXT_LUMINANCE_DC_VALUES)?;
        } else {
            writer.write_dht(0, 0, &STD_LUMINANCE_DC_LENGTHS, &STD_LUMINANCE_DC_VALUES)?;
        }
        writer.write_dht(1, 0, &self.ac_table_lum.lengths, &self.ac_table_lum.values)?;

        if components_count > 1 {
            writer.write_dht(0, 1, &crate::jpeg1::huffman::EXT_LUMINANCE_DC_LENGTHS, &crate::jpeg1::huffman::EXT_LUMINANCE_DC_VALUES)?;
            writer.write_dht(1, 1, &self.ac_table_chrom.lengths, &self.ac_table_chrom.values)?;
        }

        if self.restart_interval > 0 {
            writer.write_dri(self.restart_interval)?;
        }

        if frame_info.bits_per_sample > 8 {
            writer.write_sof1_segment(frame_info)?;
        } else {
            writer.write_sof0_segment(frame_info)?;
        }
        writer.write_sos_segment(frame_info.component_count as u8)?;

        let mut bit_writer_opt = Some(JpegBitWriter::new(writer.remaining_slice()));
        let width = frame_info.width as usize;
        let height = frame_info.height as usize;
        let mut mcus_encoded = 0;
        let mut next_restart_index = 0;
        let total_mcus = ((height + 7) / 8) * ((width + 7) / 8);

        self.huffman.dc_previous_value = [0; 4];
        let level_shift = (1 << (frame_info.bits_per_sample - 1)) as f32;

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

                if components_count == 1 {
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
                } else {
                    let mut block_y_data = [0.0f32; 64];
                    let mut block_cb_data = [0.0f32; 64];
                    let mut block_cr_data = [0.0f32; 64];
                    for y in 0..8 {
                        for x in 0..8 {
                            let py = block_y + y;
                            let px = block_x + x;
                            if py < height && px < width {
                                let idx = (py * width + px) * components_count;
                                let r = source[idx] as f32;
                                let g = source[idx + 1] as f32;
                                let b = source[idx + 2] as f32;
                                let luma = 0.299 * r + 0.587 * g + 0.114 * b;
                                let cb = -0.1687 * r - 0.3313 * g + 0.5 * b + level_shift;
                                let cr = 0.5 * r - 0.4187 * g - 0.0813 * b + level_shift;
                                block_y_data[y * 8 + x] = luma - level_shift;
                                block_cb_data[y * 8 + x] = cb - level_shift;
                                block_cr_data[y * 8 + x] = cr - level_shift;
                            }
                        }
                    }
                    Self::encode_block_internal(&mut self.huffman, &block_y_data, bit_writer, &self.dc_table_lum, &self.ac_table_lum, &self.quantization_table_lum, 0)?;
                    Self::encode_block_internal(&mut self.huffman, &block_cb_data, bit_writer, &self.dc_table_chrom, &self.ac_table_chrom, &self.quantization_table_chrom, 1)?;
                    Self::encode_block_internal(&mut self.huffman, &block_cr_data, bit_writer, &self.dc_table_chrom, &self.ac_table_chrom, &self.quantization_table_chrom, 2)?;
                }
                mcus_encoded += 1;
            }
        }

        let mut bw = bit_writer_opt.unwrap();
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

        writer.write_start_of_image()?;

        if components_count == 1 {
            writer.write_dqt_u16(0, &self.quantization_table_lum)?;
        } else {
            writer.write_dqt_u16(0, &self.quantization_table_lum)?;
            writer.write_dqt_u16(1, &self.quantization_table_chrom)?;
        }

        // Write Huffman Tables (Luminance)
        if self.bits_per_sample > 8 {
            writer.write_dht(0, 0, &crate::jpeg1::huffman::EXT_LUMINANCE_DC_LENGTHS, &crate::jpeg1::huffman::EXT_LUMINANCE_DC_VALUES)?;
        } else {
            writer.write_dht(0, 0, &STD_LUMINANCE_DC_LENGTHS, &STD_LUMINANCE_DC_VALUES)?;
        }
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
