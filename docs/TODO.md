# Project Roadmap & TODOs

This document tracks the backlog of planned features, improvements, and known issues for `jpegexp-rs`.

**Last Updated**: 2026-01-09 (Post-HTJ2K Structural Fixes)

## 🔥 Critical Issues (**FIXED** - CI Ready)

### ✅ 1. Critical Clippy Errors Fixed
- [x] **Constraint Checks**: Added to `FrameInfo` validation
- [x] **Unsafe**: Documented invariants for 20 unsafe blocks
- [x] **Unwrap**: Eliminated 57 unwraps in core library

- [ ] **HTJ2K Decoder Bug Fix**: 🔴 **HIGH PRIORITY** - Fix pixel mismatch issues in HTJ2K decoder (VLC/UVLC/EMB reconstruction).
  - Current status: 4 test failures in `test_htj2k_comprehensive` (8/12/16-bit gray, 8-bit RGB)
  - Verified: Roundtrip for single coefficients passes (`test_ht_coder_repro.rs`), proving core logic is sound.
  - Remaining issues: Integration tests show bitstream desynchronization (`0 -> 255` sign flip). This suggests `MEL/VLC` decoding is slightly off for complex patterns or block boundaries.
  - Fixes Applied: 
    - Corrected VLC tables.
    - Implemented proper Bit Stuffing in Mel/MagSgn streams.
    - Fixed Context Model to check all neighbors.
    - Synced Decoder `quad_exponents` state with Encoder.
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
- [ ] **Add test coverage measurement**: Use `cargo-tarpaulin` or `cargo-llvm-cov`
- [ ] **Document test organization**: Update `tests/README.md` (references non-existent `integration_standard_libs.py`)
- [ ] **Add property-based testing**: Use `proptest` for codec round-trip fuzzing
- [ ] **Miri validation**: Run `cargo +nightly miri test` on unsafe code blocks

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
| **J2K-01** | Encoder | Lossy quantization quality mismatch | 🟢 Fixed |
| **J2K-02** | Encoder | 12-bit Color artifacts >32x32 blocks | 🟢 Working |
| **JLS-01** | Encoder | No RGB Interleave support | 🟢 Fixed - CharLS interop verified |
| **JLS-02** | Interop | CharLS RGB interop bit over-consumption | 🟢 Fixed (Context sharing) |
| **JLS-03** | Decoder | Grayscale regression (Rb/Rd init) | 🟢 Fixed |
| **HT-01** | Encoder | Native Magnitude Encoding missing | 🟢 Fixed (EMB implemented) |
| **HT-02** | Decoder | HTJ2K pixel mismatch in reconstruction | 🔴 Active - 4 tests failing |
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
