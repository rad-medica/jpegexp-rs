//! JPEG Codestream Reader utilities.
//!
//! This module provides the `JpegStreamReader` which handles the sequential
//! reading of JPEG markers and segments (DQT, DHT, SOF, SOS, etc.).

use crate::FrameInfo;
use crate::error::JpeglsError;
use crate::jpeg_marker_code::{JPEG_MARKER_START_BYTE, JpegMarkerCode};
use crate::jpegls::coding_parameters::{CodingParameters, JpeglsPcParameters};
use crate::jpegls::{InterleaveMode, SpiffHeader};
use std::convert::{TryFrom, TryInto};

/// Metadata for an individual image component (e.g. Y, Cb, Cr).
#[derive(Debug, Clone, Default)]
pub struct JpegComponent {
    /// Component ID (typically 1, 2, 3).
    pub id: u8,
    /// Horizontal sampling factor (1-4).
    pub h_samp_factor: u8,
    /// Vertical sampling factor (1-4).
    pub v_samp_factor: u8,
    /// Quantization table destination selector (0-3).
    pub quant_table_dest: u8,
    /// DC Huffman table destination selector (0-3).
    pub dc_table_dest: u8,
    /// AC Huffman table destination selector (0-3).
    pub ac_table_dest: u8,
    /// DC prediction state (used during scan decoding).
    pub dc_pred: i16,
}

/// Internal state of the stream reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JpegStreamReaderState {
    BeforeStartOfImage,
    HeaderSection,
    ScanSection,
    EndOfImage,
}

/// A reader for JPEG/JLS codestreams that manages marker parsing and segment state.
pub struct JpegStreamReader<'a> {
    source: &'a [u8],
    position: usize,
    bit_buffer: u8,
    bits_left: u8,
    /// For J2K: true if the previous byte read was 0xFF (next byte has 7 bits)
    prev_byte_was_ff: bool,
    state: JpegStreamReaderState,
    frame_info: FrameInfo,
    parameters: CodingParameters,
    preset_coding_parameters: JpeglsPcParameters,
    spiff_header: Option<SpiffHeader>,
    pub quantization_tables: [[u8; 64]; 4],
    pub huffman_tables_dc: [Option<crate::jpeg1::huffman::HuffmanTable>; 4],
    pub huffman_tables_ac: [Option<crate::jpeg1::huffman::HuffmanTable>; 4],
    pub components: Vec<JpegComponent>,
    pub restart_interval: u16,
    pub scan_component_indices: Vec<usize>,
    pub is_lossless: bool,
    pub is_progressive: bool,
    pub lossless_predictor_selection: u8,
    pub point_transform: u8,
    pub ss: u8,
    pub se: u8,
    pub ah: u8,
    pub al: u8,
}

impl<'a> JpegStreamReader<'a> {
    pub fn new(source: &'a [u8]) -> Self {
        Self {
            source,
            position: 0,
            bit_buffer: 0,
            bits_left: 0,
            prev_byte_was_ff: false,
            state: JpegStreamReaderState::BeforeStartOfImage,
            frame_info: FrameInfo::default(),
            parameters: CodingParameters::default(),
            preset_coding_parameters: JpeglsPcParameters::default(),
            spiff_header: None,
            quantization_tables: [[0u8; 64]; 4],
            huffman_tables_dc: [const { None }; 4],
            huffman_tables_ac: [const { None }; 4],
            components: Vec::new(),
            restart_interval: 0,
            scan_component_indices: Vec::new(),
            is_lossless: false,
            is_progressive: false,
            lossless_predictor_selection: 0,
            point_transform: 0,
            ss: 0,
            se: 63,
            ah: 0,
            al: 0,
        }
    }

    pub fn frame_info(&self) -> FrameInfo {
        self.frame_info
    }

    pub fn parameters(&self) -> CodingParameters {
        self.parameters
    }

    pub fn preset_coding_parameters(&self) -> JpeglsPcParameters {
        self.preset_coding_parameters
    }

    pub fn spiff_header(&self) -> Option<SpiffHeader> {
        self.spiff_header
    }

    pub fn remaining_data(&self) -> &[u8] {
        &self.source[self.position..]
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn read_header(
        &mut self,
        spiff_header: &mut Option<SpiffHeader>,
    ) -> Result<(), JpeglsError> {
        self.read_start_of_image()?;

        loop {
            let marker = self.read_marker()?;
            match marker {
                JpegMarkerCode::StartOfFrameJpegls => {
                    self.read_start_of_frame_segment()?;
                }
                JpegMarkerCode::JpeglsPresetParameters => {
                    self.read_jpegls_preset_parameters_segment()?;
                }
                JpegMarkerCode::StartOfScan => {
                    self.position -= 2;
                    self.state = JpegStreamReaderState::HeaderSection;
                    break;
                }
                JpegMarkerCode::ApplicationData8 => {
                    let spiff = self.read_spiff_header_segment()?;
                    if spiff.is_some() {
                        *spiff_header = spiff;
                        self.spiff_header = spiff;
                    }
                }
                JpegMarkerCode::StartOfFrameBaseline => {
                    self.read_sof0_segment()?;
                }
                JpegMarkerCode::StartOfFrameProgressive => {
                    self.read_sof2_segment()?;
                }
                JpegMarkerCode::StartOfFrameLossless => {
                    self.read_sof3_segment()?;
                }
                JpegMarkerCode::DefineQuantizationTable => {
                    self.read_dqt_segment()?;
                }
                JpegMarkerCode::DefineHuffmanTable => {
                    self.read_dht_segment()?;
                }
                JpegMarkerCode::DefineRestartInterval => {
                    self.read_dri_segment()?;
                }
                _ => {
                    self.skip_segment()?;
                }
            }
        }
        Ok(())
    }

    pub fn read_u8(&mut self) -> Result<u8, JpeglsError> {
        if self.position >= self.source.len() {
            return Err(JpeglsError::InvalidData);
        }
        let b = self.source[self.position];
        self.position += 1;
        Ok(b)
    }

    #[allow(dead_code)]
    fn read_byte(&mut self) -> Result<u8, JpeglsError> {
        self.read_u8()
    }

    pub fn read_u16(&mut self) -> Result<u16, JpeglsError> {
        let b1 = self.read_u8()? as u16;
        let b2 = self.read_u8()? as u16;
        Ok((b1 << 8) | b2)
    }

    pub fn read_u32(&mut self) -> Result<u32, JpeglsError> {
        let b1 = self.read_u8()? as u32;
        let b2 = self.read_u8()? as u32;
        let b3 = self.read_u8()? as u32;
        let b4 = self.read_u8()? as u32;
        Ok((b1 << 24) | (b2 << 16) | (b3 << 8) | b4)
    }

    pub fn peek_marker(&self) -> Result<JpegMarkerCode, JpeglsError> {
        if self.position + 1 >= self.source.len() {
            return Err(JpeglsError::InvalidData);
        }
        if self.source[self.position] != JPEG_MARKER_START_BYTE {
            return Err(JpeglsError::InvalidData);
        }
        JpegMarkerCode::try_from(self.source[self.position + 1])
    }

    pub fn read_marker(&mut self) -> Result<JpegMarkerCode, JpeglsError> {
        if self.read_u8()? != JPEG_MARKER_START_BYTE {
            return Err(JpeglsError::InvalidData);
        }
        let marker_byte = self.read_u8()?;
        JpegMarkerCode::try_from(marker_byte)
    }

    fn read_start_of_image(&mut self) -> Result<(), JpeglsError> {
        if self.read_marker()? != JpegMarkerCode::StartOfImage {
            return Err(JpeglsError::InvalidData);
        }
        self.state = JpegStreamReaderState::HeaderSection;
        Ok(())
    }

    fn read_start_of_frame_segment(&mut self) -> Result<(), JpeglsError> {
        let _length = self.read_u16()?;
        self.frame_info.bits_per_sample = self.read_u8()? as i32;
        self.frame_info.height = self.read_u16()? as u32;
        self.frame_info.width = self.read_u16()? as u32;
        self.frame_info.component_count = self.read_u8()? as i32;

        for _ in 0..self.frame_info.component_count {
            let _id = self.read_u8()?;
            let _sampling = self.read_u8()?;
            let _tq = self.read_u8()?;
        }
        Ok(())
    }

    fn read_jpegls_preset_parameters_segment(&mut self) -> Result<(), JpeglsError> {
        let _length = self.read_u16()?;
        let param_type = self.read_u8()?;
        if param_type == 1 {
            self.preset_coding_parameters.maximum_sample_value = self.read_u16()? as i32;
            self.preset_coding_parameters.threshold1 = self.read_u16()? as i32;
            self.preset_coding_parameters.threshold2 = self.read_u16()? as i32;
            self.preset_coding_parameters.threshold3 = self.read_u16()? as i32;
            self.preset_coding_parameters.reset_value = self.read_u16()? as i32;
        } else {
            self.position += (_length as usize) - 3;
        }
        Ok(())
    }

    pub fn read_start_of_scan_segment_jpegls(&mut self) -> Result<(), JpeglsError> {
        if self.read_marker()? != JpegMarkerCode::StartOfScan {
            return Err(JpeglsError::InvalidData);
        }
        let length = self.read_u16()?;
        let mut consumed = 2; // Length field itself (2 bytes)

        let components_in_scan = self.read_u8()? as i32;
        consumed += 1;
        for _ in 0..components_in_scan {
            let _id = self.read_u8()?;
            let _mapping = self.read_u8()?;
            consumed += 2;
        }
        self.parameters.near_lossless = self.read_u8()? as i32;
        self.parameters.interleave_mode = InterleaveMode::try_from(self.read_u8()?)?;
        let _point_transform = self.read_u8()?;
        consumed += 3;

        if (length as i32) > consumed {
            let skip = (length as i32) - consumed;
            self.advance(skip as usize);
        }

        self.state = JpegStreamReaderState::ScanSection;
        Ok(())
    }

    pub fn read_start_of_scan_segment_jpeg1(&mut self) -> Result<(), JpeglsError> {
        if self.read_marker()? != JpegMarkerCode::StartOfScan {
            return Err(JpeglsError::InvalidData);
        }
        let _length = self.read_u16()?;
        let components_in_scan = self.read_u8()? as i32;
        self.scan_component_indices.clear();

        for _ in 0..components_in_scan {
            let id = self.read_u8()?;
            let selector = self.read_u8()?;
            let dc_dest = selector >> 4;
            let ac_dest = selector & 0x0F;

            for (idx, component) in self.components.iter_mut().enumerate() {
                if component.id == id {
                    component.dc_table_dest = dc_dest;
                    component.ac_table_dest = ac_dest;
                    self.scan_component_indices.push(idx);
                    break;
                }
            }
        }
        let ss = self.read_u8()?;
        let se = self.read_u8()?;
        let ah_al = self.read_u8()?;

        self.ss = ss;
        self.se = se;
        self.ah = ah_al >> 4;
        self.al = ah_al & 0x0F;

        if self.is_lossless {
            self.lossless_predictor_selection = ss;
            self.point_transform = self.al;
        }

        self.state = JpegStreamReaderState::ScanSection;
        Ok(())
    }

    fn read_spiff_header_segment(&mut self) -> Result<Option<SpiffHeader>, JpeglsError> {
        let length = self.read_u16()? as usize;
        if length < 32 {
            self.position += length - 2;
            return Ok(None);
        }

        let mut identifier = [0u8; 6];
        for i in 0..6 {
            identifier[i] = self.read_u8()?;
        }

        if identifier != [b'S', b'P', b'I', b'F', b'F', 0] {
            self.position += length - 8;
            return Ok(None);
        }

        let _version_major = self.read_u8()?;
        let _version_minor = self.read_u8()?;
        let profile_id = self.read_u8()?;
        let component_count = self.read_u8()?;
        let height = self.read_u32()?;
        let width = self.read_u32()?;
        let color_space = self.read_u8()?;
        let bits_per_sample = self.read_u8()?;
        let compression_type = self.read_u8()?;
        let resolution_units = self.read_u8()?;
        let vertical_resolution = self.read_u32()?;
        let horizontal_resolution = self.read_u32()?;

        Ok(Some(SpiffHeader {
            profile_id: profile_id.try_into()?,
            component_count: component_count as i32,
            height,
            width,
            color_space: color_space.try_into()?,
            bits_per_sample: bits_per_sample as i32,
            compression_type: compression_type.try_into()?,
            resolution_units: resolution_units.try_into()?,
            vertical_resolution,
            horizontal_resolution,
        }))
    }

    // Deprecated? No, used in other methods I didn't verify fully?
    // I replaced read_u32_internal usage with read_u32 above.

    pub fn skip_segment(&mut self) -> Result<(), JpeglsError> {
        let length = self.read_u16()?;
        if length < 2 {
            return Err(JpeglsError::InvalidData);
        }
        self.position += (length as usize) - 2;
        Ok(())
    }

    pub fn advance(&mut self, count: usize) {
        self.position += count;
    }

    // Helper to align to the next byte boundary.
    // Since this reader operates on a byte slice, it is always byte-aligned.
    // This is a no-op for now but kept for API compatibility with bit-stream readers.
    pub fn align_to_byte(&mut self) {
        self.bits_left = 0;
        self.bit_buffer = 0;
        self.prev_byte_was_ff = false;
    }

    pub fn read_bit(&mut self) -> Result<u8, JpeglsError> {
        if self.bits_left == 0 {
            if self.position >= self.source.len() {
                return Err(JpeglsError::InvalidData);
            }
            let b = self.source[self.position];
            self.position += 1;

            self.bit_buffer = b;
            
            // JPEG 2000 byte stuffing (per ISO/IEC 15444-1 Annex B.10.1):
            // After reading a 0xFF byte, the next byte has only 7 valid bits
            // (bit 7 is stuffed as 0 to prevent accidental marker codes)
            if self.prev_byte_was_ff {
                // Previous byte was 0xFF, so this byte only has 7 valid bits
                self.bits_left = 7;
            } else {
                self.bits_left = 8;
            }
            
            // Remember if this byte is 0xFF for next time
            self.prev_byte_was_ff = b == 0xFF;
        }

        let shift = self.bits_left - 1;
        let bit = (self.bit_buffer >> shift) & 1;

        self.bits_left -= 1;
        Ok(bit)
    }

    // JPEG 1 Headers

    fn read_sof0_segment(&mut self) -> Result<(), JpeglsError> {
        let _length = self.read_u16()?;
        self.frame_info.bits_per_sample = self.read_u8()? as i32;
        self.frame_info.height = self.read_u16()? as u32;
        self.frame_info.width = self.read_u16()? as u32;
        self.frame_info.component_count = self.read_u8()? as i32;

        self.components.clear();
        for _ in 0..self.frame_info.component_count {
            let id = self.read_u8()?;
            let sampling = self.read_u8()?;
            let tq = self.read_u8()?;
            self.components.push(JpegComponent {
                id,
                h_samp_factor: sampling >> 4,
                v_samp_factor: sampling & 0x0F,
                quant_table_dest: tq,
                dc_table_dest: 0,
                ac_table_dest: 0,
                dc_pred: 0,
            });
        }
        Ok(())
    }

    fn read_sof3_segment(&mut self) -> Result<(), JpeglsError> {
        // SOF3 is syntactically identical to SOF0 in terms of header structure,
        // but it implies lossless process.
        self.is_lossless = true;
        self.read_sof0_segment()
    }

    fn read_sof2_segment(&mut self) -> Result<(), JpeglsError> {
        // SOF2 is Progressive DCT.
        self.is_progressive = true;
        self.read_sof0_segment()
    }

    pub fn read_dqt_segment(&mut self) -> Result<(), JpeglsError> {
        let length = self.read_u16()? as usize;
        let mut remaining = length - 2;
        while remaining >= 65 {
            let pq_tq = self.read_u8()?;
            let precision = pq_tq >> 4;
            let id = (pq_tq & 0x0F) as usize;
            if id >= 4 || precision != 0 {
                return Err(JpeglsError::ParameterValueNotSupported);
            }
            for i in 0..64 {
                self.quantization_tables[id][i] = self.read_u8()?;
            }
            remaining -= 65;
        }
        Ok(())
    }

    pub fn read_dht_segment(&mut self) -> Result<(), JpeglsError> {
        let length = self.read_u16()? as usize;
        let mut remaining = length - 2;
        while remaining >= 17 {
            let tc_th = self.read_u8()?;
            let class = tc_th >> 4;
            let id = (tc_th & 0x0F) as usize;
            if id >= 4 {
                return Err(JpeglsError::ParameterValueNotSupported);
            }

            let mut lengths = [0u8; 16];
            let mut total_values = 0usize;
            for item in &mut lengths {
                *item = self.read_u8()?;
                total_values += *item as usize;
            }
            remaining -= 17;

            if remaining < total_values {
                return Err(JpeglsError::InvalidData);
            }

            let mut values = vec![0u8; total_values];
            for item in &mut values {
                *item = self.read_u8()?;
            }
            remaining -= total_values;

            let table = crate::jpeg1::huffman::HuffmanTable::build_from_dht(&lengths, &values);
            if class == 0 {
                self.huffman_tables_dc[id] = Some(table);
            } else {
                self.huffman_tables_ac[id] = Some(table);
            }
        }
        Ok(())
    }

    pub fn read_dri_segment(&mut self) -> Result<(), JpeglsError> {
        let length = self.read_u16()?;
        if length != 4 {
            return Err(JpeglsError::InvalidData);
        }
        self.restart_interval = self.read_u16()?;
        Ok(())
    }
}
