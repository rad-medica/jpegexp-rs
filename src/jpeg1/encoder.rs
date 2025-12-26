//! JPEG 1 Baseline Encoder orchestration.

use crate::error::JpeglsError;
use crate::FrameInfo;
use crate::jpeg_stream_writer::JpegStreamWriter;
use crate::jpeg1::dct::fdct_8x8;
use crate::jpeg1::quantization::{quantize_block, STD_LUMINANCE_QUANT_TABLE, STD_CHROMINANCE_QUANT_TABLE};
use crate::jpeg1::huffman::{
    HuffmanEncoder, JpegBitWriter, HuffmanTable, 
    STD_LUMINANCE_DC_LENGTHS, STD_LUMINANCE_DC_VALUES
};

/// Zigzag scan pattern for 8x8 blocks.
pub const ZIGZAG_ORDER: [usize; 64] = [
    0,  1,  8, 16,  9,  2,  3, 10,
    17, 24, 32, 25, 18, 11,  4,  5,
    12, 19, 26, 33, 40, 48, 41, 34,
    27, 20, 13,  6,  7, 14, 21, 28,
    35, 42, 49, 56, 57, 50, 43, 36,
    29, 22, 15, 23, 30, 37, 44, 51,
    58, 59, 52, 45, 38, 31, 39, 46,
    53, 60, 61, 54, 47, 55, 62, 63,
];

pub struct Jpeg1Encoder {
    huffman: HuffmanEncoder,
    dc_table_lum: HuffmanTable,
    ac_table_lum: HuffmanTable,
    dc_table_chrom: HuffmanTable,
    ac_table_chrom: HuffmanTable,
    pub quantization_table_lum: [u8; 64],
    pub quantization_table_chrom: [u8; 64],
    pub restart_interval: u16,
}

impl Jpeg1Encoder {
    pub fn new() -> Self {
        Self {
            huffman: HuffmanEncoder::new(),
            dc_table_lum: HuffmanTable::standard_luminance_dc(),
            ac_table_lum: HuffmanTable::standard_luminance_ac(),
            dc_table_chrom: HuffmanTable::standard_chrominance_dc(),
            ac_table_chrom: HuffmanTable::standard_chrominance_ac(),
            quantization_table_lum: STD_LUMINANCE_QUANT_TABLE,
            quantization_table_chrom: STD_CHROMINANCE_QUANT_TABLE,
            restart_interval: 0,
        }
    }

    pub fn set_restart_interval(&mut self, interval: u16) {
        self.restart_interval = interval;
    }

    pub fn encode(&mut self, source: &[u8], frame_info: &FrameInfo, destination: &mut [u8]) -> Result<usize, JpeglsError> {
        let mut writer = JpegStreamWriter::new(destination);
        
        let components_count = frame_info.component_count as usize;
        
        writer.write_start_of_image()?;
        
        // Write Quantization Tables
        if components_count == 1 {
            writer.write_dqt(0, &self.quantization_table_lum)?;
        } else {
            writer.write_dqt(0, &self.quantization_table_lum)?;
            writer.write_dqt(1, &self.quantization_table_chrom)?;
        }

        // Write Huffman Tables (Luminance)
        writer.write_dht(0, 0, &STD_LUMINANCE_DC_LENGTHS, &STD_LUMINANCE_DC_VALUES)?;
        writer.write_dht(1, 0, &self.ac_table_lum.lengths, &self.ac_table_lum.values)?;

        // Write Huffman Tables (Chrominance) if needed
        if components_count > 1 {
            writer.write_dht(0, 1, &crate::jpeg1::huffman::STD_CHROMINANCE_DC_LENGTHS, &crate::jpeg1::huffman::STD_CHROMINANCE_DC_VALUES)?;
            writer.write_dht(1, 1, &self.ac_table_chrom.lengths, &self.ac_table_chrom.values)?;
        }

        if self.restart_interval > 0 {
            writer.write_dri(self.restart_interval)?;
        }

        writer.write_sof0_segment(frame_info)?;
        writer.write_sos_segment(frame_info.component_count as u8)?;

        // Use Option to manage borrow of writer via bit_writer
        let mut bit_writer_opt = Some(JpegBitWriter::new(writer.remaining_slice()));
        let width = frame_info.width as usize;
        let height = frame_info.height as usize;
        
        let mut mcus_encoded = 0;
        let mut next_restart_index = 0;
        let total_mcus = ((height + 7) / 8) * ((width + 7) / 8);

        // Reset DC predictors
        self.huffman.dc_previous_value = [0; 4];

        for block_y in (0..height).step_by(8) {
            for block_x in (0..width).step_by(8) {
                
                // Restart Marker Logic
                if self.restart_interval > 0 && mcus_encoded > 0 && (mcus_encoded % self.restart_interval as usize == 0) && mcus_encoded < total_mcus {
                     let bw = bit_writer_opt.as_mut().unwrap();
                     bw.flush()?;
                     let len = bw.len();
                     bit_writer_opt = None; // Drop borrow
                     
                     writer.advance(len);
                     let marker = crate::jpeg_marker_code::JpegMarkerCode::try_from(0xD0 + (next_restart_index % 8)).unwrap();
                     writer.write_marker(marker)?;
                     next_restart_index += 1;
                     
                     // Create new bit writer
                     bit_writer_opt = Some(JpegBitWriter::new(writer.remaining_slice()));
                     
                     // Reset DC predictors
                     self.huffman.dc_previous_value = [0; 4];
                }

                let bit_writer = bit_writer_opt.as_mut().unwrap();
                
                if components_count == 1 {
                    // Grayscale
                    let mut block_data = [0.0f32; 64];
                    for y in 0..8 {
                        for x in 0..8 {
                            let py = block_y + y;
                            let px = block_x + x;
                            if py < height && px < width {
                                block_data[y * 8 + x] = source[py * width + px] as f32 - 128.0;
                            }
                        }
                    }
                    // Y: DC table 0, AC table 0, Quant table 0, Pred index 0
                    Self::encode_block_internal(&mut self.huffman, &block_data, bit_writer, &self.dc_table_lum, &self.ac_table_lum, &self.quantization_table_lum, 0)?;
                } else {
                     // YCbCr Interleaved (4:4:4)
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

                                // RGB to YCbCr
                                let luma = 0.299 * r + 0.587 * g + 0.114 * b;
                                let cb = -0.1687 * r - 0.3313 * g + 0.5 * b + 128.0;
                                let cr = 0.5 * r - 0.4187 * g - 0.0813 * b + 128.0;

                                block_y_data[y * 8 + x] = luma - 128.0;
                                block_cb_data[y * 8 + x] = cb - 128.0;
                                block_cr_data[y * 8 + x] = cr - 128.0;
                            }
                        }
                    }
                    
                    // Y: DC 0, AC 0, Quant 0, Pred 0
                    Self::encode_block_internal(&mut self.huffman, &block_y_data, bit_writer, &self.dc_table_lum, &self.ac_table_lum, &self.quantization_table_lum, 0)?;
                    
                    // Cb: DC 1, AC 1, Quant 1, Pred 1
                    Self::encode_block_internal(&mut self.huffman, &block_cb_data, bit_writer, &self.dc_table_chrom, &self.ac_table_chrom, &self.quantization_table_chrom, 1)?;
                    // Cr: DC 1, AC 1, Quant 1, Pred 2 
                    Self::encode_block_internal(&mut self.huffman, &block_cr_data, bit_writer, &self.dc_table_chrom, &self.ac_table_chrom, &self.quantization_table_chrom, 2)?;
                }
                mcus_encoded += 1;
            }
        }

        
        // Final flush
        let mut bw = bit_writer_opt.unwrap();
        bw.flush()?;
        let encoded_len = bw.len();
        writer.advance(encoded_len);
        writer.write_end_of_image()?;

        Ok(writer.len())
    }

    pub fn encode_planar(&mut self, source: &[u8], frame_info: &FrameInfo, destination: &mut [u8]) -> Result<usize, JpeglsError> {
        let mut writer = JpegStreamWriter::new(destination);
        
        let components_count = frame_info.component_count as usize;
        
        writer.write_start_of_image()?;
        
        // Write Quantization Tables (same as interleaved)
        if components_count == 1 {
            writer.write_dqt(0, &self.quantization_table_lum)?;
        } else {
            writer.write_dqt(0, &self.quantization_table_lum)?;
            writer.write_dqt(1, &self.quantization_table_chrom)?;
        }

        // Write Huffman Tables (same as interleaved)
        writer.write_dht(0, 0, &STD_LUMINANCE_DC_LENGTHS, &STD_LUMINANCE_DC_VALUES)?;
        writer.write_dht(1, 0, &self.ac_table_lum.lengths, &self.ac_table_lum.values)?;
        if components_count > 1 {
            writer.write_dht(0, 1, &crate::jpeg1::huffman::STD_CHROMINANCE_DC_LENGTHS, &crate::jpeg1::huffman::STD_CHROMINANCE_DC_VALUES)?;
            writer.write_dht(1, 1, &self.ac_table_chrom.lengths, &self.ac_table_chrom.values)?;
        }
        
        if self.restart_interval > 0 {
            writer.write_dri(self.restart_interval)?;
        }

        writer.write_sof0_segment(frame_info)?;

        let width = frame_info.width as usize;
        let height = frame_info.height as usize;

        // Loop over each component creating a separate scan
        for comp_idx in 0..components_count {
            // Write SOS for SINGLE component
             // write_sos_segment writes ALL components by default. 
             // We need write_sos_segment_component(component_id, table_id)
             // Manually write SOS here for control
             
             writer.write_marker(crate::jpeg_marker_code::JpegMarkerCode::StartOfScan)?;
             let length = 2 + 1 + (1 * 2) + 3; // 1 component
             writer.write_u16(length as u16)?;
             writer.write_byte(1)?; // 1 component in this scan
             
             // Component ID (1-based)
             writer.write_byte((comp_idx + 1) as u8)?;
             
             // Tables
             let (dc_table, ac_table, quant_table, pred_idx) = if comp_idx == 0 {
                 // Luminance
                 (0x00, 0x00, &self.quantization_table_lum, 0)
             } else {
                 // Chrominance
                 (0x11, 0x11, &self.quantization_table_chrom, comp_idx) // Use pred_idx 1 and 2
             };
             
             // DC/AC table selector
             let table_sel = if comp_idx == 0 { 0x00 } else { 0x11 };
             writer.write_byte(table_sel)?;
             
             writer.write_byte(0)?; // Ss
             writer.write_byte(63)?; // Se
             writer.write_byte(0)?; // Ah/Al

             // Encode Scan Data
            let mut bit_writer_opt = Some(JpegBitWriter::new(writer.remaining_slice()));
            let mut bit_writer = bit_writer_opt.as_mut().unwrap();
            
             // Reset DC predictor for this component at start of scan
             self.huffman.dc_previous_value[pred_idx] = 0;
             let mut mcus_encoded = 0;
             let mut next_restart_index = 0;
             // Total blocks for this component
             let total_blocks = ((height + 7) / 8) * ((width + 7) / 8);

            for block_y in (0..height).step_by(8) {
                for block_x in (0..width).step_by(8) {
                    
                    // Restart Logic (Per Scan)
                    if self.restart_interval > 0 && mcus_encoded > 0 && (mcus_encoded % self.restart_interval as usize == 0) && mcus_encoded < total_blocks {
                         bit_writer.flush()?;
                         let len = bit_writer.len();
                         drop(bit_writer); // Release borrow
                         bit_writer_opt = None;

                         writer.advance(len);
                         let marker = crate::jpeg_marker_code::JpegMarkerCode::try_from(0xD0 + (next_restart_index % 8)).unwrap();
                         writer.write_marker(marker)?;
                         next_restart_index += 1;
                         
                         bit_writer_opt = Some(JpegBitWriter::new(writer.remaining_slice()));
                         bit_writer = bit_writer_opt.as_mut().unwrap();
                         
                         self.huffman.dc_previous_value[pred_idx] = 0;
                    }

                    // Extract Block for Component
                    let mut block_data = [0.0f32; 64];
                    for y in 0..8 {
                        for x in 0..8 {
                            let py = block_y + y;
                            let px = block_x + x;
                            if py < height && px < width {
                                if components_count == 1 {
                                    block_data[y*8+x] = source[py*width+px] as f32 - 128.0;
                                } else {
                                    let idx = (py*width+px)*3;
                                    let r = source[idx] as f32;
                                    let g = source[idx+1] as f32;
                                    let b = source[idx+2] as f32;
                                    
                                    if comp_idx == 0 {
                                        block_data[y*8+x] = (0.299*r + 0.587*g + 0.114*b) - 128.0;
                                    } else if comp_idx == 1 {
                                        block_data[y*8+x] = (-0.1687*r - 0.3313*g + 0.5*b + 128.0) - 128.0;
                                    } else {
                                        block_data[y*8+x] = (0.5*r - 0.4187*g - 0.0813*b + 128.0) - 128.0;
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
            // Drop bit_writer to advance
            drop(bit_writer); 
            bit_writer_opt = None;
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
        quant_table: &[u8; 64],
        dc_pred_idx: usize
    ) -> Result<(), JpeglsError> {
        let mut dct_coeffs = [0.0f32; 64];
        fdct_8x8(block, &mut dct_coeffs);

        let mut quant_coeffs = [0i16; 64];
        quantize_block(&dct_coeffs, quant_table, &mut quant_coeffs);

        let mut zigzag_coeffs = [0i16; 64];
        for i in 0..64 {
            zigzag_coeffs[i] = quant_coeffs[ZIGZAG_ORDER[i]];
        }

        // DC
        let dc_val = zigzag_coeffs[0];
        let diff = dc_val - huffman.dc_previous_value[dc_pred_idx];
        huffman.dc_previous_value[dc_pred_idx] = dc_val;

        let dc_category = HuffmanEncoder::get_category(diff);
        let dc_code = dc_table.codes[dc_category as usize];
        bit_writer.write_bits(dc_code.value, dc_code.length)?;
        let (dc_bits, dc_bit_len) = HuffmanEncoder::get_diff_bits(diff, dc_category);
        bit_writer.write_bits(dc_bits, dc_bit_len)?;

        // AC
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
                bit_writer.write_bits(ac_code.value, ac_code.length)?;
                let (ac_bits, ac_bit_len) = HuffmanEncoder::get_diff_bits(ac_val, category);
                bit_writer.write_bits(ac_bits, ac_bit_len)?;
                run = 0;
            }
        }
        if run > 0 {
            let eob_code = ac_table.codes[0x00];
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
        // Use standard quantization table for test robustness
        
        let mut encoded = vec![0u8; 10000];
        let enc_len = encoder.encode(&source, &frame_info, &mut encoded).expect("Encode failed");
        
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
            // Generate some colors
            source[i * 3 + 0] = (i % 256) as u8; // R
            source[i * 3 + 1] = ((i * 2) % 256) as u8; // G
            source[i * 3 + 2] = ((255 - i) % 256) as u8; // B
        }

        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 3,
        };

        let mut encoder = Jpeg1Encoder::new();
        
        let mut encoded = vec![0u8; 10000];
        let enc_len = encoder.encode(&source, &frame_info, &mut encoded).expect("Encode failed");
        
        let mut decoder = crate::jpeg1::decoder::Jpeg1Decoder::new(&encoded[..enc_len]);
        decoder.read_header().expect("Read header failed");
        
        // Wait, did I update JpegStreamReader to read components correctly? Yes.
        // Did I update Decoder to output RGB? Yes.
        
        let mut decoded = vec![0u8; width * height * 3];
        decoder.decode(&mut decoded).expect("Decode failed");
        
        let tolerance = 25; 
        for i in 0..source.len() {
            let diff = (source[i] as i32 - decoded[i] as i32).abs();
            assert!(diff < tolerance, "Mismatch at index {}: src={} dec={} diff={}", i, source[i], decoded[i], diff);
        }
    }

    #[test]
    fn test_encode_decode_roundtrip_restart() {
        let width = 32; // 4 blocks wide
        let height = 16; // 2 blocks high. Total 8 blocks.
        // We set restart interval to 4. So we expect RST0 in the middle.
        
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
        encoder.set_restart_interval(4);
        
        let mut encoded = vec![0u8; 10000];
        let enc_len = encoder.encode(&source, &frame_info, &mut encoded).expect("Encode failed");
        
        // Verify RST marker is present
        let mut found_rst0 = false;
        for i in 0..enc_len-1 {
            if encoded[i] == 0xFF && encoded[i+1] == 0xD0 {
                found_rst0 = true;
                break;
            }
        }
        assert!(found_rst0, "Encoded stream should contain RST0 marker");

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
    fn test_encode_decode_roundtrip_planar() {
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
        let enc_len = encoder.encode_planar(&source, &frame_info, &mut encoded).expect("Encode failed");
        
        // Verify multiple SOS markers (should be 3)
        let mut sos_count = 0;
        for i in 0..enc_len-1 {
            if encoded[i] == 0xFF && encoded[i+1] == 0xDA {
                sos_count += 1;
            }
        }
        assert_eq!(sos_count, 3, "Should have 3 SOS markers for planar encoding, found {}", sos_count);

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
