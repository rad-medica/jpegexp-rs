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
        let width = frame_info.width as usize;
        let height = frame_info.height as usize;
        let blocks_w = (width + 7) / 8;
        let blocks_h = (height + 7) / 8;
        let components_count = self.reader.components.len();

        // Intermediate buffers for component data (IDCT output)
        // Storing as f32 to avoid clamping until final stage
        let mut component_buffers = vec![vec![0.0f32; blocks_w * blocks_h * 64]; components_count];

        loop {
            // Check for next marker
            let marker = self.reader.peek_marker();
            match marker {
                Ok(crate::jpeg_marker_code::JpegMarkerCode::StartOfScan) => {
                    self.reader.read_start_of_scan_segment_jpeg1()?;
                }
                Ok(crate::jpeg_marker_code::JpegMarkerCode::EndOfImage) => {
                    break;
                }
                Ok(crate::jpeg_marker_code::JpegMarkerCode::DefineHuffmanTable) => {
                    self.reader.read_dht_segment()?;
                    continue;
                }
                Ok(crate::jpeg_marker_code::JpegMarkerCode::DefineQuantizationTable) => {
                    self.reader.read_dqt_segment()?;
                    continue;
                }
                Ok(crate::jpeg_marker_code::JpegMarkerCode::DefineRestartInterval) => {
                    self.reader.read_dri_segment()?;
                    continue;
                }
                Ok(
                    crate::jpeg_marker_code::JpegMarkerCode::ApplicationData0 |
                    crate::jpeg_marker_code::JpegMarkerCode::ApplicationData1 |
                    crate::jpeg_marker_code::JpegMarkerCode::ApplicationData2 |
                    crate::jpeg_marker_code::JpegMarkerCode::ApplicationData3 |
                    crate::jpeg_marker_code::JpegMarkerCode::ApplicationData4 |
                    crate::jpeg_marker_code::JpegMarkerCode::ApplicationData5 |
                    crate::jpeg_marker_code::JpegMarkerCode::ApplicationData6 |
                    crate::jpeg_marker_code::JpegMarkerCode::ApplicationData7 |
                    crate::jpeg_marker_code::JpegMarkerCode::ApplicationData8 |
                    crate::jpeg_marker_code::JpegMarkerCode::ApplicationData9 |
                    crate::jpeg_marker_code::JpegMarkerCode::ApplicationData10 |
                    crate::jpeg_marker_code::JpegMarkerCode::ApplicationData11 |
                    crate::jpeg_marker_code::JpegMarkerCode::ApplicationData12 |
                    crate::jpeg_marker_code::JpegMarkerCode::ApplicationData13 |
                    crate::jpeg_marker_code::JpegMarkerCode::ApplicationData14 |
                    crate::jpeg_marker_code::JpegMarkerCode::ApplicationData15 |
                    crate::jpeg_marker_code::JpegMarkerCode::Comment
                ) => {
                    self.reader.skip_segment()?;
                    continue;
                }
                _ => {
                    // Try to catch other markers or if we are just done?
                    // Some streams might have trailing bytes.
                    break;
                }
            }

            let scan_components = self.reader.scan_component_indices.clone();
            let scan_components_count = scan_components.len();
            
            // Re-initialize predictors for this scan
            let mut dc_preds = vec![0i16; components_count]; // Using separate tracking

            let mut bit_reader = JpegBitReader::new(self.reader.remaining_data());
            
            let restart_interval = self.reader.restart_interval as usize;
            let mut mcus_decoded = 0;
            let mut next_restart_index = 0;
            
            // Logic differs for interleaved vs non-interleaved
            if scan_components_count > 1 {
                // Interleaved (MCU based)
                let total_mcus = blocks_h * blocks_w;

                for block_y in 0..blocks_h {
                    for block_x in 0..blocks_w {
                        // Restart Check
                        if restart_interval > 0 && mcus_decoded > 0 && (mcus_decoded % restart_interval == 0) && mcus_decoded < total_mcus {
                             bit_reader.align_to_byte();
                             let marker = bit_reader.read_bits(16)?;
                             let expected_marker = 0xFFD0 + (next_restart_index % 8);
                             if marker != expected_marker {
                                 // Strict check could be return Err
                             }
                             next_restart_index += 1;
                             for i in 0..components_count { dc_preds[i] = 0; }
                        }

                        for &comp_idx in &scan_components {
                            let (dc_idx, ac_idx, quant_idx) = {
                                let c = &self.reader.components[comp_idx];
                                (c.dc_table_dest as usize, c.ac_table_dest as usize, c.quant_table_dest as usize)
                            };

                            let mut block_data = [0i16; 64];
                            let mut dequant_coeffs = [0.0f32; 64];

                            let dc_table = self.reader.huffman_tables_dc[dc_idx].as_ref().ok_or(JpeglsError::InvalidData)?;
                            let ac_table = self.reader.huffman_tables_ac[ac_idx].as_ref().ok_or(JpeglsError::InvalidData)?;
                            let quant_table = &self.reader.quantization_tables[quant_idx];

                            Self::decode_block(&mut bit_reader, &mut dc_preds[comp_idx], &mut block_data, dc_table, ac_table)?;
                            dequantize_block(&block_data, quant_table, &mut dequant_coeffs);
                            
                            // Write directly to component buffer
                            // Calculate offset
                            let block_offset = (block_y * blocks_w + block_x) * 64;
                            let buffer = &mut component_buffers[comp_idx];
                            
                            // Perform IDCT and write
                             let mut idct_out = [0.0f32; 64];
                             // Use fixed point implementation for performance
                             crate::jpeg1::dct::idct_8x8_fixed_point(&dequant_coeffs, &mut idct_out);
                             for k in 0..64 {
                                 buffer[block_offset + k] = idct_out[k];
                             }
                        }
                        mcus_decoded += 1;
                    }
                }
            } else {
                // Non-Interleaved (One component)
                let comp_idx = scan_components[0];
                 // Calculate blocks for this component (assuming 4:4:4 for now, so matches image blocks)
                 let total_blocks = blocks_h * blocks_w;
                 
                 for block_y in 0..blocks_h {
                    for block_x in 0..blocks_w {
                        // Restart Check
                        if restart_interval > 0 && mcus_decoded > 0 && (mcus_decoded % restart_interval == 0) && mcus_decoded < total_blocks {
                             bit_reader.align_to_byte();
                             let marker = bit_reader.read_bits(16)?;
                             let expected_marker = 0xFFD0 + (next_restart_index % 8);
                             if marker != expected_marker {
                                 // Strict check
                             }
                             next_restart_index += 1;
                             dc_preds[comp_idx] = 0; 
                        }

                        let (dc_idx, ac_idx, quant_idx) = {
                            let c = &self.reader.components[comp_idx];
                            (c.dc_table_dest as usize, c.ac_table_dest as usize, c.quant_table_dest as usize)
                        };

                        let mut block_data = [0i16; 64];
                        let mut dequant_coeffs = [0.0f32; 64];

                        let dc_table = self.reader.huffman_tables_dc[dc_idx].as_ref().ok_or(JpeglsError::InvalidData)?;
                        let ac_table = self.reader.huffman_tables_ac[ac_idx].as_ref().ok_or(JpeglsError::InvalidData)?;
                        let quant_table = &self.reader.quantization_tables[quant_idx];

                        Self::decode_block(&mut bit_reader, &mut dc_preds[comp_idx], &mut block_data, dc_table, ac_table)?;
                        dequantize_block(&block_data, quant_table, &mut dequant_coeffs);
                        
                        let block_offset = (block_y * blocks_w + block_x) * 64;
                        let buffer = &mut component_buffers[comp_idx];
                        
                        let mut idct_out = [0.0f32; 64];
                        crate::jpeg1::dct::idct_8x8_fixed_point(&dequant_coeffs, &mut idct_out);
                        for k in 0..64 {
                            buffer[block_offset + k] = idct_out[k];
                        }
                        
                        mcus_decoded += 1;
                    }
                }
            }

            self.reader.advance(bit_reader.position());
        }

        // Final Color Conversion
        for py in 0..height {
            for px in 0..width {
                // Determine block indices
                let bx = px / 8;
                let by = py / 8;
                let tx = px % 8;
                let ty = py % 8;
                let block_idx = (by * blocks_w + bx) * 64 + (ty * 8 + tx);

                if components_count == 1 {
                    let val = (component_buffers[0][block_idx] + 128.0).round().clamp(0.0, 255.0) as u8;
                    destination[py * width + px] = val;
                } else if components_count == 3 {
                    let y_val = component_buffers[0][block_idx];
                    let cb_val = component_buffers[1][block_idx];
                    let cr_val = component_buffers[2][block_idx];

                    let r = y_val + 1.402 * cr_val + 128.0;
                    let g = y_val - 0.344136 * cb_val - 0.714136 * cr_val + 128.0;
                    let b = y_val + 1.772 * cb_val + 128.0;

                    let pixel_idx = (py * width + px) * 3;
                     if pixel_idx + 2 < destination.len() {
                        destination[pixel_idx] = r.clamp(0.0, 255.0) as u8;
                        destination[pixel_idx + 1] = g.clamp(0.0, 255.0) as u8;
                        destination[pixel_idx + 2] = b.clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }

        Ok(())
    }

    fn decode_block(bit_reader: &mut JpegBitReader, dc_prev: &mut i16, output: &mut [i16; 64], dc_table: &crate::jpeg1::huffman::HuffmanTable, ac_table: &crate::jpeg1::huffman::HuffmanTable) -> Result<(), JpeglsError> {
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
