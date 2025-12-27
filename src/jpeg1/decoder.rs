//! JPEG 1 Baseline and Progressive Decoder implementation.

use crate::error::JpeglsError;
use crate::jpeg_stream_reader::JpegStreamReader;
use crate::jpeg1::huffman::{HuffmanEncoder, JpegBitReader};
use crate::jpeg1::quantization::dequantize_block;

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
        if self.reader.is_lossless {
            return self.decode_lossless(destination);
        }

        let frame_info = self.reader.frame_info();
        let width = frame_info.width as usize;
        let height = frame_info.height as usize;
        let blocks_w = (width + 7) / 8;
        let blocks_h = (height + 7) / 8;
        let components_count = self.reader.components.len();

        let mut coefficient_buffers = vec![vec![0i16; blocks_w * blocks_h * 64]; components_count];
        let mut dc_preds = vec![0i16; components_count];
        let mut eob_runs = vec![0u16; components_count];

        loop {
            let marker = self.reader.peek_marker();
            match marker {
                Ok(crate::jpeg_marker_code::JpegMarkerCode::StartOfScan) => {
                    self.reader.read_start_of_scan_segment_jpeg1()?;
                }
                Ok(crate::jpeg_marker_code::JpegMarkerCode::EndOfImage) => break,
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
                _ if marker.is_ok()
                    && marker.unwrap() as u8 >= 0xE0
                    && (marker.unwrap() as u8) <= 0xFE =>
                {
                    self.reader.skip_segment()?;
                    continue;
                }
                _ => break,
            }

            let scan_components = self.reader.scan_component_indices.clone();
            let mut bit_reader = JpegBitReader::new(self.reader.remaining_data());
            let restart_interval = self.reader.restart_interval as usize;
            let mut mcus_decoded = 0;

            let ss = self.reader.ss;
            let se = self.reader.se;
            let ah = self.reader.ah;
            let al = self.reader.al;

            if scan_components.len() > 1 {
                let total_mcus = blocks_h * blocks_w;
                for block_y in 0..blocks_h {
                    for block_x in 0..blocks_w {
                        if restart_interval > 0
                            && mcus_decoded > 0
                            && (mcus_decoded % restart_interval == 0)
                            && mcus_decoded < total_mcus
                        {
                            bit_reader.align_to_byte();
                            let _ = bit_reader.read_bits(16)?;
                            for i in 0..components_count {
                                dc_preds[i] = 0;
                                eob_runs[i] = 0;
                            }
                        }

                        for &comp_idx in &scan_components {
                            let block_offset = (block_y * blocks_w + block_x) * 64;
                            let target_block =
                                &mut coefficient_buffers[comp_idx][block_offset..block_offset + 64];

                            if self.reader.is_progressive {
                                if ss == 0 {
                                    self.decode_dc_progressive(
                                        &mut bit_reader,
                                        &mut dc_preds[comp_idx],
                                        target_block,
                                        ah,
                                        al,
                                        comp_idx,
                                    )?;
                                } else {
                                    self.decode_ac_progressive(
                                        &mut bit_reader,
                                        target_block,
                                        ss,
                                        se,
                                        ah,
                                        al,
                                        &mut eob_runs[comp_idx],
                                        comp_idx,
                                    )?;
                                }
                            } else {
                                Self::decode_block_internal(
                                    &mut bit_reader,
                                    self,
                                    &mut dc_preds[comp_idx],
                                    target_block,
                                    comp_idx,
                                )?;
                            }
                        }
                        mcus_decoded += 1;
                    }
                }
            } else {
                let comp_idx = scan_components[0];
                let total_blocks = blocks_h * blocks_w;
                for block_y in 0..blocks_h {
                    for block_x in 0..blocks_w {
                        if restart_interval > 0
                            && mcus_decoded > 0
                            && (mcus_decoded % restart_interval == 0)
                            && mcus_decoded < total_blocks
                        {
                            bit_reader.align_to_byte();
                            let _ = bit_reader.read_bits(16)?;
                            dc_preds[comp_idx] = 0;
                            eob_runs[comp_idx] = 0;
                        }

                        let block_offset = (block_y * blocks_w + block_x) * 64;
                        let target_block =
                            &mut coefficient_buffers[comp_idx][block_offset..block_offset + 64];

                        if self.reader.is_progressive {
                            if ss == 0 {
                                self.decode_dc_progressive(
                                    &mut bit_reader,
                                    &mut dc_preds[comp_idx],
                                    target_block,
                                    ah,
                                    al,
                                    comp_idx,
                                )?;
                            } else {
                                self.decode_ac_progressive(
                                    &mut bit_reader,
                                    target_block,
                                    ss,
                                    se,
                                    ah,
                                    al,
                                    &mut eob_runs[comp_idx],
                                    comp_idx,
                                )?;
                            }
                        } else {
                            Self::decode_block_internal(
                                &mut bit_reader,
                                self,
                                &mut dc_preds[comp_idx],
                                target_block,
                                comp_idx,
                            )?;
                        }
                        mcus_decoded += 1;
                    }
                }
            }
            self.reader.advance(bit_reader.position());
        }

        let mut component_buffers_f32 =
            vec![vec![0.0f32; blocks_w * blocks_h * 64]; components_count];
        for c in 0..components_count {
            let quant_idx = self.reader.components[c].quant_table_dest as usize;
            let quant_table = &self.reader.quantization_tables[quant_idx];
            for b in 0..(blocks_w * blocks_h) {
                let block_offset = b * 64;
                let mut block_data = [0i16; 64];
                block_data
                    .copy_from_slice(&coefficient_buffers[c][block_offset..block_offset + 64]);
                let mut dequant_coeffs = [0.0f32; 64];
                dequantize_block(&block_data, quant_table, &mut dequant_coeffs);
                let mut idct_out = [0.0f32; 64];
                crate::jpeg1::dct::idct_8x8_fixed_point(&dequant_coeffs, &mut idct_out);
                component_buffers_f32[c][block_offset..block_offset + 64]
                    .copy_from_slice(&idct_out);
            }
        }

        for py in 0..height {
            for px in 0..width {
                let bx = px / 8;
                let by = py / 8;
                let tx = px % 8;
                let ty = py % 8;
                let block_idx = (by * blocks_w + bx) * 64 + (ty * 8 + tx);

                if components_count == 1 {
                    let val = (component_buffers_f32[0][block_idx] + 128.0)
                        .round()
                        .clamp(0.0, 255.0) as u8;
                    destination[py * width + px] = val;
                } else if components_count == 3 {
                    let y_val = component_buffers_f32[0][block_idx];
                    let cb_val = component_buffers_f32[1][block_idx];
                    let cr_val = component_buffers_f32[2][block_idx];
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

    fn decode_block_internal(
        bit_reader: &mut JpegBitReader,
        decoder: &Jpeg1Decoder,
        dc_prev: &mut i16,
        output: &mut [i16],
        comp_idx: usize,
    ) -> Result<(), JpeglsError> {
        let comp = &decoder.reader.components[comp_idx];
        let dc_table = decoder.reader.huffman_tables_dc[comp.dc_table_dest as usize]
            .as_ref()
            .ok_or(JpeglsError::InvalidData)?;
        let ac_table = decoder.reader.huffman_tables_ac[comp.ac_table_dest as usize]
            .as_ref()
            .ok_or(JpeglsError::InvalidData)?;

        let dc_category = dc_table.decode(bit_reader)?;
        let dc_diff_bits = bit_reader.read_bits(dc_category)?;
        let dc_diff = HuffmanEncoder::decode_value_bits(dc_diff_bits, dc_category);
        let dc_val = *dc_prev + dc_diff;
        *dc_prev = dc_val;
        output[0] = dc_val;

        let mut k = 1;
        while k < 64 {
            let symbol = ac_table.decode(bit_reader)?;
            if symbol == 0 {
                break;
            }
            if symbol == 0xF0 {
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

    fn decode_dc_progressive(
        &self,
        bit_reader: &mut JpegBitReader,
        dc_prev: &mut i16,
        block: &mut [i16],
        ah: u8,
        al: u8,
        comp_idx: usize,
    ) -> Result<(), JpeglsError> {
        let dc_idx = self.reader.components[comp_idx].dc_table_dest as usize;
        let dc_table = self.reader.huffman_tables_dc[dc_idx]
            .as_ref()
            .ok_or(JpeglsError::InvalidData)?;
        if ah == 0 {
            let cat = dc_table.decode(bit_reader)?;
            let bits = bit_reader.read_bits(cat)?;
            let diff = HuffmanEncoder::decode_value_bits(bits, cat);
            let val = *dc_prev + diff;
            *dc_prev = val;
            block[0] = val << al;
        } else {
            let bit = bit_reader.read_bits(1)?;
            block[0] |= (bit as i16) << al;
        }
        Ok(())
    }

    fn decode_ac_progressive(
        &self,
        bit_reader: &mut JpegBitReader,
        block: &mut [i16],
        ss: u8,
        se: u8,
        ah: u8,
        al: u8,
        eob_run: &mut u16,
        comp_idx: usize,
    ) -> Result<(), JpeglsError> {
        let ac_idx = self.reader.components[comp_idx].ac_table_dest as usize;
        let ac_table = self.reader.huffman_tables_ac[ac_idx]
            .as_ref()
            .ok_or(JpeglsError::InvalidData)?;

        if ah == 0 {
            if *eob_run > 0 {
                *eob_run -= 1;
                return Ok(());
            }
            let mut k = ss as usize;
            while k <= se as usize {
                let symbol = ac_table.decode(bit_reader)?;
                let run = (symbol >> 4) as usize;
                let cat = symbol & 0x0F;
                if cat > 0 {
                    k += run;
                    if k > se as usize {
                        break;
                    }
                    let bits = bit_reader.read_bits(cat)?;
                    let val = HuffmanEncoder::decode_value_bits(bits, cat);
                    block[crate::jpeg1::encoder::ZIGZAG_ORDER[k]] = val << al;
                    k += 1;
                } else {
                    if run < 15 {
                        let extra = bit_reader.read_bits(run as u8)?;
                        *eob_run = (1 << run) + extra - 1;
                        break;
                    } else {
                        k += 16;
                    }
                }
            }
        } else {
            let mut k = ss as usize;
            if *eob_run > 0 {
                while k <= se as usize {
                    let idx = crate::jpeg1::encoder::ZIGZAG_ORDER[k];
                    if block[idx] != 0 {
                        if bit_reader.read_bits(1)? != 0 {
                            if block[idx] > 0 {
                                block[idx] += 1 << al;
                            } else {
                                block[idx] -= 1 << al;
                            }
                        }
                    }
                    k += 1;
                }
                *eob_run -= 1;
                return Ok(());
            }

            while k <= se as usize {
                let symbol = ac_table.decode(bit_reader)?;
                let run = (symbol >> 4) as usize;
                let cat = symbol & 0x0F;

                if cat > 0 {
                    let mut r = run;
                    while k <= se as usize {
                        let idx = crate::jpeg1::encoder::ZIGZAG_ORDER[k];
                        if block[idx] != 0 {
                            if bit_reader.read_bits(1)? != 0 {
                                if block[idx] > 0 {
                                    block[idx] += 1 << al;
                                } else {
                                    block[idx] -= 1 << al;
                                }
                            }
                        } else {
                            if r == 0 {
                                break;
                            }
                            r -= 1;
                        }
                        k += 1;
                    }
                    if k <= se as usize {
                        let bits = bit_reader.read_bits(1)?;
                        let idx = crate::jpeg1::encoder::ZIGZAG_ORDER[k];
                        block[idx] = if bits != 0 { 1 << al } else { -(1 << al) };
                        k += 1;
                    }
                } else {
                    if run < 15 {
                        let extra = bit_reader.read_bits(run as u8)?;
                        *eob_run = (1 << run) + extra;
                        while k <= se as usize {
                            let idx = crate::jpeg1::encoder::ZIGZAG_ORDER[k];
                            if block[idx] != 0 {
                                if bit_reader.read_bits(1)? != 0 {
                                    if block[idx] > 0 {
                                        block[idx] += 1 << al;
                                    } else {
                                        block[idx] -= 1 << al;
                                    }
                                }
                            }
                            k += 1;
                        }
                        *eob_run -= 1;
                        break;
                    } else {
                        let mut r = 16;
                        while k <= se as usize && r > 0 {
                            let idx = crate::jpeg1::encoder::ZIGZAG_ORDER[k];
                            if block[idx] != 0 {
                                if bit_reader.read_bits(1)? != 0 {
                                    if block[idx] > 0 {
                                        block[idx] += 1 << al;
                                    } else {
                                        block[idx] -= 1 << al;
                                    }
                                }
                            } else {
                                r -= 1;
                            }
                            k += 1;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn decode_lossless(&mut self, destination: &mut [u8]) -> Result<(), JpeglsError> {
        let frame_info = self.reader.frame_info();
        let width = frame_info.width as usize;
        let height = frame_info.height as usize;
        let components_count = self.reader.components.len();
        let bit_depth = frame_info.bits_per_sample as u8;

        let mut component_pixels = vec![Vec::new(); components_count];

        loop {
            let marker = self.reader.peek_marker();
            match marker {
                Ok(crate::jpeg_marker_code::JpegMarkerCode::StartOfScan) => {
                    self.reader.read_start_of_scan_segment_jpeg1()?;
                }
                Ok(crate::jpeg_marker_code::JpegMarkerCode::DefineHuffmanTable) => {
                    self.reader.read_dht_segment()?;
                    continue;
                }
                Ok(crate::jpeg_marker_code::JpegMarkerCode::Comment)
                | Ok(crate::jpeg_marker_code::JpegMarkerCode::ApplicationData0) => {
                    self.reader.skip_segment()?;
                    continue;
                }
                _ => break,
            }

            let scan_components = self.reader.scan_component_indices.clone();
            let predictor_id = self.reader.lossless_predictor_selection;
            let mut bit_reader = JpegBitReader::new(self.reader.remaining_data());

            for &comp_idx in &scan_components {
                let dc_table_idx = self.reader.components[comp_idx].dc_table_dest as usize;
                let huffman_table = self.reader.huffman_tables_dc[dc_table_idx]
                    .as_ref()
                    .ok_or(JpeglsError::InvalidData)?;

                let pixels = crate::jpeg1::lossless::Jpeg1LosslessDecoder::decode_component(
                    predictor_id,
                    width,
                    height,
                    bit_depth,
                    &mut bit_reader,
                    huffman_table,
                )?;
                component_pixels[comp_idx] = pixels;
            }
            self.reader.advance(bit_reader.position());
        }

        for py in 0..height {
            for px in 0..width {
                let pixel_idx = (py * width + px) * components_count;
                for c in 0..components_count {
                    if component_pixels[c].is_empty() {
                        continue;
                    }
                    let val =
                        component_pixels[c][py * width + px].clamp(0, (1 << bit_depth) - 1) as u8;
                    if pixel_idx + c < destination.len() {
                        destination[pixel_idx + c] = val;
                    }
                }
            }
        }
        Ok(())
    }
}
