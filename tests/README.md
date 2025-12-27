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

### Integration Tests

Run the main integration test suite:

```bash
# From project root (with venv activated)
python -m pytest tests/integration_standard_libs.py -v

# Or using unittest directly
python -m unittest tests.integration_standard_libs -v
```

### Test Coverage

The test suite includes:

- **JPEG 1 (Baseline)**: Encoding and decoding tests for grayscale and RGB images
- **JPEG-LS**:
  - Encoding tests (grayscale and RGB)
  - Decoding tests (standard library compatibility)
  - Roundtrip tests (encode → decode using jpegexp)
- **JPEG 2000**: Encoding tests (decoder not yet implemented)

### Specific Tests

You can run individual test methods:

```bash
# Test JPEG-LS RGB encoding (previously had panic issues)
python -m unittest tests.integration_standard_libs.TestCodecIntegration.test_jpegls_encode_rgb -v

# Test JPEG-LS decoding (previously had "all zeros" issue)
python -m unittest tests.integration_standard_libs.TestCodecIntegration.test_jpegls_decode_standard -v
```

## Jupyter Notebooks

The test suite includes Jupyter notebooks for interactive codec comparison and visualization.

### Running Notebooks

1. **Start Jupyter Notebook Server** (with venv activated):

```bash
# From project root
jupyter notebook tests/
```

2. **Or use JupyterLab**:

```bash
jupyter lab tests/
```

3. **Open the notebook**:
   - `codec_comparison.ipynb` - Interactive codec comparison and visualization
   - `codec_comparison_result.ipynb` - Previous test results

### Notebook Dependencies

The notebooks require:
- All dependencies from `requirements.txt`
- Built Rust binaries (see Build section above)
- Jupyter kernel with access to the virtual environment

### Creating Notebooks Programmatically

You can regenerate notebooks using:

```bash
python tests/create_notebook.py
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

For CI/CD pipelines:
1. Install Rust toolchain
2. Create .venv and install dependencies: `python -m venv .venv && .venv/bin/activate && pip install -r requirements.txt`
3. Build project: `cargo build --release`
4. Run tests: `python -m pytest tests/integration_standard_libs.py -v`
