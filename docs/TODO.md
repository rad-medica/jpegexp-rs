# Project Roadmap & TODOs

This document tracks the backlog of planned features, improvements, and known issues for `jpegexp-rs`.

## 🛑 High Priority (Immediate)

### 1. JPEG 2000 Lossy Quantization Fix
**Issue**: The current encoder implementation for lossy compression (9-7 DWT + Scalar Expounded Quantization) produces poor quality results when DWT is enabled.
**Root Cause**: Mismatch between encoder's epsilon/mantissa calculation and the decoder's dequantization formula.
**Task**: 
- [ ] Debug `src/jpeg2000/encoder.rs` quantization logic.
- [ ] Align step size calculation with `src/jpeg2000/image.rs` (decoder).
- [ ] Verify PSNR > 40dB for Q90.

### 2. JPEG-LS RGB Sample Interleave
**Issue**: RGB images currently encode in "Planar" mode (RRR...GGG...BBB...) or fail. DICOM and many viewers require "Sample Interleaved" (RGBRGB...).
**Task**:
- [ ] Implement triplet processing in `src/jpegls/encoder.rs`.
- [ ] Update `scan_encoder.rs` to handle `ILV_SAMPLE` mode.
- [ ] Verify against CharLS with interleaved input.

## ⚠️ Medium Priority

### 3. Native HTJ2K Encoding
**Issue**: Current HTJ2K encoder uses "Legacy Mode" (Standard code-blocks + CAP marker). It is compliant but doesn't offer the 10x encoding speedup of native HTJ2K.
**Task**:
- [ ] Implement `HTBlockEncoder` in `src/jpeg2000/ht_block_coder/`.
- [ ] Implement MEL (Magnitude Exponent Logic) state machine.
- [ ] Implement VLC (Variable Length Coding) forward path.
- [ ] Implement MagSgn bit packing.

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
| **J2K-01** | Encoder | Lossy quantization quality mismatch | 🔴 Open |
| **J2K-02** | Encoder | 12-bit Color artifacts >32x32 blocks | 🟡 Open |
| **JLS-01** | Encoder | No RGB Interleave support | 🔴 Open |
| **HT-01** | Decoder | OpenHTJ2K compatibility (level shifting) | 🟡 Open |
| **J1-01** | Decoder | Some standard RGB JPEGs fail to decode | 🟢 Low Priority |
