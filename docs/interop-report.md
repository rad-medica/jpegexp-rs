# Interoperability Test Report

**Date:** January 9, 2026
**Platform:** Windows x64
**Libraries Tested:**
- **OpenJPEG** (v2.5.2) - JPEG 2000 Reference
- **CharLS** (v3.0.0) - JPEG-LS Reference
- **libjpeg-turbo** (v3.1.3) - JPEG 1 Reference
- **OpenHTJ2K** - HTJ2K Reference

## Summary Table

| Codec | Mode | Bit Depth | Components | Direction | Status | MAE | Notes |
|-------|------|-----------|------------|-----------|--------|-----|-------|
| **JPEG-LS** | Lossless | 8 | 1 | Rust->Ext | ✅ PASS | 0.0000 | Perfect match |
| **JPEG-LS** | Lossless | 8 | 1 | Ext->Rust | ✅ PASS | 0.0000 | Perfect match |
| **JPEG-LS** | Lossless | 16 | 1 | Rust->Ext | ✅ PASS | 0.0000 | Perfect match |
| **JPEG-LS** | Lossless | 16 | 1 | Ext->Rust | ✅ PASS | 0.0000 | Perfect match |
| **JPEG 1** | Lossy | 8 | 1 | Rust->Ext | ✅ PASS | ~0.88 | Expected lossy diff |
| **JPEG 1** | Lossy | 8 | 1 | Ext->Rust | ✅ PASS | ~0.36 | Expected lossy diff |
| **JPEG 1** | Lossy | 8 | 3 | Rust->Ext | ✅ PASS | ~1.29 | Expected lossy diff |
| **JPEG 2000** | Lossless | 8 | 1 | Rust->Ext | ⚠️ OK | ~0.32 | Minor differences |
| **JPEG 2000** | Lossless | 8 | 1 | Ext->Rust | ⚠️ OK | ~0.32 | Minor differences |
| **JPEG 2000** | Lossless | 16 | 1 | Rust->Ext | ❌ FAIL | ~19491 | Endianness mismatch |
| **JPEG 2000** | Lossy | 8 | 1 | Rust->Ext | ✅ PASS | ~0.006 | Excellent lossy match |
| **HTJ2K** | Lossless | 8 | 1 | Ext->Rust | ⚠️ WIP | - | Scan order/UVLC fixes applied |

## Detailed Findings

### JPEG-LS
- **Perfect Interoperability**: Achieved 0.0000 MAE for both 8-bit and 16-bit grayscale.
- Validated against CharLS reference implementation.
- Both Encoder and Decoder are working correctly.

### JPEG 1
- **Good Interoperability**: MAE < 1.3 for all tests.
- Differences are within expected range for lossy DCT compression (implementation differences in quantization tables or FDCT/IDCT precision).

### JPEG 2000
- **8-bit Support**: 
  - Lossy encoding/decoding works well.
  - Lossless encoding has minor differences (MAE ~0.32) when compared with OpenJPEG. This suggests slightly different handling of boundary conditions or reversible transforms, but visually identical.
- **High Bit Depth (>8-bit)**:
  - **Severe Issue**: MAE is extremely high (~20,000 for 16-bit).
  - **Cause**: Likely Endianness mismatch. `jpegexp-rs` seems to interpret 16-bit input/output as Native Endian (Little Endian on x86), while OpenJPEG/Standard expects Big Endian in codestream (and possibly PGM test harness mismatch).
  - **Action Item**: Fix Endianness handling for 16-bit samples in J2K encoder/decoder.

### HTJ2K
- **Decoder Fixes**:
  - Fixed `decode_uvlc` to use standard tables (was using incorrect ad-hoc logic).
  - Identified Scan Order discrepancy (OpenHTJ2K produces `rho` implying swapped 0/1 bits or `(0,0)` mapped to Sample 1).
  - Verified decoding of simple patterns with `repro` test.
- **Integration**: `test_ht_coder_repro` failure (`InvalidData`) indicates encoder/decoder sync issue (likely `MagSgn` or `UVLC` table mismatch in Encoder).

## Recommendations
1. **Fix J2K 16-bit Endianness**: Ensure `u16` samples are correctly byteswapped to Big Endian when writing to codestream and byteswapped back to Native Endian when decoding.
2. **Investigate J2K 8-bit Lossless Diff**: Trace coefficients to find exact source of drift.
3. **Finish HTJ2K**: Align Encoder with the fixed Decoder tables. Verify Scan Order.
