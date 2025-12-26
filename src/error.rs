use thiserror::Error;

#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum JpeglsError {
    #[error("Not enough memory")]
    NotEnoughMemory = 1,
    #[error("Callback failed")]
    CallbackFailed = 2,
    #[error("Destination too small")]
    DestinationTooSmall = 3,
    #[error("Need more data")]
    NeedMoreData = 4,
    #[error("Invalid data")]
    InvalidData = 5,
    #[error("Encoding not supported")]
    EncodingNotSupported = 6,
    #[error("Parameter value not supported")]
    ParameterValueNotSupported = 7,
    #[error("Color transform not supported")]
    ColorTransformNotSupported = 8,
    #[error("JPEG-LS preset extended parameter type not supported")]
    JpeglsPresetExtendedParameterTypeNotSupported = 9,
    #[error("JPEG marker start byte not found")]
    JpegMarkerStartByteNotFound = 10,
    #[error("Start of image marker not found")]
    StartOfImageMarkerNotFound = 11,
    #[error("Invalid SPIFF header")]
    InvalidSpiffHeader = 12,
    #[error("Unknown JPEG marker found")]
    UnknownJpegMarkerFound = 13,
    #[error("Unexpected start of scan marker")]
    UnexpectedStartOfScanMarker = 14,
    #[error("Invalid marker segment size")]
    InvalidMarkerSegmentSize = 15,
    #[error("Duplicate start of image marker")]
    DuplicateStartOfImageMarker = 16,
    #[error("Duplicate start of frame marker")]
    DuplicateStartOfFrameMarker = 17,
    #[error("Duplicate component ID in SOF segment")]
    DuplicateComponentIdInSofSegment = 18,
    #[error("Unexpected end of image marker")]
    UnexpectedEndOfImageMarker = 19,
    #[error("Invalid JPEG-LS preset parameter type")]
    InvalidJpeglsPresetParameterType = 20,
    #[error("Missing end of SPIFF directory")]
    MissingEndOfSpiffDirectory = 21,
    #[error("Unexpected restart marker")]
    UnexpectedRestartMarker = 22,
    #[error("Restart marker not found")]
    RestartMarkerNotFound = 23,
    #[error("End of image marker not found")]
    EndOfImageMarkerNotFound = 24,
    #[error("Unexpected define number of lines marker")]
    UnexpectedDefineNumberOfLinesMarker = 25,
    #[error("Define number of lines marker not found")]
    DefineNumberOfLinesMarkerNotFound = 26,
    #[error("Unknown component ID")]
    UnknownComponentId = 27,
    #[error("Abbreviated format and SPIFF header mismatch")]
    AbbreviatedFormatAndSpiffHeaderMismatch = 28,
    #[error("Invalid parameter width")]
    InvalidParameterWidth = 29,
    #[error("Invalid parameter height")]
    InvalidParameterHeight = 30,
    #[error("Invalid parameter bits per sample")]
    InvalidParameterBitsPerSample = 31,
    #[error("Invalid parameter component count")]
    InvalidParameterComponentCount = 32,
    #[error("Invalid parameter interleave mode")]
    InvalidParameterInterleaveMode = 33,
    #[error("Invalid parameter near lossless")]
    InvalidParameterNearLossless = 34,
    #[error("Invalid parameter JPEG-LS preset parameters")]
    InvalidParameterJpeglsPresetParameters = 35,
    #[error("Invalid parameter color transformation")]
    InvalidParameterColorTransformation = 36,
    #[error("Invalid parameter mapping table ID")]
    InvalidParameterMappingTableId = 37,
    #[error("Invalid parameter mapping table continuation")]
    InvalidParameterMappingTableContinuation = 38,

    // Logic errors
    #[error("Invalid operation")]
    InvalidOperation = 100,
    #[error("Invalid argument")]
    InvalidArgument = 101,
    #[error("Invalid argument width")]
    InvalidArgumentWidth = 102,
    #[error("Invalid argument height")]
    InvalidArgumentHeight = 103,
    #[error("Invalid argument bits per sample")]
    InvalidArgumentBitsPerSample = 104,
    #[error("Invalid argument component count")]
    InvalidArgumentComponentCount = 105,
    #[error("Invalid argument interleave mode")]
    InvalidArgumentInterleaveMode = 106,
    #[error("Invalid argument near lossless")]
    InvalidArgumentNearLossless = 107,
    #[error("Invalid argument JPEG-LS PC parameters")]
    InvalidArgumentJpeglsPcParameters = 108,
    #[error("Invalid argument color transformation")]
    InvalidArgumentColorTransformation = 109,
    #[error("Invalid argument size")]
    InvalidArgumentSize = 110,
    #[error("Invalid argument stride")]
    InvalidArgumentStride = 111,
    #[error("Invalid argument encoding options")]
    InvalidArgumentEncodingOptions = 112,
}
