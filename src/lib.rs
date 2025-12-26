pub mod constants;
pub mod error;

pub use coding_parameters::{CodingParameters, JpeglsPcParameters};
pub use error::JpeglsError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterleaveMode {
    None = 0,
    Line = 1,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorTransformation {
    None = 0,
    Hp1 = 1,
    Hp2 = 2,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressedDataFormat {
    Unknown = 0,
    Interchange = 1,
    AbbreviatedImageData = 2,
    AbbreviatedTableSpecification = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiffProfileId {
    None = 0,
    ContinuousToneBase = 1,
    ContinuousToneProgressive = 2,
    BiLevelFacsimile = 3,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiffColorSpace {
    BiLevelBlack = 0,
    YCbCrItuBt709Video = 1,
    None = 2,
    YCbCrItuBt6011Rgb = 3,
    YCbCrItuBt6011Video = 4,
    Grayscale = 8,
    PhotoYcc = 9,
    Rgb = 10,
    Cmy = 11,
    Cmyk = 12,
    Ycck = 13,
    CieLab = 14,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiffCompressionType {
    Uncompressed = 0,
    ModifiedHuffman = 1,
    ModifiedRead = 2,
    ModifiedModifiedRead = 3,
    Jbig = 4,
    Jpeg = 5,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiffResolutionUnits {
    AspectRatio = 0,
    DotsPerInch = 1,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameInfo {
    pub width: u32,
    pub height: u32,
    pub bits_per_sample: i32,
    pub component_count: i32,
}

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

pub mod coding_parameters;
pub mod decoder;
pub mod encoder;
pub mod golomb_lut;
pub mod jpeg_marker_code;
pub mod jpeg_stream_reader;
pub mod jpeg_stream_writer;
pub mod regular_mode_context;
pub mod run_mode_context;
pub mod scan_decoder;
pub mod scan_encoder;
pub mod traits;
pub mod validate_spiff_header;
