/*!
# jpegexp-rs

`jpegexp-rs` is a high-performance, pure Rust library for encoding and decoding various JPEG standards.
It aims to provide a unified, memory-safe interface for JPEG-LS, JPEG 1, JPEG 2000, and HTJ2K.

## Supported Standards

### JPEG-LS (ISO/IEC 14495-1)
- **Status**: Completed (Ported from CharLS)
- **Features**: Lossless and Near-Lossless compression, 8-16 bit depth, custom presets.
- **Modules**: `jpegls`

### JPEG 1 (ISO/IEC 10918-1)
- **Status**: Completed (Baseline)
- **Features**: DCT-based lossy compression, Huffman coding.
- **Modules**: `jpeg1`

### JPEG 2000 (ISO/IEC 15444-1)
- **Status**: In Progress (Partially Implemented)
- **Features**:
    - **Tier-1**: MQ Coder, Bitplane Coding (Context Modeling).
    - **Tier-2**: Tag Tree coding, Packet Header parsing/writing.
    - **Codestream**: Reading/Writing of Main and Tile headers (SOC, SIZ, COD, QCD, SOT, EOC).
    - **DWT**: 5-3 and 9-7 Filter Banks (Logic implemented, integration pending).
- **Modules**: `jpeg2000`

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

#[cfg(test)]
mod tests {
    use crate::jpeg_stream_reader::JpegStreamReader;
    use crate::jpeg2000::decoder::J2kDecoder;

    #[test]
    fn smoke_test() {
        assert!(true);
    }

    #[test]
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
            // COD
            0xFF, 0x52, 0x00, 0x0A, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x01, 0x01,
            // QCD
            0xFF, 0x5C, 0x00, 0x05, 0x06, 0x00, 0x10, // SOT
            0xFF, 0x90, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
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
