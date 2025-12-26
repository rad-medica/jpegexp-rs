use crate::error::JpeglsError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum JpegMarkerCode {
    /// SOI: Marks the start of an image.
    StartOfImage = 0xD8,

    /// EOI: Marks the end of an image.
    EndOfImage = 0xD9,

    /// SOS: Marks the start of scan.
    StartOfScan = 0xDA,

    /// DNL: Defines the number of lines in a scan.
    DefineNumberOfLines = 0xDC,

    /// DRI: Defines the restart interval used in succeeding scans.
    DefineRestartInterval = 0xDD,

    /// APP0: Application data 0: used for JFIF header.
    ApplicationData0 = 0xE0,
    /// APP1: Application data 1: used for EXIF or XMP header.
    ApplicationData1 = 0xE1,
    /// APP2: Application data 2: used for ICC profile.
    ApplicationData2 = 0xE2,
    /// APP3: Application data 3: used for meta info
    ApplicationData3 = 0xE3,
    /// APP4: Application data 4.
    ApplicationData4 = 0xE4,
    /// APP5: Application data 5.
    ApplicationData5 = 0xE5,
    /// APP6: Application data 6.
    ApplicationData6 = 0xE6,
    /// APP7: Application data 7: used for HP color-space info.
    ApplicationData7 = 0xE7,
    /// APP8: Application data 8: used for HP color-transformation info or SPIFF header.
    ApplicationData8 = 0xE8,
    /// APP9: Application data 9.
    ApplicationData9 = 0xE9,
    /// APP10: Application data 10.
    ApplicationData10 = 0xEA,
    /// APP11: Application data 11.
    ApplicationData11 = 0xEB,
    /// APP12: Application data 12: used for Picture info.
    ApplicationData12 = 0xEC,
    /// APP13: Application data 13: used by PhotoShop IRB
    ApplicationData13 = 0xED,
    /// APP14: Application data 14: used by Adobe
    ApplicationData14 = 0xEE,
    /// APP15: Application data 15.
    ApplicationData15 = 0xEF,

    /// COM: Comment block.
    Comment = 0xFE,

    // The following markers are defined in ISO/IEC 14495-1 | ITU T.87. (JPEG-LS standard)
    /// SOF_55: Marks the start of a JPEG-LS encoded frame.
    StartOfFrameJpegls = 0xF7,

    /// LSE: Marks the start of a JPEG-LS preset parameters segment.
    JpeglsPresetParameters = 0xF8,
}

impl std::convert::TryFrom<u8> for JpegMarkerCode {
    type Error = JpeglsError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0xD8 => Ok(Self::StartOfImage),
            0xD9 => Ok(Self::EndOfImage),
            0xDA => Ok(Self::StartOfScan),
            0xDC => Ok(Self::DefineNumberOfLines),
            0xDD => Ok(Self::DefineRestartInterval),
            0xE0 => Ok(Self::ApplicationData0),
            0xE1 => Ok(Self::ApplicationData1),
            0xE2 => Ok(Self::ApplicationData2),
            0xE3 => Ok(Self::ApplicationData3),
            0xE4 => Ok(Self::ApplicationData4),
            0xE5 => Ok(Self::ApplicationData5),
            0xE6 => Ok(Self::ApplicationData6),
            0xE7 => Ok(Self::ApplicationData7),
            0xE8 => Ok(Self::ApplicationData8),
            0xE9 => Ok(Self::ApplicationData9),
            0xEA => Ok(Self::ApplicationData10),
            0xEB => Ok(Self::ApplicationData11),
            0xEC => Ok(Self::ApplicationData12),
            0xED => Ok(Self::ApplicationData13),
            0xEE => Ok(Self::ApplicationData14),
            0xEF => Ok(Self::ApplicationData15),
            0xFE => Ok(Self::Comment),
            0xF7 => Ok(Self::StartOfFrameJpegls),
            0xF8 => Ok(Self::JpeglsPresetParameters),
            _ => Err(JpeglsError::InvalidData),
        }
    }
}

pub const JPEG_MARKER_START_BYTE: u8 = 0xFF;
pub const JPEG_RESTART_MARKER_BASE: u8 = 0xD0;
pub const JPEG_RESTART_MARKER_RANGE: u8 = 8;
