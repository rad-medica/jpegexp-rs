/*!
# jpegexp-rs

`jpegexp-rs` is a high-performance, pure Rust library for encoding and decoding various JPEG standards.
It aims to provide a unified, memory-safe interface for JPEG-LS, JPEG 1, JPEG 2000, and HTJ2K.
*/

pub mod constants;
pub mod error;
pub mod jpeg_marker_code;
pub mod jpeg_stream_reader;
pub mod jpeg_stream_writer;

pub mod jpegls;
pub mod jpeg1;

pub use error::JpeglsError;

/// Basic information about a JPEG image frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameInfo {
    /// Width of the frame in pixels.
    pub width: u32,
    /// Height of the frame in pixels.
    pub height: u32,
    /// Bits per sample (2-16).
    pub bits_per_sample: i32,
    /// Number of components (e.g., 1 for grayscale, 3 for RGB).
    pub component_count: i32,
}
