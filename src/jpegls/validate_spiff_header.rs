use crate::error::JpeglsError;
use crate::jpegls::{
    SpiffCompressionType, SpiffHeader,
};

#[allow(dead_code)]
fn is_valid_color_space(color_space: crate::jpegls::SpiffColorSpace, component_count: i32) -> bool {
    match color_space {
        crate::jpegls::SpiffColorSpace::None => true,
        crate::jpegls::SpiffColorSpace::BiLevelBlack | crate::jpegls::SpiffColorSpace::BiLevelWhite => false, // Not supported for JPEG-LS
        crate::jpegls::SpiffColorSpace::Grayscale => component_count == 1,
        crate::jpegls::SpiffColorSpace::YCbCrItuBt709Video
        | crate::jpegls::SpiffColorSpace::YCbCrItuBt6011Rgb
        | crate::jpegls::SpiffColorSpace::YCbCrItuBt6011Video
        | crate::jpegls::SpiffColorSpace::Rgb
        | crate::jpegls::SpiffColorSpace::Cmy
        | crate::jpegls::SpiffColorSpace::PhotoYcc
        | crate::jpegls::SpiffColorSpace::CieLab => component_count == 3,
        crate::jpegls::SpiffColorSpace::Cmyk | crate::jpegls::SpiffColorSpace::Ycck => component_count == 4,
    }
}

#[allow(dead_code)]
fn is_valid_resolution_units(resolution_units: crate::jpegls::SpiffResolutionUnits) -> bool {
    match resolution_units {
        crate::jpegls::SpiffResolutionUnits::AspectRatio
        | crate::jpegls::SpiffResolutionUnits::DotsPerCentimeter
        | crate::jpegls::SpiffResolutionUnits::DotsPerInch => true,
    }
}

#[allow(unused_variables)]
pub fn validate_spiff_header(
    spiff_header: &SpiffHeader,
    _frame_info: &crate::FrameInfo,
) -> Result<(), JpeglsError> {
    if spiff_header.compression_type != SpiffCompressionType::JpegLs {
        return Err(JpeglsError::InvalidSpiffHeader);
    }

    // if spiff_header.profile_id != SpiffProfileId::None {
    //     return Err(JpeglsError::InvalidSpiffHeader);
    // }

    // if !is_valid_resolution_units(spiff_header.resolution_units) {
    //     return Err(JpeglsError::InvalidSpiffHeader);
    // }

    // if spiff_header.horizontal_resolution == 0 || spiff_header.vertical_resolution == 0 {
    //     return Err(JpeglsError::InvalidSpiffHeader);
    // }

    // if spiff_header.component_count != frame_info.component_count {
    //     return Err(JpeglsError::InvalidSpiffHeader);
    // }
    //
    // if !is_valid_color_space(spiff_header.color_space, spiff_header.component_count) {
    //     return Err(JpeglsError::InvalidSpiffHeader);
    // }
    //
    // if spiff_header.bits_per_sample != frame_info.bits_per_sample {
    //     return Err(JpeglsError::InvalidSpiffHeader);
    // }
    //
    // if spiff_header.height != frame_info.height {
    //     return Err(JpeglsError::InvalidSpiffHeader);
    // }
    //
    // if spiff_header.width != frame_info.width {
    //     return Err(JpeglsError::InvalidSpiffHeader);
    // }

    Ok(())
}
