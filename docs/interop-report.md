# Master Interoperability Report

This report summarizes the cross-interoperability tests between `jpegexp-rs` native Rust codecs and industry-standard external libraries.

## Test Methodology
- **Rust -> External**: Image is encoded using `jpegexp-rs` and decoded using the reference external library.
- **External -> Rust**: Image is encoded using the reference external library and decoded using `jpegexp-rs`.
- **Metrics**:
    - **Speed**: Encoding and Decoding time in milliseconds.
    - **Accuracy**: Mean Absolute Error (MAE). Lossless = 0.0000.
    - **Compression**: Output file size in bytes.

## Reference Libraries
- **JPEG 2000**: OpenJPEG 2.5.2 (`opj_compress`, `opj_decompress`)
- **JPEG-LS**: CharLS 3.0.0 (`charls.exe` - renamed sample utility)
- **HTJ2K**: OpenHTJ2K 1.0.0 (`open_htj2k_enc`, `open_htj2k_dec`)
- **JPEG 1**: libjpeg-turbo 3.1.3 (`cjpeg`, `djpeg`)

## Summary Table (Latest Results - 2026-01-08)

| Codec | Direction | Status | MAE | Notes |
|-------|-----------|--------|-----|-------|
| **J2K** | Rust -> Ext | ✅ Pass | 0.23 | Fully interoperable with OpenJPEG |
| **J2K** | Ext -> Rust | ✅ Pass | 0.23 | Fully interoperable with OpenJPEG |
| **JLS** | Rust -> Ext | ✅ Pass | 0.00 | Lossless roundtrip confirmed by CharLS |
| **JLS** | Ext -> Rust | ⚠️ Bug | ~112 | Decoder fails on complex external streams |
| **JPEG1** | Rust -> Ext | ✅ Pass | 2.20 | Standard lossy interoperability |
| **JPEG1** | Ext -> Rust | ⚠️ Bug | 127.5 | Likely level-shift or mapping issue |
| **HTJ2K** | Both | ⚠️ Pending | - | Binary compatibility in progress |

## Performance Highlights (1024x1024 Grayscale)

- **JPEG 2000**: ~220ms Encode, ~200ms Decode.
- **JPEG-LS**: ~180ms Encode (Very fast).
- **JPEG 1**: ~210ms Encode.

## Detailed Metrics
Detailed CSV data can be found in `docs/metrics_master_interop.csv`.
