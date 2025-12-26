pub const DEFAULT_RESET_THRESHOLD: i32 = 64; // Default RESET value as defined in ISO/IEC 14495-1, table C.2

pub const MINIMUM_COMPONENT_COUNT: i32 = 1;
pub const MAXIMUM_COMPONENT_COUNT: i32 = 255;
pub const MAXIMUM_COMPONENT_COUNT_IN_SCAN: i32 = 4;
pub const MINIMUM_COMPONENT_INDEX: i32 = 0;
pub const MAXIMUM_COMPONENT_INDEX: i32 = MAXIMUM_COMPONENT_COUNT - 1;
pub const MINIMUM_BITS_PER_SAMPLE: i32 = 2;
pub const MAXIMUM_BITS_PER_SAMPLE: i32 = 16;
pub const MAXIMUM_NEAR_LOSSLESS: i32 = 255;
pub const MINIMUM_APPLICATION_DATA_ID: i32 = 0;
pub const MAXIMUM_APPLICATION_DATA_ID: i32 = 15;

// The following limits for mapping tables are defined in ISO/IEC 14495-1, C.2.4.1.2, table C.4.
pub const MINIMUM_MAPPING_TABLE_ID: i32 = 1;
pub const MAXIMUM_MAPPING_TABLE_ID: i32 = 255;
pub const MINIMUM_MAPPING_ENTRY_SIZE: i32 = 1;
pub const MAXIMUM_MAPPING_ENTRY_SIZE: i32 = 255;

pub const MAX_K_VALUE: i32 = 16; // This is an implementation limit (theoretical limit is 32)

// ISO/IEC 14495-1, section 4.8.1 defines the SPIFF version numbers to be used for the SPIFF header in combination with
// JPEG-LS.
pub const SPIFF_MAJOR_REVISION_NUMBER: u8 = 2;
pub const SPIFF_MINOR_REVISION_NUMBER: u8 = 0;

pub const SPIFF_END_OF_DIRECTORY_ENTRY_TYPE: u8 = 1;

// The size of a SPIFF header when serialized to a JPEG byte stream.
pub const SPIFF_HEADER_SIZE_IN_BYTES: usize = 34;

// The maximum size of the data bytes that fit in a spiff entry.
pub const SPIFF_ENTRY_MAX_DATA_SIZE: usize = 65528;

// The special value to indicate that the stride should be calculated.
// In C++ this is 0, but in Rust we might want to be explicit or use Option.
// Keeping it as 0 for now to match C++ logic where 0 means "auto".
pub const AUTO_CALCULATE_STRIDE: usize = 0;

// The size in bytes of the segment length field.
pub const SEGMENT_LENGTH_SIZE: usize = 2;

// The maximum size of the data bytes that fit in a segment.
pub const SEGMENT_MAX_DATA_SIZE: usize = u16::MAX as usize - SEGMENT_LENGTH_SIZE;

pub const J: [i32; 32] = [
    0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 9, 10, 11, 12, 13, 14, 15
];

