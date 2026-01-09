# Project Roadmap & TODOs

This document tracks the backlog of planned features, improvements, and known issues for `jpegexp-rs`.

## 🧩 Compliance & Interoperability Gaps

### JPEG 2000 Standard (ISO 15444-1)
- [x] **Markers**: Support writing `TLM` (Tile-Part Length) and `PLT` (Packet Length) markers.
- [ ] **Profiles**: Add specific profile constraints (Cinema, Broadcast) to encoder configuration.
- [ ] **Metadata**: Correctly map Color Space (sRGB, ICC) and Pixel Representation (Signed/Unsigned) to `COLR` and `SIZ` markers.

### DICOM Compliance
- [x] **Encapsulation**: ✅ Implement DICOM fragment encapsulation (`Item Tag` wrapping).
- [x] **Basic Offset Table**: ✅ Generate BOT for multi-frame support.
- [x] **Photometric Interpretation**: ✅ Support `MONOCHROME1` (Inverse Grayscale) encoding path.
- [x] **Signed Pixel Data**: ✅ Support `Pixel Representation = 1` for CT Hounsfield Units.

### JPEG 1 Extended
- [x] **12-bit Support**: ✅ Implement "Extended Sequential" process (SOF1) for 12-bit medical X-ray/CT support.

### HTJ2K Extensions
- [x] **Native Magnitude Encoding**: ✅ Implement EMB pattern and U_q state machine (Part 15).
- [ ] **RPC Mode**: Support Reduced Resolution (RPC) Transfer Syntax (.202).

## 🛑 High Priority (Immediate)

### 1. Native HTJ2K SIMD Optimization
- [ ] **DWT**: Implement AVX2/NEON intrinsics for 5-3 and 9-7 lifting steps.
- [ ] **Block Coding**: Vectorize bit-plane operations (VLC/MagSgn).

### 2. Multi-tile Support
- [ ] Implement tiling logic in the encoder to handle extremely large images (e.g., Digital Pathology).

## ⚠️ Medium Priority

### 3. Advanced JPEG 2000 Features
- [ ] **ROI**: Region of Interest coding.
- [ ] **Multi-Layer**: Progressive quality layers (currently single layer).

## 📉 Low Priority / Optimization

### 4. WASM Polish
- [ ] Improve the web demo UI.
- [ ] Expose more configuration options to JS API.

---

## 🐛 Known Issues Tracker

| ID | Component | Issue | Status |
|----|-----------|-------|--------|
| **J2K-01** | Encoder | Lossy quantization quality mismatch | 🟢 Fixed |
| **J2K-02** | Encoder | 12-bit Color artifacts >32x32 blocks | 🟢 Working |
| **JLS-01** | Encoder | No RGB Interleave support | 🟢 Fixed - CharLS interop verified |
| **JLS-02** | Interop | CharLS RGB interop bit over-consumption | 🟢 Fixed (Context sharing) |
| **JLS-03** | Decoder | Grayscale regression (Rb/Rd init) | 🟢 Fixed |
| **HT-01** | Encoder | Native Magnitude Encoding missing | 🟢 Fixed (EMB implemented) |
| **J1-01** | Decoder | Some standard RGB JPEGs fail to decode | 🟢 Low Priority |
| **J1-02** | Encoder | 12-bit SOF1 requires custom Huffman for high quality | 🟡 Research |
