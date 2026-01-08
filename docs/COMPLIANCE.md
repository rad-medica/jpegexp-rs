
## What is Missing / Future Work

While `jpegexp-rs` is highly compliant and production-ready for core medical imaging use cases, the following features are planned for future development:

### 1. High-Throughput Block Encoder (HTJ2K)
- **Current Status**: Encoder uses "Legacy Mode" (Standard JPEG 2000 blocks + HTJ2K signaling CAP marker). This is fully compliant but does not achieve the 10x encoding speedup of native HTJ2K.
- **Goal**: Implement `HTBlockEncoder` to generate native HTJ2K bitstreams (MEL/VLC/MagSgn encoding).

### 2. Advanced Lossy Compression
- **Current Status**: 9-7 Irreversible DWT is implemented.
- **Goal**: Improve rate control and quantization matrices for high-bit-depth (>8-bit) lossy compression to match OpenJPEG PSNR at low bitrates.

### 3. Optimization
- **Goal**: Implement SIMD (AVX2/NEON) for DWT and Block Coding to match or exceed OpenJPEG/OpenJPH performance.

### 4. Advanced Markers
- **Goal**: Support `TLM` (Tile-Part Length), `PLT` (Packet Length), and `RGN` (Region of Interest) markers in encoder.

### 5. Multi-Component Extensions (Part 2)
- **Goal**: Support custom Multi-Component Transforms (MCT) for hyperspectral imaging.
