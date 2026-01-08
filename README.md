# jpegexp-rs

`jpegexp-rs` is a high-performance, pure Rust library for encoding and decoding various JPEG standards.
It provides a unified, memory-safe interface for JPEG-LS, JPEG 1, JPEG 2000, and HTJ2K.

## 🚀 Quick Start with Codespaces

[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/rad-medica/jpegexp-rs)

This repository includes a complete DevContainer configuration for GitHub Codespaces and VS Code. Click the badge above or see [.devcontainer/README.md](.devcontainer/README.md) for details.

## Supported Standards

*   **JPEG 1 (ISO/IEC 10918-1)**: Classic baseline JPEG (DCT/Huffman). ✅ **Production Ready**
    - Grayscale: Excellent quality (MAE < 1.0)
    - RGB with YCbCr: MAE ~1.5 with quality=75 (default)
    - Quality parameter (1-100) using libjpeg scaling formula
*   **JPEG-LS (ISO/IEC 14495-1)**: Lossless and near-lossless compression. ✅ **Production Ready**
    - Grayscale 8-bit: Lossless (MAE = 0) ✅
    - Grayscale 16-bit: Lossless (MAE = 0) ✅
    - RGB/multi-component: Not yet supported (see `src/jpegls/mod.rs` for details)
*   **JPEG 2000 (ISO/IEC 15444-1)**: Wavelet-based compression. ✅ **Production Ready**
    - **Encoder**: Full lossless encoder with 100% OpenJPEG compatibility ✅
      - **RGB 8-bit**: Lossless (MAE = 0) ✅ **NEW!**
      - **Grayscale 8-bit**: Lossless (MAE = 0) ✅
      - Lossless 5-3 DWT with 0-5 decomposition levels
      - Bit-plane coding (EBCOT) with Run-Length Coding (RLC)
      - Verified up to 2048x2048 images (MAE=0 self-roundtrip)
      - Perfect interoperability with OpenJPEG 2.5.2 decoder
    - **Decoder**: Full reconstruction with IDWT (5-3/9-7) ✅
    - **Tested Sizes**: 8x8 to 2048x2048, all DWT levels ✅
    - **12-bit support**: In progress ⚠️
*   **HTJ2K (ISO/IEC 15444-15)**: High-Throughput JPEG 2000. ⚠️ **Decoder Working**
    - Decoder: CAP marker, HT block coder support
    - Encoder components implemented, integration pending

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
jpegexp-rs = "0.1.0"
```

## Usage Examples

### Decoding an Image

```rust
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::jpegls::JpeglsDecoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;

fn decode(data: &[u8]) -> Vec<u8> {
    if data.starts_with(&[0xFF, 0xD8]) {
        // JPEG 1
        let mut decoder = jpegexp_rs::jpeg1::decoder::Jpeg1Decoder::new(data);
        decoder.read_header().unwrap();
        let mut pixels = vec![0u8; (decoder.width * decoder.height * decoder.components as u32) as usize];
        decoder.decode(&mut pixels).unwrap();
        pixels
    } else if data.starts_with(&[0xFF, 0x4F]) {
        // JPEG 2000
        let mut reader = JpegStreamReader::new(data);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode().unwrap();
        image.reconstruct_pixels().unwrap()
    } else {
        // JPEG-LS
        let mut decoder = JpeglsDecoder::new(data);
        decoder.read_header().unwrap();
        let info = decoder.frame_info();
        let mut pixels = vec![0u8; (info.width * info.height * info.component_count as u32) as usize];
        decoder.decode(&mut pixels).unwrap();
        pixels
    }
}
```

### Encoding JPEG-LS

```rust
use jpegexp_rs::{FrameInfo, jpegls::JpeglsEncoder};

fn encode_jpegls(pixels: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut buffer = vec![0u8; pixels.len() * 2]; // Allocate sufficient buffer
    let mut encoder = JpeglsEncoder::new(&mut buffer);

    let info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 3, // RGB
    };
    encoder.set_frame_info(info).unwrap();

    // For RGB, the encoder uses Planar mode (3 scans) by default for compatibility.
    // Ensure your pixels are interleaved (RGBRGB...) as the encoder handles de-interleaving if needed.
    let len = encoder.encode(pixels).unwrap();

    buffer.truncate(len);
    buffer
}
```

## Python Bindings

This library includes Python bindings via `pyo3`.

```bash
cd python
maturin develop
```

```python
import jpegexp

# Decode
pixels = jpegexp.decode(jpeg_bytes)
info = jpegexp.get_info(jpeg_bytes)
print(f"Image: {info.width}x{info.height} {info.format}")

# Encode
jls_bytes = jpegexp.encode_jpegls(raw_pixels, width, height, components=3)
```

## CLI Utility

The crate provides a CLI tool `jpegexp`.

```bash
cargo run --release --bin jpegexp -- help
```

Commands:
*   `decode`: Decode a JPEG/JLS/J2K file to raw pixel data.
*   `encode`: Encode raw pixel data to JPEG/JLS/J2K.

## Development

### Quick Start
- **GitHub Codespaces**: Click the badge at the top of this README
- **Local Development**: See [DEVELOPMENT.md](DEVELOPMENT.md) for detailed setup instructions
- **DevContainer**: See [.devcontainer/README.md](.devcontainer/README.md) for configuration details

### Documentation
- [DEVELOPMENT.md](DEVELOPMENT.md) - Complete development guide
- [docs/JPEG2000_TODO.md](docs/JPEG2000_TODO.md) - JPEG 2000 implementation progress
- [docs/JPEG2000_RLC_FIX.md](docs/JPEG2000_RLC_FIX.md) - OpenJPEG interoperability fix details
- [docs/RGB_TESTING_RESULTS.md](docs/RGB_TESTING_RESULTS.md) - RGB JPEG 2000 encoding results and validation
- [CODEC_TEST_RESULTS.md](CODEC_TEST_RESULTS.md) - Detailed test results and analysis
- [SUMMARY.md](SUMMARY.md) - Project summary and findings
- [COMPLIANCE.md](COMPLIANCE.md) - Conformance testing details

## Current Status

**Production Ready**:
- ✅ JPEG 1 Grayscale (MAE < 1.0)
- ✅ JPEG 1 RGB with subsampling
- ✅ JPEG-LS Grayscale 8-bit (Lossless, MAE = 0)
- ✅ JPEG-LS Grayscale 16-bit (Lossless, MAE = 0)
- ✅ **JPEG 2000 Lossless RGB 8-bit** (MAE = 0, 100% OpenJPEG compatible) **NEW!**
- ✅ **JPEG 2000 Lossless Grayscale 8-bit** (MAE = 0, 100% OpenJPEG compatible)
  - Tested: 8x8 to 2048x2048 images
  - All DWT decomposition levels (0-5)
  - Multiple patterns: gradients, checkerboards, solid colors, inverted channels

**In Development**:
- ⚠️ JPEG-LS RGB/multi-component (sample-interleave not yet supported)
- ⚠️ JPEG 2000 12-bit support

## Test Results

### JPEG 2000 Encoder Validation

Comprehensive testing validates perfect lossless encoding for both RGB and Grayscale:

**RGB Images:**
| Image Size | DWT Levels | Pattern | Self-Roundtrip | OpenJPEG Compat | Status |
|------------|-----------|---------|----------------|-----------------|--------|
| 8x8 to 128x128 | 0-3 | All patterns | MAE=0 | MAE=0 | ✅ |
| 256x256 | 0-4 | All patterns | MAE=0 | MAE=0 | ✅ |
| 512x512 | 0-5 | All patterns | MAE=0 | MAE=0 | ✅ |
| 1024x1024 | 0-5 | Gradients, checkerboards | MAE=0 | MAE=0 | ✅ |
| 2048x2048 | 0-5 | Gradients, checkerboards | MAE=0 | MAE=0 | ✅ |

**Grayscale Images:**
| Image Size | DWT Levels | Pattern | Self-Roundtrip | OpenJPEG Compat | Status |
|------------|-----------|---------|----------------|-----------------|--------|
| 64x64 | 0-2 | All patterns | MAE=0 | MAE=0 | ✅ |
| 128x128 | 0-3 | All patterns | MAE=0 | MAE=0 | ✅ |
| 256x256 | 0-4 | All patterns | MAE=0 | MAE=0 | ✅ |
| 512x512 | 0-5 | All patterns | MAE=0 | MAE=0 | ✅ |
| 1024x1024 | 0-5 | Gradients | MAE=0 | MAE=0 | ✅ |

**OpenJPEG Interoperability** (verified with OpenJPEG 2.5.2):
- ✅ RGB: MAE=0 (jpegexp encoder → OpenJPEG decoder)
- ✅ Grayscale: MAE=0 (bidirectional)
- ✅ All test patterns: 100% compatible

See test files:
- [tests/test_openjpeg_interop_detailed.rs](tests/test_openjpeg_interop_detailed.rs) - OpenJPEG cross-validation
- [tests/test_various_sizes.rs](tests/test_various_sizes.rs) - Comprehensive size/DWT testing
- [tests/jpegls_charls_validation.rs](tests/jpegls_charls_validation.rs) - JPEG-LS validation

## Compliance

See [COMPLIANCE.md](COMPLIANCE.md) for details on conformance testing against standard reference implementations like `libjpeg-turbo`, `CharLS`, and `OpenJPEG`.
