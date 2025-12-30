use crate::FrameInfo;
use crate::error::JpeglsError;
use crate::jpeg_stream_writer::JpegStreamWriter;
use crate::jpegls::coding_parameters::{compute_default, compute_limit_parameter};
use crate::jpegls::scan_encoder::ScanEncoder;
use crate::jpegls::{CodingParameters, InterleaveMode, JpeglsPcParameters};

pub struct JpeglsEncoder<'a> {
    writer: JpegStreamWriter<'a>,
    frame_info: Option<FrameInfo>,
    near_lossless: i32,
    interleave_mode: InterleaveMode,
    pc_parameters: Option<JpeglsPcParameters>,
}

impl<'a> JpeglsEncoder<'a> {
    pub fn new(destination: &'a mut [u8]) -> Self {
        Self {
            writer: JpegStreamWriter::new(destination),
            frame_info: None,
            near_lossless: 0,
            interleave_mode: InterleaveMode::None,
            pc_parameters: None,
        }
    }

    pub fn set_frame_info(&mut self, frame_info: FrameInfo) -> Result<(), JpeglsError> {
        self.frame_info = Some(frame_info);
        Ok(())
    }

    pub fn set_near_lossless(&mut self, near_lossless: i32) -> Result<(), JpeglsError> {
        self.near_lossless = near_lossless;
        Ok(())
    }

    pub fn set_interleave_mode(
        &mut self,
        interleave_mode: InterleaveMode,
    ) -> Result<(), JpeglsError> {
        self.interleave_mode = interleave_mode;
        Ok(())
    }

    pub fn set_preset_coding_parameters(
        &mut self,
        pc_parameters: JpeglsPcParameters,
    ) -> Result<(), JpeglsError> {
        self.pc_parameters = Some(pc_parameters);
        Ok(())
    }

    pub fn encode(&mut self, source: &[u8]) -> Result<usize, JpeglsError> {
        let frame_info = *self
            .frame_info
            .as_ref()
            .ok_or(JpeglsError::InvalidParameterComponentCount)?;

        let max_sample_value = (1 << frame_info.bits_per_sample) - 1;
        let pc = if let Some(p) = self.pc_parameters {
            p
        } else {
            compute_default(max_sample_value, self.near_lossless)
        };

        // Fallback to Planar (None) if InterleaveMode is None but we have components > 1.
        let interleave_mode = self.interleave_mode;

        let coding_parameters = CodingParameters {
            near_lossless: self.near_lossless,
            interleave_mode,
            restart_interval: 0,
            limit: compute_limit_parameter(
                frame_info.bits_per_sample,
                self.near_lossless,
                frame_info.component_count,
            ),
            quantized_bits_per_sample: frame_info.bits_per_sample,
            transformation: crate::jpegls::ColorTransformation::None,
            mapping_table_id: 0,
        };

        self.writer.write_start_of_image()?;
        self.writer.write_start_of_frame_jpegls(&frame_info)?;
        self.writer.write_jpegls_preset_parameters_segment(&pc)?;

        if interleave_mode == InterleaveMode::None && frame_info.component_count > 1 {
            // Encode separate scans for each component
            for c in 0..frame_info.component_count {
                // Write SOS for SINGLE component `c+1`
                self.writer.write_start_of_scan_segment_planar(
                    c as u8 + 1, // Component ID (1-based)
                    self.near_lossless,
                    InterleaveMode::None,
                )?;

                // Extract component data
                // Assume source is Interleaved (standard). We need to de-interleave.
                let component_index = c as usize;
                let width = frame_info.width as usize;
                let height = frame_info.height as usize;
                let total_components = frame_info.component_count as usize;

                if frame_info.bits_per_sample <= 8 {
                    let pixel_count = width * height;
                    let mut plane_data = vec![0u8; pixel_count];
                    for i in 0..pixel_count {
                        plane_data[i] = source[i * total_components + component_index];
                    }
                    self.encode_scan_typed::<u8>(
                        &plane_data,
                        &frame_info,
                        pc,
                        coding_parameters,
                        true,
                    )?;
                } else {
                    let (head, body, tail) = unsafe { source.align_to::<u16>() };
                    if !head.is_empty() || !tail.is_empty() {
                        return Err(JpeglsError::InvalidData);
                    }
                    let pixel_count = width * height;
                    let mut plane_data = vec![0u16; pixel_count];
                    for i in 0..pixel_count {
                        plane_data[i] = body[i * total_components + component_index];
                    }
                    self.encode_scan_typed::<u16>(
                        &plane_data,
                        &frame_info,
                        pc,
                        coding_parameters,
                        true,
                    )?;
                }
            }
        } else {
            // Single Scan (Monochrome or Interleaved)
            self.writer.write_start_of_scan_segment(
                frame_info.component_count,
                self.near_lossless,
                interleave_mode,
            )?;

            let _bytes_written = if frame_info.bits_per_sample <= 8 {
                self.encode_scan_typed::<u8>(source, &frame_info, pc, coding_parameters, false)?
            } else {
                let (head, body, tail) = unsafe { source.align_to::<u16>() };
                if !head.is_empty() || !tail.is_empty() {
                    return Err(JpeglsError::InvalidData);
                }
                self.encode_scan_typed::<u16>(body, &frame_info, pc, coding_parameters, false)?
            };
        }

        self.writer.write_end_of_image()?;

        Ok(self.writer.len())
    }

    fn encode_scan_typed<T: crate::jpegls::traits::JpeglsSample>(
        &mut self,
        source: &[T],
        frame_info: &FrameInfo,
        pc: JpeglsPcParameters,
        mut coding_params: CodingParameters,
        is_planar_component: bool,
    ) -> Result<usize, JpeglsError> {
        let stride = frame_info.width as usize;

        let dest_slice = self.writer.remaining_slice();

        let mut scan_frame_info = *frame_info;
        if is_planar_component {
            scan_frame_info.component_count = 1;
            coding_params.interleave_mode = InterleaveMode::None;
        }

        let mut scan_encoder = ScanEncoder::new(scan_frame_info, pc, coding_params, dest_slice);

        let bytes_written = scan_encoder.encode_scan(source, stride)?;
        drop(scan_encoder);

        self.writer.advance(bytes_written);

        Ok(bytes_written)
    }
}
