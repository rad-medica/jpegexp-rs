# jpegexp-rs

**High-Performance Pure Rust JPEG Codec Library**

[![CI](https://github.com/rad-medica/jpegexp-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/rad-medica/jpegexp-rs/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

`jpegexp-rs` is a universal JPEG library providing unified, memory-safe interfaces for **JPEG 1**, **JPEG-LS**, **JPEG 2000**, and **HTJ2K**. It is designed for medical imaging (DICOM), archival, and high-performance applications.

---

## 📚 Documentation

| Topic | Description |
|-------|-------------|
| [**Project Status**](docs/STATUS.md) | Current readiness, roadmap, and known issues. |
| [**Comparisons**](docs/COMPARISON.md) | Benchmarks vs OpenJPEG/CharLS. Feature matrix. |
| [**Test Results**](docs/TEST_RESULTS.md) | Validation data, interoperability reports. |
| [**Compliance**](docs/compliance/jpeg_standard.md) | Standard compliance details. |
| [**DICOM**](docs/compliance/dicom.md) | Specific medical imaging requirements. |

### API Reference
*   [**CLI**](docs/api/cli.md) - Command Line Interface
*   [**Rust**](docs/api/rust.md) - Native Crate
*   [**Python**](docs/api/python.md) - Bindings
*   [**C / C++**](docs/api/c.md) - FFI
*   [**WASM**](docs/api/wasm.md) - WebAssembly

---

## 🌟 Key Features

*   **JPEG 2000 Lossless**: 100% OpenJPEG compatible, medical-grade accuracy (MAE=0).
*   **JPEG-LS**: Extremely fast lossless compression for 8-bit and 16-bit grayscale.
*   **HTJ2K**: High-Throughput JPEG 2000 support (Legacy Mode + Decoder).
*   **Pure Rust**: Memory safe, no segfaults, easy cross-compilation.
*   **Medical Focus**: Native support for 12-bit/16-bit depth and DICOM-compliant bitstreams.

---

## 🚀 Quick Start

### Installation (CLI)

```bash
cargo install --path .
```

### Basic Usage

**Decode an image:**
```bash
jpegexp decode -i medical.j2k -o raw_pixels.bin
```

**Encode an image (JPEG-LS):**
```bash
jpegexp encode -i pixels.bin -o output.jls -w 512 -H 512 -c jpegls
```

**Transcode (JPEG to J2K):**
```bash
jpegexp transcode -i scan.jpg -o archive.j2k -c jpeg2000
```

---

## 🛠️ Development

See [DEVELOPMENT.md](DEVELOPMENT.md) for setup instructions, building from source, and running the test suite.

## 📄 License

This project is licensed under the MIT License.
