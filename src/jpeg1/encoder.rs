//! JPEG 1 Baseline Encoder orchestration.

use crate::error::JpeglsError;
use crate::FrameInfo;
use crate::jpeg_stream_writer::JpegStreamWriter;
use crate::jpeg1::dct::{fdct_8x8};
use crate::jpeg1::quantization::{quantize_block, STD_LUMINANCE_QUANT_TABLE};
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
}

impl Jpeg1Encoder {
    pub fn new() -> Self {
        Self {
            huffman: HuffmanEncoder::new(),
            dc_table_lum: HuffmanTable::standard_luminance_dc(),
            ac_table_lum: HuffmanTable::standard_luminance_ac(),
        }
    }

    pub fn encode_grayscale(&mut self, source: &[u8], frame_info: &FrameInfo, destination: &mut [u8]) -> Result<usize, JpeglsError> {
        let mut writer = JpegStreamWriter::new(destination);
        
        // 1. Write Headers
        writer.write_start_of_image()?;
        
        // DQT
        writer.write_dqt(0, &STD_LUMINANCE_QUANT_TABLE)?;
        
        // DHT (DC)
        writer.write_dht(0, 0, &STD_LUMINANCE_DC_LENGTHS, &STD_LUMINANCE_DC_VALUES)?;
        
        // DHT (AC)
        writer.write_dht(1, 0, &self.ac_table_lum.lengths, &self.ac_table_lum.values)?;
        
        // SOF0
        writer.write_sof0_segment(frame_info)?;
        
        // SOS
        writer.write_sos_segment(1)?;

        // 2. Entropy Coded Data
        let mut bit_writer = JpegBitWriter::new(writer.remaining_slice());
        let width = frame_info.width as usize;
        let height = frame_info.height as usize;

        // Reset DC prediction
        self.huffman.dc_previous_value = [0; 4];

        for block_y in (0..height).step_by(8) {
            for block_x in (0..width).step_by(8) {
                let mut block_data = [0.0f32; 64];
                
                for y in 0..8 {
                    for x in 0..8 {
                        let py = block_y + y;
                        let px = block_x + x;
                        if py < height && px < width {
                            block_data[y * 8 + x] = source[py * width + px] as f32 - 128.0;
                        } else {
                            block_data[y * 8 + x] = 0.0;
                        }
                    }
                }

                self.encode_block_internal(&block_data, &mut bit_writer)?;
            }
        }

        bit_writer.flush()?;
        let encoded_len = bit_writer.len();
        writer.advance(encoded_len);

        // 3. Write EOI
        writer.write_end_of_image()?;

        Ok(writer.len())
    }

    fn encode_block_internal(&mut self, block: &[f32; 64], bit_writer: &mut JpegBitWriter) -> Result<(), JpeglsError> {
        let mut dct_coeffs = [0.0f32; 64];
        fdct_8x8(block, &mut dct_coeffs);

        let mut quant_coeffs = [0i16; 64];
        quantize_block(&dct_coeffs, &STD_LUMINANCE_QUANT_TABLE, &mut quant_coeffs);

        let mut zigzag_coeffs = [0i16; 64];
        for i in 0..64 {
            zigzag_coeffs[i] = quant_coeffs[ZIGZAG_ORDER[i]];
        }

        // DC
        let dc_val = zigzag_coeffs[0];
        let diff = dc_val - self.huffman.dc_previous_value[0];
        self.huffman.dc_previous_value[0] = dc_val;

        let dc_category = HuffmanEncoder::get_category(diff);
        let dc_code = self.dc_table_lum.codes[dc_category as usize];
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
                    let zrl_code = self.ac_table_lum.codes[0xF0];
                    bit_writer.write_bits(zrl_code.value, zrl_code.length)?;
                    run -= 16;
                }

                let category = HuffmanEncoder::get_category(ac_val);
                let symbol = (run << 4) | (category as usize);
                let ac_code = self.ac_table_lum.codes[symbol];
                
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
            let eob_code = self.ac_table_lum.codes[0x00];
            bit_writer.write_bits(eob_code.value, eob_code.length)?;
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let width = 16;
        let height = 16;
        let mut source = vec![128u8; width * height];

        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 1,
        };

        let mut encoded = vec![0u8; 10000];
        let mut encoder = Jpeg1Encoder::new();
        let enc_len = encoder.encode_grayscale(&source, &frame_info, &mut encoded).expect("Encode failed");
        
        let mut decoder = crate::jpeg1::decoder::Jpeg1Decoder::new(&encoded[..enc_len]);
        decoder.read_header().expect("Read header failed");
        
        let mut decoded = vec![0u8; width * height];
        decoder.decode(&mut decoded).expect("Decode failed");
        
        for i in 0..source.len() {
            let diff = (source[i] as i32 - decoded[i] as i32).abs();
            assert!(diff < 35, "Mismatch at {}: {} vs {} (diff {})", i, source[i], decoded[i], diff);
        }
    }
}
