# TESTS KNOWLEDGE BASE

**Generated:** 2026-01-15  
**Context:** Multi-tier testing infrastructure (Unit -> Integration -> Interop -> Debug)

## OVERVIEW
Comprehensive testing ecosystem for cross-validating 4 JPEG standards against reference implementations with 1260+ interop tests.

## STRUCTURE
```
tests/
├── interop/      # Gold-standard validation vs OpenJPEG, CharLS, libjpeg-turbo
├── integration/  # End-to-end roundtrip tests for all codecs and bit depths
├── debug/        # 51+ granular diagnostic tools (MQ coder trace, DWT inspection)
├── common/       # Shared utilities (Synthetic Image Generator)
├── analysis/     # Bitstream and pixel distribution analysis tools
├── dwt/          # Wavelet transform correctness and boundary tests
├── mq/           # MQ-coder symbol-by-symbol comparison tests
├── regression/   # Targeted bug reproduction and regression tracking
├── fixtures/     # Reference bitstreams (.j2k, .jls) and ground truth (.pgm)
└── scripts/      # Python helper scripts for batch generation/validation
```

## WHERE TO LOOK

| Component | Location | Notes |
|-----------|----------|-------|
| Synthetic Generator | `tests/common/synthetic_images.rs` | Patterns (Solid, Gradient, Noise) for 8-16 bit |
| Interop Suite | `tests/interop/comprehensive_interop.rs` | Main entry point for 1260+ cross-validation tests |
| Universal Test | `tests/integration/universal_codec_test.rs` | Generic wrapper for any supported codec |
| External Binaries | `libs/bin/` | Windows .exe for OpenJPEG, CharLS, libjpeg-turbo |
| Test Results | `docs/test-results/` | Auto-generated CSVs and INTEROP_REPORT.md |

## CONVENTIONS

### Testing Strategy
- **Reference-Driven Validation**: Never self-validate. Always compare Rust encoder vs Reference decoder AND Reference encoder vs Rust decoder.
- **MAE=0 Goal**: Lossless modes MUST achieve zero Mean Absolute Error vs reference implementations.
- **Synthetic Coverage**: Every feature must be tested against 8/10/12/16-bit synthetic patterns (Solid, Gradient, Checkerboard, Noise, MedicalCT).
- **Environment Isolation**: Interop tests are marked `#[ignore]` by default; run with `--ignored` to enable external binary dependency.

### Test Organization
- **Granular Targets**: 50+ `[[test]]` entries in `Cargo.toml` allow running specific diagnostic tools without full suite overhead.
- **Diagnostic Focus**: Use `tests/debug/` tools for byte-by-byte bitstream comparison when interop fails.
- **Medical Validation**: First-class testing for signed pixels and MONOCHROME1/2 interpretation.

## NOTES
- **CI Status**: Library unit tests (36/36) run in CI; Interop tests require Windows environment and manual execution.
- **Known J2K Gaps**: Complex patterns at >8-bit are currently failing in J2K (see `INTEROP_REPORT.md`).
- **Data Cleanup**: Tests use system temp directories; paths are printed on failure for manual inspection.
