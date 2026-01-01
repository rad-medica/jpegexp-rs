# jpegexp-rs

`jpegexp-rs` is a high-performance, pure Rust library for encoding and decoding various JPEG standards.
It provides a unified, memory-safe interface for JPEG-LS, JPEG 1, JPEG 2000, and HTJ2K.

## 🚀 Quick Start with Codespaces

[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/rad-medica/jpegexp-rs)

This repository includes a complete DevContainer configuration for GitHub Codespaces and VS Code. Click the badge above or see [.devcontainer/README.md](.devcontainer/README.md) for details.

## Supported Standards

*   **JPEG 1 (ISO/IEC 10918-1)**: Classic baseline JPEG (DCT/Huffman). ✅ **Production Ready**
    - Grayscale: Excellent quality (MAE < 1.0)
    - RGB with chroma subsampling: Fully supported
*   **JPEG-LS (ISO/IEC 14495-1)**: Lossless and near-lossless compression. ⚠️ **In Development**
    - Current status: Partial implementation with known issues
    - Not recommended for production use
*   **JPEG 2000 (ISO/IEC 15444-1)**: Wavelet-based compression. ⚠️ **Stub Implementation**
    - Current status: Proof-of-concept only
    - Requires significant development effort
*   **HTJ2K (ISO/IEC 15444-15)**: High-Throughput JPEG 2000. ⚠️ **Not Yet Implemented**

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
- [CODEC_TEST_RESULTS.md](CODEC_TEST_RESULTS.md) - Detailed test results and analysis
- [SUMMARY.md](SUMMARY.md) - Project summary and findings
- [COMPLIANCE.md](COMPLIANCE.md) - Conformance testing details

## Current Status

**Production Ready**:
- ✅ JPEG 1 Grayscale (MAE < 1.0)
- ✅ JPEG 1 RGB with subsampling (MAE 42-55)

**In Development**:
- ⚠️ JPEG-LS (requires architectural rewrite, 2-3 weeks)
- ⚠️ JPEG 2000 (stub implementation, 4-8 weeks)

See [CODEC_TEST_RESULTS.md](CODEC_TEST_RESULTS.md) for detailed analysis.

## Compliance

See [COMPLIANCE.md](COMPLIANCE.md) for details on conformance testing against standard reference implementations like `libjpeg-turbo`, `CharLS`, and `OpenJPEG`.
