//! JPEG 1 (Baseline) Implementation (ISO/IEC 10918-1 / ITU-T T.81)
//!
//! This module implements the classic DCT-based baseline JPEG standard.
//!
//! Features:
//! - 8-bit depth support for grayscale and YCbCr images.
//! - Huffman coding with standard and custom tables.
//! - Support for Restart Markers (DRI/RSTm).
//! - Planar and Interleaved scan support.

pub mod dct;
pub mod decoder;
pub mod encoder;
pub mod huffman;
pub mod lossless;
pub mod quantization;

pub use decoder::Jpeg1Decoder;
pub use encoder::Jpeg1Encoder;
