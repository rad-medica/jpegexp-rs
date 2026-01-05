# jpegexp-rs Documentation

## Quick Links

- [CLI Reference](cli.md) - Command-line interface
- [Rust API](rust-api.md) - Native Rust library
- [Python API](python-api.md) - Python bindings
- [C API](c-api.md) - C/C++ FFI
- [WebAssembly API](wasm-api.md) - JavaScript/Browser bindings
- [JPEG 2000 Progress](JPEG2000_TODO.md) - Current implementation status

## Overview

jpegexp-rs is a universal JPEG codec library supporting:

| Format    | Standard         | Encode | Decode | Notes |
| --------- | ---------------- | ------ | ------ | ----- |
| JPEG      | ISO/IEC 10918-1  | ✓      | ✓      | Production ready |
| JPEG-LS   | ISO/IEC 14495-1  | ✓      | ✓      | Grayscale lossless (MAE=0) |
| JPEG 2000 | ISO/IEC 15444-1  | ⚠️     | ✓      | Encoder: DWT done, packets in progress |
| HTJ2K     | ISO/IEC 15444-15 | ✗      | ⚠️     | Decoder working, encoder pending |

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

## Troubleshooting

### Common Issues

**JPEG 2000 encoding produces large files or fails roundtrip testing:**
- The encoder currently produces empty packets, resulting in constant-value reconstruction
- This is expected behavior during development - packet encoding is in progress
- Use JPEG 1 or JPEG-LS for production compression

**JPEG-LS RGB encoding not supported:**
- JPEG-LS currently only supports grayscale images
- Use JPEG 1 for RGB images

**High MAE values in testing:**
- Check which codec you're testing - JPEG 1 should have MAE < 1.0
- JPEG 2000 encoder currently has MAE ~108 due to empty packets

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
