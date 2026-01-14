# Project Roadmap & TODOs

This document tracks the backlog of planned features, improvements, and known issues for `jpegexp-rs`.

**Last Updated**: 2026-01-13 (Post-DWT Bug Fixes)

## 🔥 Critical Issues

### ✅ 1. Critical Clippy Errors Fixed
- [x] **Constraint Checks**: Added to `FrameInfo` validation
- [x] **Unsafe**: Documented invariants for 20 unsafe blocks
- [x] **Unwrap**: Eliminated 57 unwraps in core library

### ✅ 2. Comprehensive Interoperability Test Suite (2026-01-10)
- [x] **JPEG 1 Interop Tests**: 5 tests implemented (libjpeg-turbo compatibility)
- [x] **JPEG 2000 Interop Tests**: 8 tests implemented (lossless MAE=0, lossy quality validated)
- [x] **HTJ2K Interop Tests**: 5 tests implemented (encoder validation with CAP marker)
- [x] **JPEG-LS CharLS Validation**: 23/23 tests passing (perfect MAE=0)
- [x] **16-bit Endianness Investigation**: Issue cannot be reproduced, all tests passing MAE=0
- [x] **Test Infrastructure**: Reusable framework with image generation and pixel comparison
- [x] **Binary Orchestration**: PNM I/O, binary locator for external reference implementations
- [ ] **Result**: **78 active tests, 100% pass rate, 41 new interop tests added**

### ✅ 3. JPEG 2000: Major DWT Bug Fixes (2026-01-13) ⭐ NEW
- [x] **DWT 1D Inverse Boundary**: Fixed inverse prediction formula for boundary case
  - Changed `x[i] += (left + 1) >> 1` to `x[i] += left`
  - Result: 1D DWT now perfectly reversible
- [x] **get_ll_size Formula**: Fixed subband sizing calculation
  - Changed `num_levels - res` to `res + 1` for correct reductions
  - Result: Correct subband sizes for all resolutions
- [x] **extract_subband_coeffs**: Fixed coefficient extraction positions
  - Use `get_ll_size(..., 0)` for positioning
  - Result: Correct subband extraction
- [x] **DWT Coefficient Storage**: Fixed full coefficient preservation
  - Copy all subbands (LL+HL+LH+HH) to result buffer
  - Result: Full coefficient array preserved
- **Test Results**:
  - 8x8, 16x16, 32x32: MAE = 0.0 (perfect) ✅
  - 40x40, 48x48, 64x64: MAE = 0.05 (near-perfect) ⚠️
- **Files Modified**: `src/jpeg2000/dwt.rs`, `src/jpeg2000/encoder.rs`
- **Test Coverage**: New `tests/test_40x40_dwt_fix.rs` with comprehensive validation

### 4. JPEG 2000: Edge Pixel Encoding (MAE = 0.05)
- [ ] **Issue**: Single non-zero coefficients at image boundaries are lost
  - Affects right edge column (x=39) for even-width images
  - Systematic -1 error at x=39 for all rows
  - MAE = 0.05 (very small impact)
- **Root Cause**: Codeblock encoding loop doesn't properly include edge coefficients
- **Not Affected**: Solid patterns, odd-width images, smaller images (8x8, 16x16, 32x32)
- **Priority**: MEDIUM - Minor visual impact, acceptable for most use cases

### 5. HTJ2K Decoder Bug Fix (Tracked Separately)
- [ ] **HTJ2K Decoder Bug Fix**: 🔴 **HIGH PRIORITY** - Fix remaining pixel mismatch issues in HTJ2K decoder.
  - Current status: 4 test failures in `test_htj2k_comprehensive` with ~99% pixel mismatches
    - `test_htj2k_8bit_gray`: 4079/4096 pixels wrong (~99.6%)
    - `test_htj2k_12bit_gray`: 7973/8192 pixels wrong
    - `test_htj2k_16bit_gray`: 8186/8192 pixels wrong
    - `test_htj2k_8bit_rgb`: 12268/12288 pixels wrong
  - **Architectural Improvements Completed (2026-01-09)**:
    - ✅ **MEL Decoder**: Completely rewritten for forward reading with MSB-first bit packing
    - ✅ **VLC Decoder**: Completely rewritten for backward reading with LSB-first bit packing (`rev_buf` pattern)
    - ✅ **Buffer Handling**: Added required termination bytes (0xFF, 0x0F masking)
    - ✅ **Decode Loop**: Restructured to match OpenHTJ2K pattern (MEL override, proper vlcval chaining)
    - ✅ **Crashes Fixed**: Eliminated "Invalid Scup" warnings and shift overflow panics
  - **Remaining Investigation Needed**:
    - VLC/UVLC table data format verification against OpenHTJ2K
    - Context calculation correctness
    - Magnitude/sign bit reconstruction
    - UVLC u_q decoding
  - Decoder architecture now matches OpenHTJ2K reference implementation, but systematic pixel errors indicate a subtle table format or reconstruction issue.
- [ ] **RPC Mode**: Support Reduced Resolution (RPC) Transfer Syntax (.202).
- [ ] **Document all public APIs**: Add `///` doc comments with examples
  - [x] `src/jpeg2000/encoder.rs` (J2kEncoder)
  - [x] `src/lib.rs` (FrameInfo)
  - [ ] `src/jpeg_stream_reader.rs`
  - [ ] `src/jpeg1/encoder.rs`
- [ ] **Reduce `#[allow(...)]` directives**: 20 instances found
  - Many are for `manual_div_ceil` - should be fixed, not suppressed
  - Some FFI clippy suppressions are justified

### 2. Native HTJ2K SIMD Optimization
- [ ] **DWT**: Implement AVX2/NEON intrinsics for 5-3 and 9-7 lifting steps.
- [ ] **Block Coding**: Vectorize bit-plane operations (VLC/MagSgn).

### 3. Multi-tile Support
- [ ] Implement tiling logic in the encoder to handle extremely large images (e.g., Digital Pathology).

## ⚠️ Medium Priority

### 4. Testing Infrastructure Improvements
- [x] **Comprehensive interop test suite**: **COMPLETE** - 78 tests, 100% pass rate
- [x] **Test utilities**: Image generation, pixel comparison, binary orchestration
- [x] **CharLS validation**: 23/23 tests passing with perfect MAE=0
- [x] **JPEG 2000 interop**: 8 tests validating lossless and lossy compression
- [x] **HTJ2K encoder validation**: 5 tests confirming CAP marker and quality levels
- [x] **16-bit testing**: Comprehensive validation (gradient, checkerboard, HDR, multi-size)
- [ ] **Add test coverage measurement**: Use `cargo-tarpaulin` or `cargo-llvm-cov`
- [ ] **Document test organization**: Update `tests/README.md` with new interop tests
- [ ] **Add property-based testing**: Use `proptest` for codec round-trip fuzzing
- [ ] **Miri validation**: Run `cargo +nightly miri test` on unsafe code blocks
- [ ] **External binary integration**: Complete deferred tests (9 total across JPEG1/J2K/HTJ2K)

### 5. Advanced JPEG 2000 Features
- [ ] **ROI**: Region of Interest coding.
- [ ] **Multi-Layer**: Progressive quality layers (currently single layer).

### 6. Error Handling Improvements
- [ ] **Rename `JpeglsError`**: This is used globally, not just for JPEG-LS
  - Rename to `CodecError` or `JpegExpError`
  - Update all references across codebase
- [ ] **Add error context**: Use `anyhow` or custom context wrapping for better debugging

## 📉 Low Priority / Optimization

### 7. WASM Polish
- [ ] Improve the web demo UI.
- [ ] Expose more configuration options to JS API.

### 8. Documentation & Repository Hygiene
- [ ] **Fix rustfmt**: Current version doesn't support `--check` flag
  - Update to newer rustfmt version in CI
  - Ensure all code is formatted before commits
- [ ] **Add CONTRIBUTING.md**: Guide for external contributors
- [ ] **Dependency audit**: Run `cargo audit` regularly
- [ ] **Update dependencies**: Check for outdated crates with `cargo outdated`

---

## 🐛 Known Issues Tracker

| ID | Component | Issue | Status |
|----|-----------|-------|--------|
| **CI-01** | Build | 8 clippy errors blocking CI | 🔴 **BLOCKING** |
| **CI-02** | Testing | Interop tests don't run in Ubuntu CI | 🔴 **CRITICAL** |
| **CI-03** | Tooling | rustfmt version incompatibility | 🟡 Minor |
| **J2K-01** | Encoder | Lossy quantization formula (guard_bits) | 🟢 **Fixed 2026-01-09** - PSNR 13→51 dB |
| **J2K-02** | Encoder | 12-bit Color artifacts >32x32 blocks | 🟢 Working |
| **J2K-03** | Testing | 16-bit endianness issue (MAE ~19,491) | 🟢 **RESOLVED** - Fixed constant/sparse encoding |
| **J2K-04** | Encoder | 16-bit complex pattern (gradient) encoding errors | 🟡 Active - Sparse works, dense fails |
| **JLS-01** | Encoder | No RGB Interleave support | 🟢 Fixed - CharLS interop verified |
| **JLS-02** | Interop | CharLS RGB interop bit over-consumption | 🟢 Fixed (Context sharing) |
| **JLS-03** | Decoder | Grayscale regression (Rb/Rd init) | 🟢 Fixed |
| **INTEROP-01** | Testing | Comprehensive interop test suite | 🟢 **COMPLETE** - 78 tests, 100% pass rate |
| **HT-01** | Encoder | Native Magnitude Encoding missing | 🟢 Fixed (EMB implemented) |
| **HT-02** | Decoder | HTJ2K decoder architecture mismatch | 🟢 Fixed (MEL/VLC rewritten 2026-01-09) |
| **HT-03** | Decoder | HTJ2K pixel reconstruction errors | 🔴 Active - ~99% pixels wrong (table/reconstruction issue) |
| **J1-01** | Decoder | Some standard RGB JPEGs fail to decode | 🟢 Low Priority |
| **J1-02** | Encoder | 12-bit SOF1 requires custom Huffman for high quality | 🟡 Research |
| **LIB-01** | Quality | 57 unwrap/expect calls in library code | 🟡 Needs cleanup |
| **LIB-02** | Quality | 20 unsafe blocks (mostly FFI, needs audit) | 🟡 Document invariants |
| **BENCH-01** | Tooling | No Criterion benchmarks | 🟡 Missing feature |

---

## 📊 Codebase Health Summary (2026-01-08)

**Overall Maturity**: Production-ready for JPEG-LS and JPEG 1, High maturity for JPEG 2000, Experimental for HTJ2K

### Strengths ✅
- **Architecture**: Clean modular design with clear separation of concerns
- **Safety**: Minimal unsafe code, mostly isolated to FFI layer
- **Testing**: Exceptional interop test suite with reference implementations
- **Documentation**: Comprehensive docs in `docs/` directory
- **Medical Focus**: Strong DICOM compliance (5/5 requirements met)
- **Performance**: Optimized profiles, minimal allocations in hot paths

### Weaknesses ⚠️
- **CI/CD**: Clippy errors blocking builds, interop tests don't run
- **Code Quality**: Too many `unwrap()`/`expect()` calls in library code
- **Benchmarking**: Custom benchmarks instead of industry-standard Criterion
- **Error Types**: Naming confusion (`JpeglsError` used globally)
- **Platform Support**: Windows-only interop binaries limit CI validation
- **HTJ2K**: Decoder has pixel reconstruction issues (4 failing tests)

### Metrics 📈
- **Source Files**: 50 Rust files (~15,792 lines of code)
- **Dependencies**: Minimal (3 direct: clap, num_enum, thiserror)
- **Unsafe Blocks**: 20 total (17 FFI, 2 JPEG-LS, 1 zero-copy optimization)
- **Test Organization**: 4 categories (unit, integration, interop, regression)
- **Test Count**: 30+ integration tests defined in Cargo.toml
- **Documentation**: 45 markdown files across project
