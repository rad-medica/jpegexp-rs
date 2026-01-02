use crate::error::JpeglsError;
use crate::jpeg_stream_reader::JpegStreamReader;
use crate::jpegls::validate_spiff_header::validate_spiff_header;
use crate::FrameInfo;
use crate::jpegls::SpiffHeader;

pub struct JpeglsDecoder<'a> {
    reader: JpegStreamReader<'a>,
    spiff_header: Option<SpiffHeader>,
}

impl<'a> JpeglsDecoder<'a> {
    pub fn new(source: &'a [u8]) -> Self {
        Self {
            reader: JpegStreamReader::new(source),
            spiff_header: None,
        }
    }

    pub fn read_header(&mut self) -> Result<(), JpeglsError> {
        self.reader.read_header(&mut self.spiff_header)?;
        if let Some(spiff) = &self.spiff_header {
            let frame_info = self.frame_info();
            validate_spiff_header(spiff, &frame_info)?;
        }
        Ok(())
    }

    pub fn read_spiff_header(&mut self) -> Result<bool, JpeglsError> {
        // Logic to just read spiff header if present
        // self.reader.try_read_spiff_header... ?
        // For now, let's rely on read_header doing it all
        Ok(self.spiff_header.is_some())
    }

    pub fn frame_info(&self) -> FrameInfo {
        self.reader.frame_info()
    }

    pub fn decode(&mut self, destination: &mut [u8]) -> Result<(), JpeglsError> {
        self.reader.read_start_of_scan_segment_jpegls()?;
        let frame_info = self.frame_info();

        let preset = self.reader.preset_coding_parameters();
        let preset = if crate::jpegls::coding_parameters::is_default(&preset, &Default::default()) {
            let max_val = (1 << frame_info.bits_per_sample) - 1;
            crate::jpegls::coding_parameters::compute_default(
                max_val,
                self.reader.parameters().near_lossless,
            )
        } else {
            preset
        };

        // Build coding parameters with proper limit computation
        let mut coding_params = self.reader.parameters();
        coding_params.limit = crate::jpegls::coding_parameters::compute_limit_parameter(
            frame_info.bits_per_sample,
            coding_params.near_lossless,
            frame_info.component_count,
        );
        coding_params.quantized_bits_per_sample = frame_info.bits_per_sample;

        let mut scan_decoder = crate::jpegls::scan_decoder::ScanDecoder::new(
            frame_info,
            preset,
            coding_params,
            self.reader.remaining_data(),
        )?;

        let components = frame_info.component_count as usize;
        let width = frame_info.width as usize;
        let bytes_per_sample = if frame_info.bits_per_sample <= 8 { 1 } else { 2 };
        let stride = width * components * bytes_per_sample;

        scan_decoder.decode_scan(destination, stride)?;


        Ok(())
    }
}
