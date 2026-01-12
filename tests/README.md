# Test Suite Documentation

This directory contains the test suite for `jpegexp-rs`, including integration tests and Jupyter notebooks for codec comparison.

## Prerequisites

1. **Python 3.8+** installed on your system
2. **Rust toolchain** (for building the project)
3. **Virtual environment** (venv) for Python dependencies

## Setup

### 1. Create and Activate Virtual Environment

**Windows (PowerShell):**
```powershell
# Create venv in project root
python -m venv .venv

# Activate venv
.\.venv\Scripts\Activate.ps1
```

**Linux/macOS:**
```bash
# Create venv in project root
python3 -m venv .venv

# Activate venv
source .venv/bin/activate
```

### 2. Install Python Dependencies

```bash
# From project root
pip install -r requirements.txt
```

### 3. Build the Rust Project

Before running tests, you must build the Rust project:

```bash
# Build in release mode (recommended for testing)
cargo build --release

# Or build in debug mode (faster compilation, slower execution)
cargo build
```

The tests will automatically detect the binary at:
- `target/release/jpegexp.exe` (Windows) or `target/release/jpegexp` (Linux/macOS)
- Falls back to `target/debug/` if release build is not found

## Running Tests

### Comprehensive Interoperability Tests (NEW - 2026-01-11)

Run the comprehensive codec interoperability test suite:

```bash
# Quick test (8-bit lossless JPEG-LS only)
cargo test --release quick_jpegls_interop -- --nocapture

# Full comprehensive suite (all codecs, ~2 minutes)
cargo test --release run_all_comprehensive_interop -- --nocapture --ignored

# Individual codec families
cargo test --release comprehensive_jpegls_interop -- --nocapture --ignored
cargo test --release comprehensive_j2k_interop -- --nocapture --ignored
cargo test --release comprehensive_jpeg1_interop -- --nocapture --ignored
```

**Test Results:**
- Generates CSV files in `docs/test-results/`
- Creates detailed comparison report: `docs/test-results/INTEROP_REPORT.md`
- See [Interop Report](../docs/test-results/INTEROP_REPORT.md) for 573-line analysis

### Other Rust Tests

```bash
# All tests
cargo test --release

# Specific test categories
cargo test --release --test jpegls_charls_validation  # JPEG-LS validation
cargo test --release unit                      # Unit tests
cargo test --release integration               # Integration tests

# Run a specific test
cargo test --release --test jpegls_charls_validation -- --nocapture
```

### Test Coverage (Updated 2026-01-11)

**Comprehensive Interoperability Tests:**
- **JPEG 1**: 320 tests (100% pass) - libjpeg-turbo 3.1.3 validation
- **JPEG 2000**: 300 tests (43% pass) - OpenJPEG 2.5.2 validation (solid patterns work, gradients fail)
- **JPEG-LS**: 640 tests (15% pass) - CharLS 3.0.0 validation (8-bit perfect, CLI limitations)
- **Total**: 1,260 interoperability tests with detailed metrics

**Individual Test Suites:**
- **JPEG 1 (Baseline)**: SOF0/SOF1/SOF2 encoding and decoding for grayscale and RGB
- **JPEG-LS**: CharLS validation (`tests/interop/jpegls_charls_validation.rs`): 23/23 passing
  - Lossless grayscale 8-bit: MAE = 0
  - Lossless grayscale 16-bit: MAE = 0
  - Lossless RGB sample-interleaved: MAE = 0
  - Edge cases (1x1, 1x8, 8x1, large images)
- **JPEG 2000**: OpenJPEG compatibility tests (solid patterns: MAE=0, complex patterns: issues)
- **HTJ2K**: Encoder validation (CAP marker compliance)

### Interop Tests Requirements

Interop tests require external binaries in `libs/bin/`:
- `opj_decompress.exe` (OpenJPEG 2.5.2)
- `opj_compress.exe` (OpenJPEG 2.5.2)
- `charls.exe` (CharLS 3.0.0)
- `cjpeg.exe`, `djpeg.exe` (libjpeg-turbo 3.1.3)
- `oj_compress.exe`, `oj_decompress.exe` (OpenHTJ2K 0.6.0)

Run interop tests:
```bash
cargo test --release --test final_interop -- --nocapture
```

## Jupyter Notebooks

**Note**: Python-based notebooks are currently deprecated. All testing is done via Rust tests.

The `notebooks/` directory contains legacy Jupyter notebooks for reference:
- Historical codec comparison visualizations
- Previous test results

For current test results, use:
```bash
cargo test --release -- --nocapture
```

## Troubleshooting

### Import Errors

If you see import errors:
1. Ensure venv is activated: `which python` should point to .venv
2. Reinstall dependencies: `pip install -r requirements.txt --force-reinstall`

### Binary Not Found

If tests fail with "binary not found":
1. Build the project: `cargo build --release`
2. Check that `target/release/jpegexp.exe` (or `jpegexp` on Linux/macOS) exists
3. On Windows, ensure you're using the `.exe` extension

### DLL/Shared Library Errors

On Windows, if you see DLL errors:
- Ensure `target/release/jpegexp_rs.dll` exists
- The DLL should be in the same directory as the executable or in PATH

### Jupyter Kernel Issues

If Jupyter can't find your venv:
1. Install ipykernel in .venv: `pip install ipykernel`
2. Register .venv as kernel: `python -m ipykernel install --user --name=jpegexp-rs --display-name "Python (jpegexp-rs)"`
3. Select the kernel in Jupyter: Kernel → Change Kernel → Python (jpegexp-rs)

## Test Output

Tests create temporary files in a system temp directory. The path is printed at the end of test execution for debugging purposes.

Test output includes:
- File sizes of encoded images
- PSNR (Peak Signal-to-Noise Ratio) for lossy codecs
- Pixel mismatch information for lossless codecs
- Detailed error messages for failed assertions

## Continuous Integration

The CI pipeline (`.github/workflows/ci.yml`) runs:
1. Rust toolchain setup
2. Build: `cargo build --release`
3. Tests: `cargo test --lib` (36/36 passing)
4. Linting: `cargo clippy --all-targets --all-features`
5. Formatting: `cargo fmt --check`

**Note**: Interop tests are not run in CI (require Windows binaries, CI runs on Ubuntu).
