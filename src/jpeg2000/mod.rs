//! JPEG 2000 Implementation (Part 1, ISO/IEC 15444-1)
//!
//! This module contains the implementation of the JPEG 2000 standard.
//! It is divided into several sub-modules handling different aspects of the codec:
//!
//! - `parser` / `writer`: Handling of the Codestream syntax (Markers, Headers).
//! - `packet`: Parsing and writing of Tile-Part packets and packet headers.
//! - `tag_tree`: Implementation of Tag Trees (used in packet headers).
//! - `image`: Data structures representing the Image, Tiles, Components, and Code-blocks.
//! - `mq_coder`: The MQ Arithmetic Coder (Tier-1 Coding).
//! - `bit_plane_coder`: Context modeling and bit-plane coding (Tier-1 Coding).
//! - `dwt`: Discrete Wavelet Transform (5-3 and 9-7).
//! - `quantization`: Scalar quantization.

pub mod bit_io;
pub mod bit_plane_coder;
pub mod decoder;
pub mod dwt;
pub mod ht_block_coder;
pub mod image;
pub mod mq_coder;
pub mod packet;
pub mod parser;
pub mod quantization;
pub mod tag_tree;
pub mod writer;
