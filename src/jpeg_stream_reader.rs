use crate::jpegls::coding_parameters::{CodingParameters, JpeglsPcParameters};
use crate::error::JpeglsError;
use crate::jpeg_marker_code::{JPEG_MARKER_START_BYTE, JpegMarkerCode};
use crate::jpegls::{InterleaveMode, SpiffHeader};
use crate::FrameInfo;
use std::convert::{TryFrom, TryInto};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JpegStreamReaderState {
    BeforeStartOfImage,
    HeaderSection,
    ScanSection,
    EndOfImage,
}

pub struct JpegStreamReader<'a> {
    source: &'a [u8],
    position: usize,
    state: JpegStreamReaderState,
    frame_info: FrameInfo,
    parameters: CodingParameters,
    preset_coding_parameters: JpeglsPcParameters,
    spiff_header: Option<SpiffHeader>,
    pub quantization_tables: [[u8; 64]; 4],
    pub huffman_tables_dc: Vec<Option<crate::jpeg1::huffman::HuffmanTable>>,
    pub huffman_tables_ac: Vec<Option<crate::jpeg1::huffman::HuffmanTable>>,
}

impl<'a> JpegStreamReader<'a> {
    pub fn new(source: &'a [u8]) -> Self {
        Self {
            source,
            position: 0,
            state: JpegStreamReaderState::BeforeStartOfImage,
            frame_info: FrameInfo::default(),
            parameters: CodingParameters::default(),
            preset_coding_parameters: JpeglsPcParameters::default(),
            spiff_header: None,
            quantization_tables: [[0u8; 64]; 4],
            huffman_tables_dc: vec![None, None, None, None],
            huffman_tables_ac: vec![None, None, None, None],
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
                JpegMarkerCode::DefineQuantizationTable => {
                    self.read_dqt_segment()?;
                }
                JpegMarkerCode::DefineHuffmanTable => {
                    self.read_dht_segment()?;
                }
                _ => {
                    self.skip_segment()?;
                }
            }
        }
        Ok(())
    }

    fn read_byte(&mut self) -> Result<u8, JpeglsError> {
        if self.position >= self.source.len() {
            return Err(JpeglsError::InvalidData);
        }
        let val = self.source[self.position];
        self.position += 1;
        Ok(val)
    }

    fn read_u16(&mut self) -> Result<u16, JpeglsError> {
        let b1 = self.read_byte()? as u16;
        let b2 = self.read_byte()? as u16;
        Ok((b1 << 8) | b2)
    }

    fn read_marker(&mut self) -> Result<JpegMarkerCode, JpeglsError> {
        if self.read_byte()? != JPEG_MARKER_START_BYTE {
            return Err(JpeglsError::InvalidData);
        }
        let marker_byte = self.read_byte()?;
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
        self.frame_info.bits_per_sample = self.read_byte()? as i32;
        self.frame_info.height = self.read_u16()? as u32;
        self.frame_info.width = self.read_u16()? as u32;
        self.frame_info.component_count = self.read_byte()? as i32;

        for _ in 0..self.frame_info.component_count {
            let _id = self.read_byte()?;
            let _sampling = self.read_byte()?;
            let _tq = self.read_byte()?;
        }
        Ok(())
    }

    fn read_jpegls_preset_parameters_segment(&mut self) -> Result<(), JpeglsError> {
        let _length = self.read_u16()?;
        let param_type = self.read_byte()?;
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
        let _length = self.read_u16()?;
        let components_in_scan = self.read_byte()? as i32;
        for _ in 0..components_in_scan {
            let _id = self.read_byte()?;
            let _mapping = self.read_byte()?;
        }
        self.parameters.near_lossless = self.read_byte()? as i32;
        self.parameters.interleave_mode = InterleaveMode::try_from(self.read_byte()?)?;
        let _point_transform = self.read_byte()?;

        self.state = JpegStreamReaderState::ScanSection;
        Ok(())
    }

    pub fn read_start_of_scan_segment_jpeg1(&mut self) -> Result<(), JpeglsError> {
        let _length = self.read_u16()?;
        let components_in_scan = self.read_byte()? as i32;
        for _ in 0..components_in_scan {
            let _id = self.read_byte()?;
            let _selector = self.read_byte()?;
        }
        let _ss = self.read_byte()?;
        let _se = self.read_byte()?;
        let _ah_al = self.read_byte()?;

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
            identifier[i] = self.read_byte()?;
        }

        if identifier != [b'S', b'P', b'I', b'F', b'F', 0] {
            self.position += length - 8;
            return Ok(None);
        }

        let _version_major = self.read_byte()?;
        let _version_minor = self.read_byte()?;
        let profile_id = self.read_byte()?;
        let component_count = self.read_byte()?;
        let height = self.read_u32_internal()?;
        let width = self.read_u32_internal()?;
        let color_space = self.read_byte()?;
        let bits_per_sample = self.read_byte()?;
        let compression_type = self.read_byte()?;
        let resolution_units = self.read_byte()?;
        let vertical_resolution = self.read_u32_internal()?;
        let horizontal_resolution = self.read_u32_internal()?;

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

    fn read_u32_internal(&mut self) -> Result<u32, JpeglsError> {
        let b1 = self.read_byte()? as u32;
        let b2 = self.read_byte()? as u32;
        let b3 = self.read_byte()? as u32;
        let b4 = self.read_byte()? as u32;
        Ok((b1 << 24) | (b2 << 16) | (b3 << 8) | b4)
    }

    fn skip_segment(&mut self) -> Result<(), JpeglsError> {
        let length = self.read_u16()?;
        if length < 2 {
            return Err(JpeglsError::InvalidData);
        }
        self.position += (length as usize) - 2;
        Ok(())
    }

    fn read_sof0_segment(&mut self) -> Result<(), JpeglsError> {
        let _length = self.read_u16()?;
        self.frame_info.bits_per_sample = self.read_byte()? as i32;
        self.frame_info.height = self.read_u16()? as u32;
        self.frame_info.width = self.read_u16()? as u32;
        self.frame_info.component_count = self.read_byte()? as i32;

        for _ in 0..self.frame_info.component_count {
            let _id = self.read_byte()?;
            let _sampling = self.read_byte()?;
            let _tq = self.read_byte()?;
        }
        Ok(())
    }

    fn read_dqt_segment(&mut self) -> Result<(), JpeglsError> {
        let length = self.read_u16()? as usize;
        let mut remaining = length - 2;
        while remaining >= 65 {
            let pq_tq = self.read_byte()?;
            let precision = pq_tq >> 4;
            let id = (pq_tq & 0x0F) as usize;
            if id >= 4 || precision != 0 {
                return Err(JpeglsError::ParameterValueNotSupported);
            }
            for i in 0..64 {
                self.quantization_tables[id][i] = self.read_byte()?;
            }
            remaining -= 65;
        }
        Ok(())
    }

    fn read_dht_segment(&mut self) -> Result<(), JpeglsError> {
        let length = self.read_u16()? as usize;
        let mut remaining = length - 2;
        while remaining >= 17 {
            let tc_th = self.read_byte()?;
            let class = tc_th >> 4;
            let id = (tc_th & 0x0F) as usize;
            if id >= 4 {
                return Err(JpeglsError::ParameterValueNotSupported);
            }

            let mut lengths = [0u8; 16];
            let mut total_values = 0usize;
            for i in 0..16 {
                lengths[i] = self.read_byte()?;
                total_values += lengths[i] as usize;
            }
            remaining -= 17;

            if remaining < total_values {
                return Err(JpeglsError::InvalidData);
            }

            let mut values = vec![0u8; total_values];
            for i in 0..total_values {
                values[i] = self.read_byte()?;
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
}
