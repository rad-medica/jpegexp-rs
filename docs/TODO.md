# Project Roadmap & TODOs

This document tracks the backlog of planned features, improvements, and known issues for `jpegexp-rs`.

## 🧩 Compliance & Interoperability Gaps

### JPEG 2000 Standard (ISO 15444-1)
- [ ] **Markers**: Support writing `TLM` (Tile-Part Length) and `PLT` (Packet Length) markers for faster random access decoding.
- [ ] **Profiles**: Add specific profile constraints (Cinema, Broadcast) to encoder configuration.
- [ ] **Metadata**: Correctly map Color Space (sRGB, ICC) and Pixel Representation (Signed/Unsigned) to `COLR` and `SIZ` markers.

### DICOM Compliance
- [x] **Encapsulation**: ✅ Implement DICOM fragment encapsulation (`Item Tag` wrapping) for raw codestreams.
- [x] **Basic Offset Table**: ✅ Generate BOT for multi-frame support.
- [x] **Photometric Interpretation**: ✅ Support `MONOCHROME1` (Inverse Grayscale) encoding path.
- [x] **Signed Pixel Data**: ✅ Support `Pixel Representation = 1` for CT Hounsfield Units.

### JPEG 1 Extended
- [ ] **12-bit Support**: Implement "Extended Sequential" process for 12-bit medical X-ray/CT support.

### HTJ2K Extensions
- [ ] **RPC Mode**: Support Reduced Resolution (RPC) Transfer Syntax (.202).

## 🛑 High Priority (Immediate)

### 1. JPEG 2000 Lossy Quantization Fix
**Issue**: The current encoder implementation for lossy compression (9-7 DWT + Scalar Expounded Quantization) produces poor quality results when DWT is enabled.
**Status**: ✅ Fixed
**Task**: 
- [x] Debug `src/jpeg2000/encoder.rs` quantization logic.
- [x] Align step size calculation with `src/jpeg2000/image.rs` (decoder).
- [x] Verify PSNR > 40dB for Q90.

### 2. JPEG-LS RGB Sample Interleave
**Issue**: RGB images currently encode in "Planar" mode (RRR...GGG...BBB...) or fail. DICOM and many viewers require "Sample Interleaved" (RGBRGB...).
**Status**: ⚠️ Deferred - Grayscale production-ready
**Task**:
- [x] Implement triplet processing in `src/jpegls/encoder.rs`.
- [x] Update `scan_encoder.rs` to handle `ILV_SAMPLE` mode.
- [x] Fix grayscale regression (reverted incorrect buffer indexing changes)
- [ ] Fix RGB CharLS interop bit over-consumption issue (~2.1x efficiency gap)
- [ ] Verify against CharLS with interleaved input (Decoder compliance issue remains).

**Decision**: Focus on grayscale production deployment first. RGB support deferred pending different debugging approach (see `docs/SESSION_SUMMARY_RGB_JPEGLS_DEBUG.md`).

## ⚠️ Medium Priority

### 3. Native HTJ2K Encoding
**Issue**: Current HTJ2K encoder uses "Legacy Mode" (Standard code-blocks + CAP marker). It is compliant but doesn't offer the 10x encoding speedup of native HTJ2K.
**Status**: ⚠️ Partially implemented - Basic structure exists but magnitude encoding (EMB pattern) incomplete
**Task**:
- [ ] Complete `HTBlockEncoder` magnitude encoding using EMB pattern
- [ ] Implement U_q state machine for magnitude prediction
- [ ] Implement pLSB (predicted Least Significant Bit) logic
- [ ] Complete HTJ2K decoder magnitude refinement
- [ ] Test against OpenHTJ2K reference implementation
- [ ] Verify lossless encoding (MAE=0)

### 4. Advanced JPEG 2000 Features
- [ ] **Tiling**: Support for splitting large images into tiles (currently single tile).
- [ ] **ROI**: Region of Interest coding.
- [ ] **Multi-Layer**: Progressive quality layers (currently single layer).

## 📉 Low Priority / Optimization

### 5. SIMD Optimization
- [ ] **DWT**: Implement AVX2/NEON intrinsics for 5-3 and 9-7 lifting steps.
- [ ] **Color Transform**: SIMD for ICT/RCT.
- [ ] **Block Coding**: Vectorize bit-plane operations.

### 6. WASM Polish
- [ ] Improve the web demo UI.
- [ ] Expose more configuration options to JS API.

---

## 🐛 Known Issues Tracker

| ID | Component | Issue | Status |
|----|-----------|-------|--------|
| **J2K-01** | Encoder | Lossy quantization quality mismatch | 🟢 Fixed |
| **J2K-02** | Encoder | 12-bit Color artifacts >32x32 blocks | 🟢 Working |
| **JLS-01** | Encoder | No RGB Interleave support | 🟢 Fixed - Self-consistent |
| **JLS-02** | Interop | CharLS RGB interop (bit over-consumption) | 🟡 Deferred |
| **JLS-03** | Decoder | Grayscale regression (Rb/Rd init) | 🟢 Fixed (2026-01-08) |
| **JLS-04** | Decoder | 1-pixel wide images (edge case) | 🟡 Open - Very rare edge case |
| **JLS-05** | Decoder | Solid/constant images (run mode) | 🟡 Open - CharLS specific encoding |
| **HT-01** | Decoder | OpenHTJ2K compatibility (level shifting) | 🟡 Open |
| **HT-02** | Encoder | Native HT magnitude encoding incomplete | 🟡 Open |
| **J1-01** | Decoder | Some standard RGB JPEGs fail to decode | 🟢 Low Priority |
