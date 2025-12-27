# jpegexp-rs Documentation

## Quick Links

- [CLI Reference](cli.md) - Command-line interface
- [Rust API](rust-api.md) - Native Rust library
- [Python API](python-api.md) - Python bindings
- [C API](c-api.md) - C/C++ FFI
- [WebAssembly API](wasm-api.md) - JavaScript/Browser bindings

## Overview

jpegexp-rs is a universal JPEG codec library supporting:

| Format    | Standard         | Encode | Decode |
| --------- | ---------------- | ------ | ------ |
| JPEG      | ISO/IEC 10918-1  | ✓      | ✓      |
| JPEG-LS   | ISO/IEC 14495-1  | ✓      | ✓      |
| JPEG 2000 | ISO/IEC 15444-1  | ✗      | ✓      |
| HTJ2K     | ISO/IEC 15444-15 | ✗      | ✓      |

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

## Repository

https://github.com/rad-medica/jpegexp-rs

## License

MIT License - © 2024 Rad Medica
