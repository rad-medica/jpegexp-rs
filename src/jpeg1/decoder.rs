//! JPEG 1 Baseline Decoder implementation.

use crate::error::JpeglsError;
use crate::jpeg_stream_reader::JpegStreamReader;
use crate::jpeg1::dct::idct_8x8_baseline;
use crate::jpeg1::quantization::dequantize_block;
use crate::jpeg1::huffman::{HuffmanEncoder, JpegBitReader};

pub struct Jpeg1Decoder<'a> {
    reader: JpegStreamReader<'a>,
}

impl<'a> Jpeg1Decoder<'a> {
    pub fn new(source: &'a [u8]) -> Self {
        Self {
            reader: JpegStreamReader::new(source),
        }
    }

    pub fn read_header(&mut self) -> Result<(), JpeglsError> {
        let mut spiff = None;
        self.reader.read_header(&mut spiff)
    }

    pub fn decode(&mut self, destination: &mut [u8]) -> Result<(), JpeglsError> {
        let frame_info = self.reader.frame_info();
        self.reader.read_start_of_scan_segment_jpeg1()?;
        
        let mut bit_reader = JpegBitReader::new(self.reader.remaining_data());
        let mut dc_prev = [0i16; 4];

        let width = frame_info.width as usize;
        let height = frame_info.height as usize;
        let blocks_h = (height + 7) / 8;
        let blocks_w = (width + 7) / 8;

        for block_y in 0..blocks_h {
            for block_x in 0..blocks_w {
                let mut block_data = [0i16; 64];
                self.decode_block(&mut bit_reader, &mut dc_prev[0], &mut block_data)?;
                
                let mut dequant_coeffs = [0.0f32; 64];
                dequantize_block(&block_data, &self.reader.quantization_tables[0], &mut dequant_coeffs);
                
                let mut idct_result = [0.0f32; 64];
                idct_8x8_baseline(&dequant_coeffs, &mut idct_result);
                
                // Copy back to destination with clamping and level-shift back (+128)
                for y in 0..8 {
                    for x in 0..8 {
                        let py = block_y * 8 + y;
                        let px = block_x * 8 + x;
                        if py < height && px < width {
                            let val = (idct_result[y * 8 + x] + 128.0).round();
                            destination[py * width + px] = val.clamp(0.0, 255.0) as u8;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn decode_block(&self, bit_reader: &mut JpegBitReader, dc_prev: &mut i16, output: &mut [i16; 64]) -> Result<(), JpeglsError> {
        let dc_table = self.reader.huffman_tables_dc[0].as_ref().ok_or(JpeglsError::InvalidData)?;
        let ac_table = self.reader.huffman_tables_ac[0].as_ref().ok_or(JpeglsError::InvalidData)?;
        
        // 1. Decode DC
        let dc_category = dc_table.decode(bit_reader)?;
        let dc_diff_bits = bit_reader.read_bits(dc_category)?;
        let dc_diff = HuffmanEncoder::decode_value_bits(dc_diff_bits, dc_category);
        let dc_val = *dc_prev + dc_diff;
        *dc_prev = dc_val;
        output[0] = dc_val;

        // 2. Decode AC
        let mut k = 1;
        while k < 64 {
            let symbol = ac_table.decode(bit_reader)?;
            if symbol == 0 { // EOB
                break;
            }
            if symbol == 0xF0 { // ZRL
                k += 16;
                continue;
            }
            
            let run = (symbol >> 4) as usize;
            let category = symbol & 0x0F;
            k += run;
            if k >= 64 {
                return Err(JpeglsError::InvalidData);
            }
            
            let bits = bit_reader.read_bits(category)?;
            let val = HuffmanEncoder::decode_value_bits(bits, category);
            output[crate::jpeg1::encoder::ZIGZAG_ORDER[k]] = val;
            k += 1;
        }
        
        Ok(())
    }
}
