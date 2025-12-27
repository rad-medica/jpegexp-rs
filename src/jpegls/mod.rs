//! JPEG-LS Implementation (ISO/IEC 14495-1 / ITU-T T.87)
//!
//! JPEG-LS is a low-complexity, high-performance lossless and near-lossless
//! image compression standard. It is particularly effective for medical
//! images and synthetic graphics.
//!
//! This module provides:
//! - `JpeglsEncoder`: Support for encoding images with custom LSE parameters.
//! - `JpeglsDecoder`: Capability to decode scans with multiple interleave modes (None, Line, Sample).
//! - `SPIFF`: Full support for the Still Picture Interchange File Format header.

pub mod coding_parameters;
pub mod decoder;
pub mod encoder;
pub mod golomb_lut;
pub mod regular_mode_context;
pub mod run_mode_context;
pub mod scan_decoder;
pub mod scan_encoder;
pub mod traits;
pub mod validate_spiff_header;

pub use coding_parameters::{CodingParameters, JpeglsPcParameters};
pub use decoder::JpeglsDecoder;
pub use encoder::JpeglsEncoder;

use crate::error::JpeglsError;

/// Interleave mode for multi-component scans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterleaveMode {
    /// No interleaving (non-interleaved).
    None = 0,
    /// Interleaved by line.
    Line = 1,
    /// Interleaved by sample.
    Sample = 2,
}

impl std::convert::TryFrom<u8> for InterleaveMode {
    type Error = JpeglsError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::None),
            1 => Ok(Self::Line),
            2 => Ok(Self::Sample),
            _ => Err(JpeglsError::InvalidData),
        }
    }
}

/// Color transformation for multi-component scans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorTransformation {
    /// No color transformation.
    None = 0,
    /// HP1 color transformation.
    Hp1 = 1,
    /// HP2 color transformation.
    Hp2 = 2,
    /// HP3 color transformation.
    Hp3 = 3,
}

impl std::convert::TryFrom<u8> for ColorTransformation {
    type Error = JpeglsError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::None),
            1 => Ok(Self::Hp1),
            2 => Ok(Self::Hp2),
            3 => Ok(Self::Hp3),
            _ => Err(JpeglsError::InvalidData),
        }
    }
}

/// JPEG-LS compressed data format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressedDataFormat {
    /// Unknown format.
    Unknown = 0,
    /// Interchange format.
    Interchange = 1,
    /// Abbreviated image data format.
    AbbreviatedImageData = 2,
    /// Abbreviated table specification format.
    AbbreviatedTableSpecification = 3,
}

/// SPIFF profile identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiffProfileId {
    /// No profile.
    None = 0,
    /// Continuous tone base profile.
    ContinuousToneBase = 1,
    /// Continuous tone progressive profile.
    ContinuousToneProgressive = 2,
    /// Bi-level facsimile profile.
    BiLevelFacsimile = 3,
    /// Continuous tone facsimile profile.
    ContinuousToneFacsimile = 4,
}

impl std::convert::TryFrom<u8> for SpiffProfileId {
    type Error = JpeglsError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::None),
            1 => Ok(Self::ContinuousToneBase),
            2 => Ok(Self::ContinuousToneProgressive),
            3 => Ok(Self::BiLevelFacsimile),
            4 => Ok(Self::ContinuousToneFacsimile),
            _ => Err(JpeglsError::InvalidData),
        }
    }
}

/// SPIFF color space identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiffColorSpace {
    /// Bi-level black.
    BiLevelBlack = 0,
    /// YCbCr (ITU-R BT.709) video.
    YCbCrItuBt709Video = 1,
    /// No color space (none).
    None = 2,
    /// YCbCr (ITU-R BT.601-1) RGB.
    YCbCrItuBt6011Rgb = 3,
    /// YCbCr (ITU-R BT.601-1) video.
    YCbCrItuBt6011Video = 4,
    /// Grayscale.
    Grayscale = 8,
    /// PhotoYCC.
    PhotoYcc = 9,
    /// RGB.
    Rgb = 10,
    /// CMY.
    Cmy = 11,
    /// CMYK.
    Cmyk = 12,
    /// YCCK.
    Ycck = 13,
    /// CIE Lab.
    CieLab = 14,
    /// Bi-level white.
    BiLevelWhite = 15,
}

impl std::convert::TryFrom<u8> for SpiffColorSpace {
    type Error = JpeglsError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::BiLevelBlack),
            1 => Ok(Self::YCbCrItuBt709Video),
            2 => Ok(Self::None),
            3 => Ok(Self::YCbCrItuBt6011Rgb),
            4 => Ok(Self::YCbCrItuBt6011Video),
            8 => Ok(Self::Grayscale),
            9 => Ok(Self::PhotoYcc),
            10 => Ok(Self::Rgb),
            11 => Ok(Self::Cmy),
            12 => Ok(Self::Cmyk),
            13 => Ok(Self::Ycck),
            14 => Ok(Self::CieLab),
            15 => Ok(Self::BiLevelWhite),
            _ => Err(JpeglsError::InvalidData),
        }
    }
}

/// SPIFF compression type identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiffCompressionType {
    /// Uncompressed.
    Uncompressed = 0,
    /// Modified Huffman.
    ModifiedHuffman = 1,
    /// Modified Read.
    ModifiedRead = 2,
    /// Modified Modified Read.
    ModifiedModifiedRead = 3,
    /// JBIG.
    Jbig = 4,
    /// JPEG.
    Jpeg = 5,
    /// JPEG-LS.
    JpegLs = 6,
}

impl std::convert::TryFrom<u8> for SpiffCompressionType {
    type Error = JpeglsError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Uncompressed),
            1 => Ok(Self::ModifiedHuffman),
            2 => Ok(Self::ModifiedRead),
            3 => Ok(Self::ModifiedModifiedRead),
            4 => Ok(Self::Jbig),
            5 => Ok(Self::Jpeg),
            6 => Ok(Self::JpegLs),
            _ => Err(JpeglsError::InvalidData),
        }
    }
}

/// SPIFF resolution units identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiffResolutionUnits {
    /// Aspect ratio.
    AspectRatio = 0,
    /// Dots per inch (DPI).
    DotsPerInch = 1,
    /// Dots per centimeter.
    DotsPerCentimeter = 2,
}

impl std::convert::TryFrom<u8> for SpiffResolutionUnits {
    type Error = JpeglsError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::AspectRatio),
            1 => Ok(Self::DotsPerInch),
            2 => Ok(Self::DotsPerCentimeter),
            _ => Err(JpeglsError::InvalidData),
        }
    }
}

/// Still Picture Interchange File Format (SPIFF) header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpiffHeader {
    pub profile_id: SpiffProfileId,
    pub component_count: i32,
    pub height: u32,
    pub width: u32,
    pub color_space: SpiffColorSpace,
    pub bits_per_sample: i32,
    pub compression_type: SpiffCompressionType,
    pub resolution_units: SpiffResolutionUnits,
    pub vertical_resolution: u32,
    pub horizontal_resolution: u32,
}
