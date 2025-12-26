use crate::error::JpeglsError;
use crate::{
    FrameInfo, SpiffColorSpace, SpiffCompressionType, SpiffHeader, SpiffProfileId,
    SpiffResolutionUnits,
};

fn is_valid_color_space(color_space: SpiffColorSpace, component_count: i32) -> bool {
    match color_space {
        SpiffColorSpace::None => true,
        SpiffColorSpace::BiLevelBlack | SpiffColorSpace::BiLevelWhite => false, // Not supported for JPEG-LS
        SpiffColorSpace::Grayscale => component_count == 1,
        SpiffColorSpace::YCbCrItuBt709Video
        | SpiffColorSpace::YCbCrItuBt6011Rgb
        | SpiffColorSpace::YCbCrItuBt6011Video
        | SpiffColorSpace::Rgb
        | SpiffColorSpace::Cmy
        | SpiffColorSpace::PhotoYcc
        | SpiffColorSpace::CieLab => component_count == 3,
        SpiffColorSpace::Cmyk | SpiffColorSpace::Ycck => component_count == 4,
    }
}

fn is_valid_resolution_units(resolution_units: SpiffResolutionUnits) -> bool {
    match resolution_units {
        SpiffResolutionUnits::AspectRatio
        | SpiffResolutionUnits::DotsPerCentimeter
        | SpiffResolutionUnits::DotsPerInch => true,
    }
}

pub fn validate_spiff_header(
    spiff_header: &SpiffHeader,
    frame_info: &FrameInfo,
) -> Result<(), JpeglsError> {
    if spiff_header.compression_type != SpiffCompressionType::JpegLs {
        return Err(JpeglsError::InvalidSpiffHeader);
    }

    if spiff_header.profile_id != SpiffProfileId::None {
        return Err(JpeglsError::InvalidSpiffHeader);
    }

    if !is_valid_resolution_units(spiff_header.resolution_units) {
        return Err(JpeglsError::InvalidSpiffHeader);
    }

    if spiff_header.horizontal_resolution == 0 || spiff_header.vertical_resolution == 0 {
        return Err(JpeglsError::InvalidSpiffHeader);
    }

    if spiff_header.component_count != frame_info.component_count {
        return Err(JpeglsError::InvalidSpiffHeader);
    }

    if !is_valid_color_space(spiff_header.color_space, spiff_header.component_count) {
        return Err(JpeglsError::InvalidSpiffHeader);
    }

    if spiff_header.bits_per_sample != frame_info.bits_per_sample {
        return Err(JpeglsError::InvalidSpiffHeader);
    }

    if spiff_header.height != frame_info.height {
        return Err(JpeglsError::InvalidSpiffHeader);
    }

    if spiff_header.width != frame_info.width {
        return Err(JpeglsError::InvalidSpiffHeader);
    }

    Ok(())
}
