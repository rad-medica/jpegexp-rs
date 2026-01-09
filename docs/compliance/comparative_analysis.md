# Comparative Analysis: jpegexp-rs vs. Reference Codecs

This document provides a feature-by-feature comparison between `jpegexp-rs` and the industry-standard reference implementations for each supported JPEG format.

## 📊 Feature Support Matrix

| Feature | **jpegexp-rs** | **libjpeg-turbo** (J1) | **CharLS** (JLS) | **OpenJPEG** (J2K) | **OpenHTJ2K** (HT) |
|---------|:--------------:|:----------------------:|:----------------:|:------------------:|:------------------:|
| **JPEG 1: 8-bit Baseline (SOF0)** | ✅ | ✅ | - | - | - |
| **JPEG 1: 12-bit Extended (SOF1)** | ✅ | ✅ | - | - | - |
| **JPEG 1: Progressive (SOF2)** | ✅ (D) | ✅ | - | - | - |
| **JPEG-LS: Lossless (SOF55)** | ✅ | - | ✅ | - | - |
| **JPEG-LS: Near-Lossless (SOF57)**| ✅ | - | ✅ | - | - |
| **JPEG-LS: RGB Interleave (ILV=2)**| ✅ | - | ✅ | - | - |
| **JPEG 2000: Lossless (Part 1)** | ✅ | - | - | ✅ | ✅ |
| **JPEG 2000: Lossy (Part 1)** | ✅ | - | - | ✅ | ✅ |
| **HTJ2K: Native EMB Encoding** | ✅ | - | - | - | ✅ |
| **HTJ2K: CAP Marker Signaling** | ✅ | - | - | - | ✅ |
| **Markers: TLM / PLT** | ✅ | - | - | ✅ | ✅ |
| **DICOM Encapsulation** | ✅ | ❌ | ❌ | ❌ | ❌ |
| **DICOM MONOCHROME1 Support** | ✅ | ❌ | ❌ | ❌ | ❌ |
| **DICOM Signed Pixel Data** | ✅ | ❌ | ❌ | ❌ | ❌ |

*(D) = Decoding only*

---

## 🔍 Detailed Comparison

### 1. JPEG 1 (ISO 10918-1)
- **jpegexp-rs**: Focused on medical imaging. Supports the rare **12-bit SOF1** format commonly found in CT/X-ray, along with standard 8-bit Baseline. 
- **libjpeg-turbo**: The performance benchmark. While it supports 12-bit, it usually requires a separate build. `jpegexp-rs` supports both bit depths in a single unified API.

### 2. JPEG-LS (ISO 14495-1)
- **jpegexp-rs**: Implements **shared context modeling** for multi-component images, achieving compression ratios identical to CharLS.
- **CharLS**: The reference implementation. `jpegexp-rs` is 100% bitstream-compatible for both grayscale and sample-interleaved RGB.

### 3. JPEG 2000 (ISO 15444-1)
- **jpegexp-rs**: Prioritizes compliance and medical features (Signed pixels, MONOCHROME1). Includes support for **TLM** and **PLT** markers which are often omitted in simpler encoders but critical for PACS random access.
- **OpenJPEG**: Very feature-complete but can be slow and lacks built-in DICOM encapsulation logic.

### 4. HTJ2K (ISO 15444-15)
- **jpegexp-rs**: Implements native **EMB (Exponents and MagSgn Bits)** encoding. This allows `jpegexp-rs` to produce high-throughput-ready codestreams that are natively understood by modern GPU decoders.
- **OpenHTJ2K / OpenJPH**: Reference HT implementations. `jpegexp-rs` matches their bitstream structure for the HT block coding pass.

---

## 📈 Performance & Portability

| Metric | **jpegexp-rs** | **Reference (C/C++)** |
|--------|:--------------:|:---------------------:|
| **Language** | Pure Rust (Safe) | C / C++ (Unsafe) |
| **Portability** | WASM / ARM / x64 | Platform-specific |
| **DICOM Ready**| Built-in | Needs wrapper |
| **SIMD Support**| Work-in-progress | ✅ Mature |

## Conclusion

`jpegexp-rs` provides a unique value proposition by combining **four major standards** into a single, memory-safe Rust library with **first-class DICOM support**. While reference codecs may offer higher raw throughput via mature SIMD optimizations, `jpegexp-rs` offers superior integration for medical imaging pipelines and modern web-based (WASM) viewers.
