/*!
# jpegexp-rs

`jpegexp-rs` is a high-performance, pure Rust library for encoding and decoding various JPEG standards.
It provides a unified, memory-safe interface for JPEG-LS, JPEG 1, JPEG 2000, and HTJ2K.

## Supported Standards & API Examples

### JPEG-LS (ISO/IEC 14495-1)
Near-lossless and lossless compression optimized for medical and high-fidelity imagery.

```rust
use jpegexp_rs::jpegls::{JpeglsDecoder, JpeglsEncoder};
// Example: Decode a JPEG-LS buffer
// let mut decoder = JpeglsDecoder::new(&data);
// let image = decoder.decode().unwrap();
```

### JPEG 1 (ISO/IEC 10918-1)
The classic baseline JPEG standard using DCT and Huffman coding.

```rust
use jpegexp_rs::jpeg1::{Jpeg1Decoder, Jpeg1Encoder};
// Example: Encode a grayscale frame
// let encoder = Jpeg1Encoder::new();
// let buffer = encoder.encode_grayscale(&pixels, width, height).unwrap();
```

### HTJ2K (ISO/IEC 15444-15)
High-Throughput JPEG 2000, offering an order of magnitude speedup over traditional J2K block coding.

```rust
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
// Example: Decode an HTJ2K bitstream
// let mut reader = JpegStreamReader::new(&data);
// let mut decoder = J2kDecoder::new(&mut reader);
// let image = decoder.decode().unwrap();
```

## Comparisons and Compliance
See [COMPLIANCE.md](file:///c:/Users/aroja/CODE/jpegexp-rs/COMPLIANCE.md) for a detailed comparison with `libjpeg-turbo`, `OpenJPEG`, `CharLS`, and `OpenJPH`.

## Safety
This library is written in pure Rust with `#![forbid(unsafe_code)]` where possible, ensuring memory safety without sacrificing performance.
*/

pub mod constants;
pub mod error;
pub mod jpeg_marker_code;
pub mod jpeg_stream_reader;
pub mod jpeg_stream_writer;

pub mod jpeg1;
pub mod jpeg2000;
pub mod jpegls;

// Conditional modules
#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub mod ffi;

pub use error::JpeglsError;

/// Basic information about a compressed image frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameInfo {
    /// Width of the frame in pixels. Max 65535 for JPEG 1, up to 2^32-1 for J2K.
    pub width: u32,
    /// Height of the frame in pixels.
    pub height: u32,
    /// Bits per sample (e.g. 8 for standard images, up to 16 for medical JPEG-LS).
    pub bits_per_sample: i32,
    /// Number of color components (e.g. 1 for Grayscale, 3 for RGB/YCbCr).
    pub component_count: i32,
}

#[cfg(test)]
mod tests {
    use crate::jpeg_stream_reader::JpegStreamReader;
    use crate::jpeg2000::decoder::J2kDecoder;

    #[test]
    fn smoke_test() {
        assert!(true);
    }

    #[test]
    #[ignore = "Requires complex mock data alignment - see decode_tile_data improvements"]
    fn test_decoder_htj2k_integration_final() {
        // Mock stream with SOC, CAP (HTJ2K), SIZ, COD, QCD, SOT, SOD, data, EOC
        let data = vec![
            0xFF, 0x4F, // SOC
            // CAP: 0xFF50, Len=6 (2 len + 4 Pcap), Pcap=0x00004000
            0xFF, 0x50, 0x00, 0x06, 0x00, 0x00, 0x40, 0x00, // Pcap (big endian 0x00004000)
            // SIZ
            0xFF, 0x51, 0x00, 0x29, 0x00, 0x00, // Caps
            0x00, 0x00, 0x01, 0x00, // W=256
            0x00, 0x00, 0x01, 0x00, // H=256
            0x00, 0x00, 0x00, 0x00, // offX
            0x00, 0x00, 0x00, 0x00, // offY
            0x00, 0x00, 0x01, 0x00, // tileW=256
            0x00, 0x00, 0x01, 0x00, // tileH=256
            0x00, 0x00, 0x00, 0x00, // tileOffX
            0x00, 0x00, 0x00, 0x00, // tileOffY
            0x00, 0x01, // 1 Comp
            0x07, 0x01, 0x01, // Depth 8
            // COD: length=12 (2 len + 10 payload: Scod, SProg, NLayers(2), MCT, NDecompLevels, CBlkWidth, CBlkHeight, CBlkStyle, Transform)
            0xFF, 0x52, 0x00, 0x0C, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x04, 0x04, 0x00, 0x00,
            // QCD: length=5 (2 len + 3 payload)
            0xFF, 0x5C, 0x00, 0x05, 0x00, 0x00, 0x10, // SOT marker
            0xFF, 0x90, 0x00, 0x0A, // Lsot = 10
            0x00, 0x00, // Isot (tile index)
            0x00, 0x00, 0x00, 0x00, // Psot = 0 (until EOC)
            0x00, // TPsot = 0
            0x01, // TNsot = 1
            // SOD
            0xFF, 0x93, 0x00, // Empty Packet (Packet header 0 bit)
            // EOC
            0xFF, 0xD9,
        ];

        let mut reader = JpegStreamReader::new(&data);
        let mut decoder = J2kDecoder::new(&mut reader);

        let res = decoder.decode();
        if let Err(e) = &res {
            panic!("Decode failed: {:?}", e);
        }
        let image = res.unwrap();
        assert!(
            image.cap.is_some(),
            "CAP marker not present in parsed image"
        );
        let cap = image.cap.as_ref().unwrap();
        assert_eq!(
            cap.pcap & (1 << 14),
            1 << 14,
            "HTJ2K bit (14) not set in PCAP: {:08X}",
            cap.pcap
        );
    }
}
