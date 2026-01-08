# DICOM Compliance Statement

**Standard:** DICOM PS3.5 - Data Structures and Encoding
**Date:** January 8, 2026

## Overview

This document details the compliance of `jpegexp-rs` with DICOM Transfer Syntaxes for image compression. The library is designed to support the core compression algorithms used in medical imaging: JPEG, JPEG-LS, JPEG 2000, and HTJ2K.

---

## 1. Summary of Supported Transfer Syntaxes

| Standard | Transfer Syntax UID | Name | Status | Notes |
|----------|---------------------|------|--------|-------|
| **JPEG 1** | 1.2.840.10008.1.2.4.50 | JPEG Baseline (Process 1) | ✅ Supported | 8-bit only. Default lossy. |
| | 1.2.840.10008.1.2.4.51 | JPEG Extended (Process 2 & 4) | ❌ No | 12-bit JPEG 1 not supported. |
| | 1.2.840.10008.1.2.4.57 | JPEG Lossless (Process 14) | ❌ No | Legacy lossless not supported. |
| | 1.2.840.10008.1.2.4.70 | JPEG Lossless (Selection 1) | ❌ No | Legacy lossless not supported. |
| **JPEG-LS** | 1.2.840.10008.1.2.4.80 | JPEG-LS Lossless | ✅ Supported | 8/16-bit Grayscale. |
| | 1.2.840.10008.1.2.4.81 | JPEG-LS Near-Lossless | ✅ Supported | Configurable error (NEAR). |
| **JPEG 2000** | 1.2.840.10008.1.2.4.90 | JPEG 2000 Lossless Only | ✅ Supported | 5-3 DWT. 8-16 bit. |
| | 1.2.840.10008.1.2.4.91 | JPEG 2000 | ✅ Supported | 9-7 or 5-3 DWT. Lossy/Lossless. |
| | 1.2.840.10008.1.2.4.92 | JPEG 2000 Part 2 Lossless | ❌ No | Multi-component not supported. |
| | 1.2.840.10008.1.2.4.93 | JPEG 2000 Part 2 | ❌ No | Multi-component not supported. |
| **HTJ2K** | 1.2.840.10008.1.2.4.201 | HTJ2K Lossless | ✅ Supported | Legacy mode (CAP marker). |
| | 1.2.840.10008.1.2.4.202 | HTJ2K RPC | ❌ No | Reduced Resolution not supported. |
| | 1.2.840.10008.1.2.4.203 | HTJ2K | ✅ Supported | Legacy mode (CAP marker). |

---

## 2. JPEG 1 Compliance (ISO 10918-1)

### Requirements
- **Photometric Interpretation**: MONOCHROME2, YBR_FULL_422 (for color).
- **Bit Depth**: 8-bit only for Baseline.
- **Pixel Representation**: Unsigned only.

### Implementation
- **Encoder**: Produces standard Baseline JPEG bitstreams.
- **Decoder**: Decodes standard Baseline JPEG.
- **Limitations**:
    - No 12-bit support (requires Extended Hierarchical/lossless processes).
    - No Arithmetic coding support.

---

## 3. JPEG-LS Compliance (ISO 14495-1)

### Requirements
- **Bit Depth**: 2 to 16 bits.
- **Modes**: Lossless and Near-Lossless.
- **Photometric Interpretation**: MONOCHROME2, RGB (if interleaved).

### Implementation
- **Grayscale**: Full support for 8-bit and 16-bit. MAE=0 (Lossless).
- **Color**: **Not Supported** for DICOM (DICOM requires Planar Configuration=0 / Sample Interleaved, but current implementation only supports planar or fails).
- **Markers**: Correctly writes SOF55 (Lossless) or SOF57 (Near-Lossless) markers.
- **Interoperability**: Validated against CharLS.

---

## 4. JPEG 2000 Compliance (ISO 15444-1)

### Core Requirements
- **SOC Marker**: `0xFF4F`
- **SIZ Marker**: Image/tile size, bit depth (preserved from input).
- **COD Marker**:
    - 5-3 Reversible DWT for Lossless (`.90`).
    - 9-7 Irreversible DWT allowed for General (`.91`).
- **QCD Marker**: Quantization steps (0x00 for lossless).

### Medical Imaging Features
- **12-bit / 16-bit Support**:
    - ✅ 12-bit Grayscale verified.
    - ⚠️ 16-bit Grayscale implemented (limited testing).
    - ⚠️ 12-bit Color has known artifacts in large blocks.
- **Photometric Interpretations**:
    - ✅ MONOCHROME2 (Grayscale)
    - ✅ YBR_RCT (Lossless Color)
    - ✅ YBR_ICT (Lossy Color)
    - ❌ MONOCHROME1 (Inverse Grayscale)
- **Encapsulation**:
    - Library generates the **raw codestream** (contiguous).
    - **Note**: DICOM requires encapsulation in Item Tags (FFFE,E000). The user must wrap the raw output of `jpegexp-rs` into DICOM fragments.

### Compliance Checklist
| Feature | Status | Notes |
|---------|--------|-------|
| 5-3 Reversible DWT | ✅ Pass | Bit-exact reconstruction. |
| 9-7 Irreversible DWT | ✅ Pass | High quality. |
| Scalar Quantization | ✅ Pass | |
| Region of Interest | ❌ No | |
| Multi-component | ❌ No | Part 2 extension. |

---

## 5. HTJ2K Compliance (ISO 15444-15)

**Supplement 235** introduces High-Throughput JPEG 2000.

### Requirements
- **CAP Marker**: **Mandatory**. Must be present in Main Header.
    - `Pcap` bit 14 must be set (indicating HTJ2K).
    - `Ccap` parameters must be present.
- **Codestream**: Can use "HT" block coding (fast) or "Legacy" block coding (standard J2K + CAP marker).

### Implementation
- **Mode**: **Legacy Mode**. The encoder produces standard JPEG 2000 code-blocks but adds the **CAP marker** with bit 14 set.
- **Compliance**: This is a valid HTJ2K codestream (compliant with ISO 15444-15). It allows HTJ2K decoders to recognize the file, though it doesn't offer the decoding speedup of native HT blocks.
- **Validation**: Verified with OpenHTJ2K decoder.

### Checklist
- [x] **CAP Marker**: Generated correctly (Pcap = 0x20000).
- [x] **Transfer Syntax**: Compatible with `.201` (Lossless) and `.203` (General).
- [ ] **Native HT Encoding**: Future work (for speed).

---

## 6. Known Limitations for Clinical Use

1.  **Encapsulation**: `jpegexp-rs` outputs raw bitstreams (`.j2k`, `.jls`, `.jpg`). The host application is responsible for fragmenting and encapsulating these into DICOM tags (`7FE0,0010`).
2.  **Color Space**:
    - JPEG-LS RGB is not yet supported in the interleaved format required by DICOM.
    - JPEG 2000 RGB assumes sRGB primaries; explicit color profile handling is minimal.
3.  **Signed Pixels**:
    - Support for `Pixel Representation = 1` (signed integers) is implemented for JPEG 2000 but requires careful validation of the `Siz` marker `Ssigned` bit.

## Conclusion

`jpegexp-rs` is a strong candidate for a DICOM transcoding library, particularly for **JPEG 2000 Lossless (Grayscale)** and **JPEG-LS (Grayscale)**. It meets the rigorous bit-exact requirements for diagnostic imaging storage.
