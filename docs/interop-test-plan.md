# Comprehensive Interoperability Test Plan

## Overview

This document describes the comprehensive interoperability test suite for `jpegexp-rs`. The test suite validates codec implementations against reference implementations to ensure correctness, measure performance, and document compatibility.

## Test Philosophy

**CRITICAL RULE**: Never test a codec against itself. All tests follow the cross-validation principle:
- When testing **encoding capability**: Our encoder -> Reference decoder
- When testing **decoding capability**: Reference encoder -> Our decoder

This ensures true interoperability validation rather than just internal consistency.

---

## Reference Codecs

| Codec Family | Reference Implementation | Binary Location | Version |
|--------------|-------------------------|-----------------|---------|
| JPEG-LS | CharLS | `libs/bin/charls.exe` | 3.0.0 |
| JPEG 1 | libjpeg-turbo | `libs/bin/cjpeg.exe`, `djpeg.exe` | 3.1.3 |
| JPEG 2000 | OpenJPEG | `libs/bin/opj_compress.exe`, `opj_decompress.exe` | 2.5.2 |
| HTJ2K | OpenHTJ2K | `libs/bin/open_htj2k_enc.exe`, `open_htj2k_dec.exe` | latest |

---

## Synthetic Test Images

### Image Patterns

| Pattern | Description | Purpose |
|---------|-------------|---------|
| `solid` | Uniform color/gray value | Baseline compression efficiency |
| `gradient_h` | Horizontal gradient | Test prediction accuracy |
| `gradient_v` | Vertical gradient | Test row-based prediction |
| `gradient_d` | Diagonal gradient | Test 2D prediction |
| `checkerboard` | Alternating pattern | Worst-case for prediction |
| `noise` | Random values | Stress test for entropy coding |
| `medical_ct` | CT-like patterns (high contrast edges) | Medical imaging simulation |
| `natural` | Natural image-like gradients with noise | Real-world approximation |

### Resolutions

| Category | Dimensions | Aspect Ratio |
|----------|------------|--------------|
| Tiny | 8x8, 16x16 | 1:1 |
| Small | 32x32, 64x64 | 1:1 |
| Medium | 128x128, 256x256 | 1:1 |
| Large | 512x512, 1024x1024 | 1:1 |
| Wide | 256x64, 512x128 | 4:1 |
| Tall | 64x256, 128x512 | 1:4 |
| Odd | 127x129, 255x257 | Non-power-of-2 |

### Bit Depths

| Bit Depth | Max Value | Primary Use Cases |
|-----------|-----------|-------------------|
| 8-bit | 255 | Standard images, web |
| 10-bit | 1023 | Video, HDR preview |
| 12-bit | 4095 | Medical CT/MR, professional |
| 16-bit | 65535 | Scientific, raw sensor data |

### Color Modes

| Mode | Components | Description |
|------|------------|-------------|
| Grayscale | 1 | Single-channel intensity |
| RGB | 3 | Sample-interleaved color |

---

## Test Matrix

### JPEG-LS Tests

| Test Category | Encoder | Decoder | Bit Depths | Patterns |
|---------------|---------|---------|------------|----------|
| Encode Validation | jpegexp-rs | CharLS | 8, 10, 12, 16 | All |
| Decode Validation | CharLS | jpegexp-rs | 8, 10, 12, 16 | All |
| Near-Lossless | jpegexp-rs | CharLS | 8, 16 | gradient, noise |
| Near-Lossless | CharLS | jpegexp-rs | 8, 16 | gradient, noise |

**Near-Lossless Parameters**: NEAR = 0 (lossless), 1, 2, 3, 5, 10

### JPEG 1 Tests

| Test Category | Encoder | Decoder | Bit Depths | Patterns |
|---------------|---------|---------|------------|----------|
| Baseline Encode | jpegexp-rs | libjpeg-turbo | 8 | All |
| Baseline Decode | libjpeg-turbo | jpegexp-rs | 8 | All |
| Extended 12-bit | jpegexp-rs | libjpeg-turbo | 12 | All |
| Lossless (SOF3) | jpegexp-rs | libjpeg-turbo | 8, 12, 16 | All |
| Progressive | jpegexp-rs | libjpeg-turbo | 8 | All |

**Quality Levels**: 50, 75, 90, 95, 100 (lossless where applicable)

### JPEG 2000 Tests

| Test Category | Encoder | Decoder | Bit Depths | Patterns |
|---------------|---------|---------|------------|----------|
| Lossless Encode | jpegexp-rs | OpenJPEG | 8, 10, 12, 16 | All |
| Lossless Decode | OpenJPEG | jpegexp-rs | 8, 10, 12, 16 | All |
| Lossy Encode | jpegexp-rs | OpenJPEG | 8, 16 | All |
| Lossy Decode | OpenJPEG | jpegexp-rs | 8, 16 | All |

**Compression Ratios**: 1 (lossless), 2:1, 5:1, 10:1, 20:1

### HTJ2K Tests

| Test Category | Encoder | Decoder | Bit Depths | Patterns |
|---------------|---------|---------|------------|----------|
| HT Lossless Encode | jpegexp-rs | OpenHTJ2K | 8, 12, 16 | All |
| HT Lossless Decode | OpenHTJ2K | jpegexp-rs | 8, 12, 16 | All |

---

## Metrics Collected

### Correctness Metrics

| Metric | Formula | Lossless Target | Lossy Notes |
|--------|---------|-----------------|-------------|
| MAE (Mean Absolute Error) | Σ\|orig - recon\| / N | 0.0 | Lower is better |
| Max Error | max(\|orig - recon\|) | 0 | Should be ≤ NEAR for near-lossless |
| PSNR | 20·log₁₀(MAX/RMSE) | ∞ | Higher is better |
| Mismatch Count | Σ(orig ≠ recon) | 0 | Should be 0 for lossless |

### Performance Metrics

| Metric | Unit | Description |
|--------|------|-------------|
| Encode Time | ms | Time to compress |
| Decode Time | ms | Time to decompress |
| Throughput | MB/s | Megapixels per second |
| Compressed Size | bytes | Output file size |
| Compression Ratio | X:1 | Original / Compressed |

### Comparison Metrics

For each test, we compare:
1. **jpegexp-rs Encode + Reference Decode** vs original
2. **Reference Encode + jpegexp-rs Decode** vs original
3. **jpegexp-rs Decode** vs **Reference Decode** (for same encoded file)

---

## Output Format

### CSV Files

All test results are saved as CSV files in `docs/test-results/`:

```
docs/test-results/
├── jpegls_interop_YYYYMMDD_HHMMSS.csv
├── jpeg1_interop_YYYYMMDD_HHMMSS.csv
├── j2k_interop_YYYYMMDD_HHMMSS.csv
├── htj2k_interop_YYYYMMDD_HHMMSS.csv
└── summary_YYYYMMDD_HHMMSS.csv
```

### CSV Schema

```csv
Timestamp,Codec,Direction,Mode,Width,Height,BitDepth,Components,Pattern,Quality,EncTime_us,DecTime_us,FileSize,MAE,MaxError,PSNR,Status
```

### Markdown Report

A comprehensive Markdown report is generated at `docs/test-results/REPORT.md` containing:

1. Executive Summary
2. Test Configuration
3. Per-Codec Results Tables
4. Performance Charts (ASCII)
5. Failure Analysis
6. Recommendations

---

## Running Tests

### Full Test Suite

```bash
cargo test --release comprehensive_interop -- --nocapture --ignored
```

### Individual Codec Tests

```bash
cargo test --release jpegls_comprehensive -- --nocapture --ignored
cargo test --release jpeg1_comprehensive -- --nocapture --ignored
cargo test --release j2k_comprehensive -- --nocapture --ignored
cargo test --release htj2k_comprehensive -- --nocapture --ignored
```

### Benchmarks

```bash
cargo bench --bench interop_benchmarks
```

---

## Success Criteria

### Lossless Codecs (JPEG-LS, J2K Lossless, JPEG 1 Lossless)

- **MAE = 0.0** for all lossless configurations
- **Max Error = 0** for all lossless configurations
- All tests must PASS (no decode failures)

### Near-Lossless (JPEG-LS with NEAR > 0)

- **Max Error ≤ NEAR parameter**
- **MAE ≤ NEAR / 2** (typical)

### Lossy Codecs (JPEG 1 Baseline, J2K Lossy)

- **Decode success** for all configurations
- **PSNR > 30 dB** for quality ≥ 75
- **PSNR > 40 dB** for quality ≥ 95

---

## Appendix: Pattern Generation Algorithms

### Gradient Generation

```rust
fn gradient_horizontal(x: u32, y: u32, w: u32, h: u32, max: u64) -> u64 {
    x as u64 * max / (w - 1) as u64
}

fn gradient_vertical(x: u32, y: u32, w: u32, h: u32, max: u64) -> u64 {
    y as u64 * max / (h - 1) as u64
}

fn gradient_diagonal(x: u32, y: u32, w: u32, h: u32, max: u64) -> u64 {
    ((x + y) as u64 * max) / ((w + h - 2) as u64)
}
```

### Checkerboard Generation

```rust
fn checkerboard(x: u32, y: u32, block_size: u32, max: u64) -> u64 {
    let bx = (x / block_size) % 2;
    let by = (y / block_size) % 2;
    if bx ^ by == 0 { 0 } else { max }
}
```

### Noise Generation (Deterministic)

```rust
fn noise(x: u32, y: u32, w: u32, seed: u32, max: u64) -> u64 {
    let idx = y * w + x;
    // LCG for reproducibility
    let val = (idx as u64 * 1103515245 + 12345 + seed as u64) % (max + 1);
    val
}
```

---

## Version History

| Date | Version | Changes |
|------|---------|---------|
| 2025-01-11 | 1.0.0 | Initial comprehensive test plan |
