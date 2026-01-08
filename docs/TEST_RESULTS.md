# Test Results & Validation

**Date**: January 8, 2026
**Platform**: Windows x64 / Rust 1.83

This document aggregates test results from the comprehensive test suite (`tests/`).

## ✅ Compliance Testing

### 1. JPEG 2000 (Lossless)
*   **Method**: Roundtrip (Rust Encode -> OpenJPEG Decode) and (OpenJPEG Encode -> Rust Decode).
*   **Metric**: Mean Absolute Error (MAE). Target is 0.0.

| Test Case | Size | DWT | MAE | Status |
|-----------|------|-----|-----|--------|
| Grayscale Gradient | 64x64 | 5 | 0.0000 | ✅ Pass |
| Grayscale Gradient | 1024x1024 | 5 | 0.0000 | ✅ Pass |
| RGB Gradient | 256x256 | 5 | 0.0000 | ✅ Pass |
| RGB Real Photo | 512x512 | 5 | 0.0000 | ✅ Pass |
| Checkerboard (High Freq) | 512x512 | 5 | 0.0000 | ✅ Pass |

### 2. JPEG-LS
*   **Method**: Comparison against CharLS reference.

| Test Case | Bit Depth | MAE | Status |
|-----------|-----------|-----|--------|
| Grayscale 8-bit | 8 | 0.00 | ✅ Pass |
| Grayscale 16-bit | 16 | 0.00 | ✅ Pass |
| Near-Lossless (NEAR=1) | 8 | <= 1.0 | ✅ Pass |
| Near-Lossless (NEAR=3) | 8 | <= 3.0 | ✅ Pass |

### 3. DICOM Compliance (J2K)
*   **Method**: Verification against PS3.5 requirements.

| Requirement | Implementation | Status |
|-------------|----------------|--------|
| **Encapsulation** | Fragment wrapping support | ✅ Pass |
| **12-bit Depth** | 12-bit scaling/packing | ✅ Pass |
| **Signed Pixel** | Two's complement handling | ✅ Pass |
| **Monochrome1** | Inverse grayscale support | ✅ Pass |

---

## 🧪 Available Test Suites

We provide a comprehensive suite of integration tests. Run them with `cargo test --release`.

### Core Functional Tests
| Test File | Description | Coverage |
|-----------|-------------|----------|
| `tests/final_interop.rs` | Main interoperability test with OpenJPEG binaries. | Grayscale 64x64, DWT 5-3 |
| `tests/test_various_sizes.rs` | Validates encoding across range of sizes. | 64x64 - 1024x1024 |
| `tests/test_large_rgb_images.rs` | Massive RGB validation suite. | 256x256 - 2048x2048 |
| `tests/test_htj2k.rs` | HTJ2K functionality and marker validation. | Encoder Legacy Mode, Decoder |
| `tests/test_j2k_lossy.rs` | Lossy compression quality checks. | Q100, Q90, Q75, Q50 |

### Special Validation Tests
| Test File | Focus | Status |
|-----------|-------|--------|
| `tests/test_12bit_support.rs` | 12-bit depth validation. | ✅ Pass (Lossless) |
| `tests/test_16bit_support.rs` | 16-bit depth validation. | ✅ Pass (Lossless) |
| `tests/test_monochrome1.rs` | Inverse grayscale handling. | ✅ Pass |
| `tests/test_signed_pixel.rs` | Signed integer pixel data. | ✅ Pass |

### Ignored / Long-Running Tests
These tests are excluded from default runs (`cargo test`) due to execution time or external dependencies. Run with `-- --ignored`.

| Test File | Reason | Command to Run |
|-----------|--------|----------------|
| `tests/test_comprehensive_comparison.rs` | Runs 144 benchmark permutations (slow). | `cargo test --test test_comprehensive_comparison -- --ignored` |
| `tests/test_large_rgb_interop.rs` | 4K resolution processing (very slow). | `cargo test test_4k_interop -- --ignored` |
| `tests/test_htj2k.rs` | Requires external OpenHTJ2K decoder. | `cargo test test_htj2k_decoder_openjpeg_interop -- --ignored` |
| `tests/compare_with_openjpeg.rs` | Benchmark vs OpenJPEG. | `cargo test compare_with_openjpeg -- --ignored` |

---

## 🧪 Special Investigations

### RGB Lossless Fix (2026-01-08)
**Issue**: RGB images >16x16 showed corruption.
**Fix**: Increased guard bits from 2 to 3 for RGB to handle RCT range expansion.
**Verification**:
- Tested 100+ RGB cases.
- Validated sizes 8x8 to 2048x2048.
- Validated DWT levels 0-5.
- Result: **All Pass (MAE=0)**.

### RLC Interoperability Fix
**Issue**: Run-Length Coding in Cleanup pass was misinterpreting the standard regarding zero-context bits.
**Fix**: Adjusted context logic to skip zero-context for the first pixel of a run.
**Verification**:
- OpenJPEG can now decode our streams without error.
- MAE dropped from ~15.7 to 0.0.

---

## 📉 Known Failures

1.  **Large 12-bit Color J2K**:
    - **Symptom**: Artifacts in U/V channels for blocks >32x32.
    - **Cause**: Arithmetic coder desync on large signed values.
    - **Workaround**: Use smaller tile sizes.

2.  **JPEG 1 RGB Decoding**:
    - **Symptom**: Some standard JPEGs fail to decode.
    - **Status**: Low priority (use `image-rs` or `libjpeg-turbo` for standard JPEG decoding if needed; this library focuses on encoding/medical formats).
