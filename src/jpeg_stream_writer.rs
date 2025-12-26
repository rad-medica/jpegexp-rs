use crate::error::JpeglsError;
use crate::jpeg_marker_code::{JPEG_MARKER_START_BYTE, JpegMarkerCode};
use crate::jpegls::{InterleaveMode, JpeglsPcParameters};
use crate::FrameInfo;

pub struct JpegStreamWriter<'a> {
    destination: &'a mut [u8],
    position: usize,
}

impl<'a> JpegStreamWriter<'a> {
    pub fn new(destination: &'a mut [u8]) -> Self {
        Self {
            destination,
            position: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.position
    }

    pub fn write_byte(&mut self, value: u8) -> Result<(), JpeglsError> {
        if self.position >= self.destination.len() {
            return Err(JpeglsError::ParameterValueNotSupported); // Use appropriate error (BufferTooSmall)
        }
        self.destination[self.position] = value;
        self.position += 1;
        Ok(())
    }

    pub fn write_u16(&mut self, value: u16) -> Result<(), JpeglsError> {
        let bytes = value.to_be_bytes();
        self.write_byte(bytes[0])?;
        self.write_byte(bytes[1])?;
        Ok(())
    }

    pub fn write_u32(&mut self, value: u32) -> Result<(), JpeglsError> {
        let bytes = value.to_be_bytes();
        self.write_byte(bytes[0])?;
        self.write_byte(bytes[1])?;
        self.write_byte(bytes[2])?;
        self.write_byte(bytes[3])?;
        Ok(())
    }

    pub fn write_marker(&mut self, marker: JpegMarkerCode) -> Result<(), JpeglsError> {
        self.write_byte(JPEG_MARKER_START_BYTE)?;
        self.write_byte(marker as u8)?;
        Ok(())
    }

    pub fn write_start_of_image(&mut self) -> Result<(), JpeglsError> {
        self.write_marker(JpegMarkerCode::StartOfImage)
    }

    pub fn write_end_of_image(&mut self) -> Result<(), JpeglsError> {
        self.write_marker(JpegMarkerCode::EndOfImage)
    }

    pub fn write_start_of_frame_jpegls(
        &mut self,
        frame_info: &FrameInfo,
    ) -> Result<(), JpeglsError> {
        self.write_marker(JpegMarkerCode::StartOfFrameJpegls)?;
        let length = 2 + 6 + (frame_info.component_count as usize * 3);
        self.write_u16(length as u16)?;

        self.write_byte(frame_info.bits_per_sample as u8)?;
        self.write_u16(frame_info.height as u16)?;
        self.write_u16(frame_info.width as u16)?;
        self.write_byte(frame_info.component_count as u8)?;

        for i in 0..frame_info.component_count {
            self.write_byte((i + 1) as u8)?; // Component ID
            self.write_byte(0x11)?; // H=1, V=1
            self.write_byte(0)?; // Tq
        }
        Ok(())
    }

    pub fn write_dqt(&mut self, table_id: u8, table: &[u8; 64]) -> Result<(), JpeglsError> {
        self.write_marker(JpegMarkerCode::DefineQuantizationTable)?;
        self.write_u16(2 + 1 + 64)?;
        self.write_byte(table_id & 0x0F)?; // Precision 0 (8-bit), ID
        for &val in table {
            self.write_byte(val)?;
        }
        Ok(())
    }

    pub fn write_dht(&mut self, table_class: u8, table_id: u8, lengths: &[u8; 16], values: &[u8]) -> Result<(), JpeglsError> {
        self.write_marker(JpegMarkerCode::DefineHuffmanTable)?;
        let length = 2 + 1 + 16 + values.len();
        self.write_u16(length as u16)?;
        self.write_byte(((table_class & 1) << 4) | (table_id & 0x0F))?;
        for &len in lengths {
            self.write_byte(len)?;
        }
        for &val in values {
            self.write_byte(val)?;
        }
        Ok(())
    }

    pub fn write_sof0_segment(&mut self, frame_info: &FrameInfo) -> Result<(), JpeglsError> {
        self.write_marker(JpegMarkerCode::StartOfFrameBaseline)?;
        let length = 2 + 1 + 2 + 2 + 1 + (frame_info.component_count as usize * 3);
        self.write_u16(length as u16)?;
        self.write_byte(frame_info.bits_per_sample as u8)?;
        self.write_u16(frame_info.height as u16)?;
        self.write_u16(frame_info.width as u16)?;
        self.write_byte(frame_info.component_count as u8)?;

        for i in 0..frame_info.component_count {
            self.write_byte((i + 1) as u8)?;
            self.write_byte(0x11)?; // Sampling factors 1x1
            self.write_byte(0)?; // Quantization table ID
        }
        Ok(())
    }

    pub fn write_sos_segment(&mut self, component_count: u8) -> Result<(), JpeglsError> {
        self.write_marker(JpegMarkerCode::StartOfScan)?;
        let length = 2 + 1 + (component_count as usize * 2) + 3;
        self.write_u16(length as u16)?;
        self.write_byte(component_count)?;
        for i in 0..component_count {
            self.write_byte(i + 1)?; // Component selector
            self.write_byte(0x00)?; // DC/AC entropy table destination (DC 0, AC 0)
        }
        self.write_byte(0)?; // Ss
        self.write_byte(63)?; // Se
        self.write_byte(0)?; // Ah/Al
        Ok(())
    }

    pub fn write_start_of_scan_segment(
        &mut self,
        component_count: i32,
        near_lossless: i32,
        interleave_mode: InterleaveMode,
    ) -> Result<(), JpeglsError> {
        self.write_marker(JpegMarkerCode::StartOfScan)?;
        let length = 2 + 1 + (component_count as usize * 2) + 3;
        self.write_u16(length as u16)?;

        self.write_byte(component_count as u8)?;
        for i in 0..component_count {
            self.write_byte((i + 1) as u8)?; // Component Selector (assuming 1-based sequential)
            self.write_byte(0)?; // Mapping table selector
        }

        self.write_byte(near_lossless as u8)?;
        self.write_byte(interleave_mode as u8)?;
        self.write_byte(0)?; // Ah, Al point transform (0)

        Ok(())
    }

    pub fn write_jpegls_preset_parameters_segment(
        &mut self,
        pc: &JpeglsPcParameters,
    ) -> Result<(), JpeglsError> {
        self.write_marker(JpegMarkerCode::JpeglsPresetParameters)?;
        let length = 2 + 1 + 5 * 2;
        self.write_u16(length as u16)?;
        self.write_byte(1)?; // Type 1: Preset coding parameters

        self.write_u16(pc.maximum_sample_value as u16)?;
        self.write_u16(pc.threshold1 as u16)?;
        self.write_u16(pc.threshold2 as u16)?;
        self.write_u16(pc.threshold3 as u16)?;
        self.write_u16(pc.reset_value as u16)?;

        Ok(())
    }

    pub fn remaining_slice(&mut self) -> &mut [u8] {
        if self.position >= self.destination.len() {
            &mut []
        } else {
            &mut self.destination[self.position..]
        }
    }

    pub fn advance(&mut self, count: usize) {
        self.position += count;
    }
}
