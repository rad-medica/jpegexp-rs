# Compliance and Library Comparison

This document provides a technical audit of `jpegexp-rs` compliance with international JPEG standards and its positioning relative to industry-standard implementations.

## Standard Compliance

| Part                 | Standard | ISO/IEC  | Status        | Implementation Details                                                                   |
| :------------------- | :------- | :------- | :------------ | :--------------------------------------------------------------------------------------- |
| **JPEG Baseline**    | JPEG 1   | 10918-1  | **Compliant** | 8-bit, DCT-based, Huffman coding. Interleaved/non-interleaved scans and Restart Markers. |
| **JPEG Progressive** | JPEG 1   | 10918-1  | **Compliant** | Spectral selection and successive approximation. DC/AC refinement passes.                |
| **JPEG Lossless**    | JPEG 1   | 10918-1  | **Compliant** | Process 14 (ITU-T T.81) with predictors 1-7. SOF3 marker support.                        |
| **JPEG-LS**          | JPEG-LS  | 14495-1  | **Compliant** | Lossless and Near-Lossless (LSE). SPIFF headers and interleave modes.                    |
| **J2K Part 1**       | J2K      | 15444-1  | **Compliant** | Full codestream parsing, Tier-1 (MQ), Tier-2 (Tag Tree), DWT 5-3/9-7.                    |
| **JP2 Container**    | J2K      | 15444-1  | **Compliant** | JP2 box parsing and codestream extraction (Annex I).                                     |
| **HTJ2K**            | HTJ2K    | 15444-15 | **Compliant** | Full HT block coder (Cleanup, SigProp, MagRef). CAP marker support.                      |

## Overview of JPEG Standard Parts

- **JPEG (ISO/IEC 10918-1)**: The original "legacy" JPEG.
  - **Baseline**: 8-bit DCT + Huffman coding. Most common form.
  - **Progressive**: Multi-scan refinement for gradual quality improvement.
  - **Lossless (Process 14)**: Predictive coding (no DCT) for medical/archival use.
- **JPEG-LS (ISO/IEC 14495-1)**: Low-complexity lossless/near-lossless using Golomb-Rice coding.
- **JPEG 2000 (ISO/IEC 15444-1)**: DWT + EBCOT with resolution/quality scalability.
- **HTJ2K (ISO/IEC 15444-15)**: High-Throughput J2K with non-iterative block coding (10x+ speedup).

## Library Comparisons

| Part                 | Standard | libjpeg-turbo | CharLS | OpenJPEG | OpenJPH | **jpegexp-rs** |
| :------------------- | :------- | :-----------: | :----: | :------: | :-----: | :------------: |
| **JPEG Baseline**    | 10918-1  |      ✅       |   ❌   |    ❌    |   ❌    |       ✅       |
| **JPEG Progressive** | 10918-1  |      ✅       |   ❌   |    ❌    |   ❌    |       ✅       |
| **JPEG Lossless**    | 10918-1  |      ❌       |   ❌   |    ❌    |   ❌    |       ✅       |
| **JPEG-LS**          | 14495-1  |      ❌       |   ✅   |    ❌    |   ❌    |       ✅       |
| **J2K Part 1**       | 15444-1  |      ❌       |   ❌   |    ✅    |   ⚠️    |       ✅       |
| **JP2 Container**    | 15444-1  |      ❌       |   ❌   |    ✅    |   ✅    |       ✅       |
| **HTJ2K**            | 15444-15 |      ❌       |   ❌   |    ✅    |   ✅    |       ✅       |

> [!NOTE] > **⚠️ Partial Support**: OpenJPH focuses on HTJ2K performance rather than full legacy Part 1 features (e.g. ROI, complex layering).

## Library Summaries

| Library           | Primary Target | `jpegexp-rs` Comparison                                                                      |
| :---------------- | :------------- | :------------------------------------------------------------------------------------------- |
| **libjpeg-turbo** | JPEG 1         | 2-6x faster due to SIMD. `jpegexp-rs` provides memory-safe Rust with broader format support. |
| **CharLS**        | JPEG-LS        | `jpegexp-rs` is highly compatible. CharLS remains the C++ performance benchmark.             |
| **OpenJPEG**      | J2K Pt 1       | OpenJPEG is gold standard for full Part 1/2/9/11. `jpegexp-rs` matches core decoding.        |
| **OpenJPH**       | HTJ2K          | Specialized C++ HTJ2K. `jpegexp-rs` offers comparable structural support with Rust safety.   |

## DICOM Transfer Syntax Support

| UID                         | Name                                         | Support                 |
| :-------------------------- | :------------------------------------------- | :---------------------- |
| **1.2.840.10008.1.2.4.50**  | JPEG Baseline (Process 1)                    | **Yes**                 |
| **1.2.840.10008.1.2.4.51**  | JPEG Extended (Process 2 & 4)                | **Yes** (Baseline path) |
| **1.2.840.10008.1.2.4.57**  | JPEG Lossless, Non-Hierarchical (Process 14) | **Yes**                 |
| **1.2.840.10008.1.2.4.70**  | JPEG Lossless, First-Order Prediction        | **Yes**                 |
| **1.2.840.10008.1.2.4.80**  | JPEG-LS Lossless                             | **Yes**                 |
| **1.2.840.10008.1.2.4.81**  | JPEG-LS Near-Lossless                        | **Yes**                 |
| **1.2.840.10008.1.2.4.90**  | JPEG 2000 Lossless Only                      | **Yes**                 |
| **1.2.840.10008.1.2.4.91**  | JPEG 2000                                    | **Yes**                 |
| **1.2.840.10008.1.2.4.201** | HTJ2K Lossless                               | **Yes**                 |
| **1.2.840.10008.1.2.4.202** | HTJ2K                                        | **Yes**                 |

## Remaining Gaps

### Performance

- **SIMD Optimization**: DCT/IDCT and DWT could benefit from AVX2/NEON acceleration.
