# Interoperability Test Results

This directory contains the results of comprehensive interoperability testing between `jpegexp-rs` and reference codec implementations.

## Reference Codecs

| Codec Family | Reference Implementation | Version | Location |
|--------------|-------------------------|---------|----------|
| JPEG-LS | CharLS | 3.0.0 | `libs/bin/charls.exe` |
| JPEG 1 | libjpeg-turbo | 3.1.3 | `libs/bin/cjpeg.exe`, `djpeg.exe` |
| JPEG 2000 | OpenJPEG | 2.5.2 | `libs/bin/opj_compress.exe`, `opj_decompress.exe` |
| HTJ2K | OpenHTJ2K | latest | `libs/bin/open_htj2k_enc.exe`, `open_htj2k_dec.exe` |

## Test Philosophy

**CRITICAL RULE**: Never test a codec against itself.

- **Encoding tests**: Our encoder -> Reference decoder
- **Decoding tests**: Reference encoder -> Our decoder

This ensures true interoperability validation.

## Running Tests

### Quick Tests (CI-friendly)

```bash
cargo test --release quick_jpegls_interop -- --nocapture
```

### Comprehensive Tests

```bash
# All codecs
cargo test --release run_all_comprehensive_interop -- --nocapture --ignored

# Individual codec families
cargo test --release comprehensive_jpegls_interop -- --nocapture --ignored
cargo test --release comprehensive_j2k_interop -- --nocapture --ignored
cargo test --release comprehensive_jpeg1_interop -- --nocapture --ignored
```

### Benchmarks

```bash
cargo bench --bench interop_benchmarks
```

## Output Files

Test runs generate timestamped result files:

- `jpegls_interop_<timestamp>.csv` - JPEG-LS results
- `jpeg1_interop_<timestamp>.csv` - JPEG 1 results
- `j2k_interop_<timestamp>.csv` - JPEG 2000 results
- `htj2k_interop_<timestamp>.csv` - HTJ2K results

### CSV Schema

```
Codec,Direction,Mode,Width,Height,BitDepth,Components,Pattern,QualityParam,
EncTime_us,DecTime_us,OriginalSize,CompressedSize,CompressionRatio,MAE,MaxError,PSNR,Throughput_MBps,Status
```

## Metrics

### Correctness Metrics

| Metric | Target (Lossless) | Target (Lossy) |
|--------|-------------------|----------------|
| MAE (Mean Absolute Error) | 0.0 | < 5.0 |
| Max Error | 0 | < 50 |
| PSNR | Infinity | > 30 dB |

### Performance Metrics

- **Encode Time**: Time to compress (microseconds)
- **Decode Time**: Time to decompress (microseconds)
- **Throughput**: MB/s for roundtrip
- **Compression Ratio**: Original size / Compressed size

## Test Matrix

### Synthetic Images

| Pattern | Description |
|---------|-------------|
| solid | Uniform value |
| gradient_h | Horizontal gradient |
| gradient_v | Vertical gradient |
| gradient_d | Diagonal gradient |
| checkerboard | Alternating blocks |
| noise | Pseudo-random noise |
| medical_ct | CT-like edges |
| natural | Gradient with subtle noise |

### Bit Depths

- 8-bit (all codecs)
- 10-bit (JPEG-LS, J2K)
- 12-bit (JPEG-LS, J2K, JPEG 1 Extended)
- 16-bit (JPEG-LS, J2K)

### Quality Modes

| Codec | Lossless | Near-Lossless | Lossy |
|-------|----------|---------------|-------|
| JPEG-LS | NEAR=0 | NEAR=1,2,5,10 | - |
| JPEG 1 | SOF3 | - | Q=50,75,90,95 |
| J2K | 5-3 DWT | - | 9-7 DWT |
| HTJ2K | - | - | - |

## Latest Results

### 📊 Comprehensive Interoperability Report

**[View Full Report: INTEROP_REPORT.md](./INTEROP_REPORT.md)**

A 573-line comprehensive analysis with detailed comparison tables covering:
- Executive summary with pass/fail rates
- MAE comparison tables by bit depth, pattern, and resolution
- Compression ratio analysis
- Speed benchmarks
- Detailed failure analysis for each codec family

### Quick Summary (2026-01-11)

| Codec | Tests Run | Passed | Pass Rate | Status |
|-------|-----------|--------|-----------|--------|
| **JPEG 1** | 320 | 320 | **100%** | ✅ Production Ready |
| **JPEG 2000** | 300 | 128 | **43%** | ⚠️ Needs Work |
| **JPEG-LS** | 640 | 98 | **15%** | ⚠️ Limited (CharLS CLI issues) |

### Data Files

- `jpeg1_interop_1768182324.csv` — 320 JPEG 1 tests
- `j2k_interop_1768182293.csv` — 300 JPEG 2000 tests
- `jpegls_interop_1768182576.csv` — 640 JPEG-LS tests
