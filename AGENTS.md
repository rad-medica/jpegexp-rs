# PROJECT KNOWLEDGE BASE

**Generated:** 2026-01-15 18:26:00  
**Commit:** 3e9327f  
**Branch:** master

## OVERVIEW
Pure Rust universal JPEG codec library supporting JPEG 1 (DCT/Huffman), JPEG-LS (LOCO-I), JPEG 2000 (Wavelet/MQ), and HTJ2K. Focus on medical imaging (DICOM), high bit-depths (12/16-bit), and perfect interoperability (MAE=0).

## STRUCTURE
```
jpegexp-rs/
├── src/
│   ├── jpeg1/           # JPEG 1 Baseline/Progressive/Lossless (DCT, Huffman)
│   ├── jpeg2000/        # JPEG 2000/HTJ2K (DWT, MQ-coder, Tag Trees)
│   ├── jpegls/          # JPEG-LS ISO 14495-1 (LOCO-I, Golomb)
│   ├── dicom/           # DICOM encapsulation (PS3.5)
│   └── bin/jpegexp.rs   # CLI tool
├── tests/
│   ├── interop/         # Cross-validation vs OpenJPEG/CharLS/libjpeg-turbo
│   ├── integration/     # High-level roundtrip tests
│   └── debug/           # 51 diagnostic tools for codec internals
├── libs/                # Bundled reference implementations (C/C++) + Windows binaries
├── docs/                # Standard compliance, test results, architecture
└── python/              # PyO3 bindings (separate crate)
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add JPEG 2000 feature | `src/jpeg2000/encoder.rs` or `decoder.rs` | 1447/1114 lines - consider splitting |
| Fix HTJ2K bitstream | `src/jpeg2000/ht_block_coder/` | MEL/VLC/Mag-Sgn specialized codecs |
| Add JPEG-LS mode | `src/jpegls/scan_encoder.rs` or `scan_decoder.rs` | LOCO-I state machine |
| Fix JPEG 1 DCT | `src/jpeg1/dct.rs` | Manual FDCT/IDCT implementations |
| Stream parsing | `src/jpeg_stream_reader.rs` | Unified marker reader for all standards |
| Stream writing | `src/jpeg_stream_writer.rs` | Unified marker writer |
| Add new marker | `src/jpeg_marker_code.rs` | Central marker code definitions |
| Errors | `src/error.rs` | `JpeglsError` used across all codecs (rename pending) |
| FFI/C bindings | `src/ffi.rs` | 798 lines - needs splitting by standard |
| Python bindings | `python/src/lib.rs` | PyO3 wrapper |
| WASM | `src/wasm.rs` | wasm-bindgen exports |
| Interop tests | `tests/interop/comprehensive_interop.rs` | 1,260+ cross-validation tests |
| Test images | `tests/common/synthetic_images.rs` | Pattern generator (gradients, noise, checkerboard) |

## CONVENTIONS

### Architecture
- **Reference-Driven Development**: "Never test a codec against itself" - all validation uses external reference implementations
- **MAE=0 Goal**: Lossless modes must achieve zero Mean Absolute Error vs references
- **Unified Abstraction**: `FrameInfo` struct (width, height, bits, components) shared across all codecs
- **Marker State Machine**: All formats use `JpegStreamReader`/`JpegStreamWriter` for parsing/writing

### Code Patterns
- **Bit Depth Polymorphism**: `JpeglsSample` trait abstracts `u8`/`u16` processing - compiler optimizes per type
- **Medical Focus**: First-class support for signed pixels, 12/16-bit, MONOCHROME1
- **Pure Rust**: Only 20 `unsafe` blocks (mostly FFI), 13 `dyn`, 17 traits - static dispatch preferred
- **Explicit Test Targets**: 50+ `[[test]]` entries in `Cargo.toml` for granular control (non-standard)

### Testing
- **Interop Binaries**: `libs/bin/` contains Windows `.exe`/`.dll` for OpenJPEG, CharLS, libjpeg-turbo
- **Synthetic Images**: Deterministic patterns at 8/10/12/16-bit depths for edge-case validation
- **Two-Way Validation**: Rust encoder → Reference decoder AND Reference encoder → Rust decoder
- **Strict MAE**: Lossless must be `MAE = 0.0`, lossy uses PSNR thresholds

## ANTI-PATTERNS (THIS PROJECT)

### Critical Rules
- **NEVER test codec against itself** - always cross-validate with reference implementations
- **NEVER suppress type errors** - no `as any`, `@ts-ignore` equivalent in Rust (use proper types)
- **NEVER commit without MAE=0** for lossless modes - validates perfect interoperability
- **DO NOT use JPEG 2000 with decomposition_levels >= 2** for images >= 128 pixels until multi-level DWT bug is fixed

### Documentation Requirements
- **ALWAYS update documentation BEFORE commit** if code or test results change
- **ALWAYS update affected AGENTS.md** when changing rules, conventions, or module behavior
- **NEVER create duplicate documentation** - update existing files (e.g., append to INTEROP_REPORT.md, not create new reports)
- **NEVER delete unrelated information** - only modify sections directly affected by code changes
- **AVOID documentation bloat** - don't add redundant TODOs if existing ones cover the same issue

### Known Issues
- **J2K Large Images**: Images >128×128 have small MAE errors (0.0001-0.6) when using multiple code-blocks per subband - suspected buffer stride issue in code-block reconstruction
- **HTJ2K Decoder**: Experimental/broken (4 tests failing with pixel reconstruction errors)
- **Technical Debt**: 57 `unwrap()` calls remaining, 20 `unsafe` blocks need invariant docs
- **Naming**: `JpeglsError` used globally but should be renamed to `CodecError`

### Refactoring Targets
- `src/jpeg1/encoder.rs` (2468 lines): **3x duplication** for `u8`/`u16`/planar - needs trait abstraction
- `src/jpeg2000/encoder.rs` (1447 lines): **Monolith** - split packetization logic
- `src/jpeg2000/ht_block_coder/vlc_ohtj2k.rs` (2332 lines): **Data bloat** - move tables to separate file
- `src/ffi.rs` (798 lines): **Dumping ground** - split by standard (jpeg1.rs, jpegls.rs, etc.)

## COMMANDS

```bash
# Build
cargo build --release

# Run all lib tests (36/36 passing)
cargo test --lib

# Run specific integration test
cargo test --release --test j2k_roundtrip_test

# Run full interop suite (requires Windows binaries in libs/bin/)
cargo test --release --test comprehensive_interop run_all_comprehensive_interop -- --ignored --nocapture

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt

# Python bindings (requires maturin)
cd python && maturin develop

# CLI tool
cargo run --bin jpegexp -- --help
```

## NOTES

### Current Status (as of 2026-01-16)
- **JPEG 1**: ✅ Production ready (320/320 interop tests passing)
- **JPEG-LS Decoder**: ✅ Production ready (100% validation vs CharLS)
- **JPEG-LS Encoder**: ⚠️ 61.3% interop (98/160) - 10/12-bit has CharLS CLI compatibility issues
- **JPEG 2000**: ⚠️ 36% interop (107/300) - bit-plane coder fixed, multi-level DWT bug identified
- **HTJ2K Encoder**: ⚠️ Experimental
- **HTJ2K Decoder**: ❌ Broken

### Recent Fixes
- **2026-01-16**: **CRITICAL FIX** - Bit-plane coder now ISO 15444-1 compliant
  - Fixed SigProp pass: Now delays neighbor flag updates until pass completion (Section C.4.1.2)
  - Fixed MagRef pass: Same deferred update pattern applied
  - Applied to both encoder AND decoder for symmetric operation
  - Files modified: `src/jpeg2000/bit_plane_coder.rs` (lines 261-304, 491-541)
- **2026-01-16**: Fixed multi-level DWT grid calculation bug (encoder.rs:854-872) - 128×128 with 2-5 decomposition levels now achieves MAE=0.0 (previously MAE=0.06-29.61). Changed subband size calculation to use parent LL (res-1) instead of deepest LL (res=0), matching decoder logic.
- **2026-01-15**: Added bit depth masking to J2K encoder - fixed solid pattern MAE from ~250-10000 to 0.0

### Dependencies
- `num_enum`: Type-safe marker enums
- `thiserror`: Error handling
- `clap`: CLI parsing
- `criterion`: Benchmarks
- `pyo3`: Python bindings
- `wasm-bindgen`: WASM support

### External References
- See `docs/ARCHITECTURE.md` for data flow diagrams
- See `docs/status.md` for implementation maturity levels
- See `docs/test-results/INTEROP_REPORT.md` for detailed test analysis
- See `docs/TODO.md` for technical debt tracker
