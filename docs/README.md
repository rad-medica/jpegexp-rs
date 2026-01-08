# jpegexp-rs Documentation

## Quick Links

### API References
- [CLI Reference](cli.md) - Command-line interface
- [Rust API](rust-api.md) - Native Rust library
- [Python API](python-api.md) - Python bindings
- [C API](c-api.md) - C/C++ FFI
- [WebAssembly API](wasm-api.md) - JavaScript/Browser bindings

### Implementation Status
- [JPEG 2000 Progress](JPEG2000_TODO.md) - Implementation status and TODO items
- [DICOM Compliance Summary](SESSION_SUMMARY_DICOM_COMPLIANCE.md) - Complete DICOM implementation report
- [DICOM Requirements](DICOM_J2K_REQUIREMENTS.md) - DICOM PS3.5 compliance matrix

### Performance & Validation
- [OpenJPEG Comparison](OPENJPEG_COMPARISON.md) - Performance benchmarks vs OpenJPEG 2.5.2
- [Session Summary: OpenJPEG](SESSION_SUMMARY_OPENJPEG_COMPARISON.md) - Cross-validation results
- [Comprehensive Test Report](COMPREHENSIVE_TEST_REPORT.md) - Full test coverage analysis

### Technical Details
- [JPEG2000 RLC Fix](JPEG2000_RLC_FIX.md) - Run-Length Coding interoperability fix
- [RGB Testing Results](RGB_TESTING_RESULTS.md) - RGB encoding validation

## Overview

jpegexp-rs is a universal JPEG codec library supporting:

| Format    | Standard         | Encode | Decode | Notes |
| --------- | ---------------- | ------ | ------ | ----- |
| JPEG      | ISO/IEC 10918-1  | ✓      | ✓      | Production ready |
| JPEG-LS   | ISO/IEC 14495-1  | ✓      | ✓      | Grayscale lossless (MAE=0) |
| JPEG 2000 | ISO/IEC 15444-1  | ✓      | ✓      | **Production ready for medical imaging** (MAE=0) |
| HTJ2K     | ISO/IEC 15444-15 | ✗      | ⚠️     | Decoder working, encoder pending |

### JPEG 2000 Features

**Encoder Status: ✅ Production Ready**
- Lossless compression: 8-bit, 12-bit, 16-bit (MAE=0)
- 100% OpenJPEG 2.5.2 compatibility
- DICOM PS3.5 compliant (all 5 high-priority requirements)
- Validated for medical imaging: CT, MRI, PET, SPECT, X-ray
- Test coverage: 26 DICOM tests + 37 core tests, all passing

### JPEG-LS Support Details

| Image Type | Encode | Decode | Status |
|------------|--------|--------|--------|
| Grayscale 8-bit | ✓ | ✓ | Lossless (MAE=0) |
| Grayscale 16-bit | ✓ | ✓ | Lossless (MAE=0) |
| RGB (sample interleave) | ✗ | ✗ | Not yet supported |

## Quick Start

### CLI

```bash
cargo install --path .
jpegexp decode -i image.jpg -o pixels.raw
jpegexp encode -i pixels.raw -o output.jls -w 512 -H 512 -c jpegls
jpegexp transcode -i image.jpg -o image.jls -c jpegls
```

### Rust

```rust
use jpegexp_rs::jpegls::JpeglsDecoder;

let mut decoder = JpeglsDecoder::new(&data);
decoder.read_header()?;
decoder.decode(&mut pixels)?;
```

### Python

```python
import jpegexp
pixels = jpegexp.decode(data)
info = jpegexp.get_info(data)
encoded = jpegexp.encode_jpegls(pixels, width, height, 1)
```

### JavaScript

```javascript
import init, { decode_jpeg } from "./jpegexp.js";
await init();
const pixels = decode_jpeg(data);
```

### C

```c
JpegExpDecoder* dec = jpegexp_decoder_new(data, len);
jpegexp_decoder_read_header(dec, &info);
jpegexp_decoder_decode(dec, output, output_len);
jpegexp_decoder_free(dec);
```

## Building

```bash
# Library
cargo build --release

# CLI
cargo install --path .

# WASM
wasm-pack build --target web

# Python
cd python && maturin develop

# C header
cbindgen --output jpegexp.h
```

## DICOM Compliance

**Complete DICOM PS3.5 compliance achieved** (5/5 high-priority requirements):

| Requirement | Status | Notes |
|-------------|--------|-------|
| DICOM Encapsulation (PS3.5 §8.2.4) | ✅ | Fragment wrapping, multi-frame |
| 12-bit Support | ✅ | CT, MRI, CR/DR (MAE=0) |
| 16-bit Support | ✅ | Nuclear medicine (MAE=0) |
| Signed Pixel Data (Pixel Rep = 1) | ✅ | Hounsfield Units support |
| MONOCHROME1 | ✅ | Inverse grayscale for X-ray |

**Test Results**: 26 DICOM tests passing, all with MAE=0

## Troubleshooting

### Common Issues

**JPEG-LS RGB encoding not supported:**
- JPEG-LS currently only supports grayscale images
- Use JPEG 1 for RGB images

**JPEG 2000 lossy compression quality:**
- Lossy compression works but quantization for >8-bit needs improvement
- Use lossless mode for medical imaging (default)

**Compilation errors:**
- Ensure you have Rust 1.70+ installed
- Run `rustup update` to get the latest toolchain
- For Python bindings: `pip install maturin` and `cd python && maturin develop`

### Getting Help

- **GitHub Issues**: https://github.com/rad-medica/jpegexp-rs/issues
- **Documentation**: Check [DEVELOPMENT.md](../DEVELOPMENT.md) for detailed setup
- **Tests**: Run `cargo test` to verify your installation

## Repository

https://github.com/rad-medica/jpegexp-rs

## License

MIT License - © 2024 Rad Medica
