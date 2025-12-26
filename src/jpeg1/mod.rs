// JPEG 1 (Baseline) implementation.
pub mod dct;
pub mod quantization;
pub mod huffman;
pub mod encoder;
pub mod decoder;

pub use encoder::Jpeg1Encoder;
pub use decoder::Jpeg1Decoder;
