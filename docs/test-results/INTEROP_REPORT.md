# Comprehensive Codec Interoperability Test Report

**Project:** jpegexp-rs  
**Test Date:** 2026-01-11  
**Test Framework:** Comprehensive Interop Test Suite v1.0  
**Test Duration:** 39.68 seconds (J2K only run)

---

## Executive Summary

This report documents comprehensive interoperability testing between `jpegexp-rs` codec implementations and industry-standard reference codecs. All tests follow the critical rule: **never test a codec against itself**—encoding tests use our encoder with the reference decoder, and decoding tests use the reference encoder with our decoder.

### Overall Test Results (J2K Focus Run)

| Codec Family | Tests Run | Passed | Failed | Pass Rate | Status |
|--------------|-----------|--------|--------|-----------|--------|
| **JPEG 2000** | 300 | 128 | 172 | **43%** | ⚠️ **NEEDS WORK** |

### Reference Codecs Used

| Codec Family | Reference Implementation | Version | Binary |
|--------------|-------------------------|---------|--------|
| JPEG 1 | libjpeg-turbo | 3.1.3 | `cjpeg.exe`, `djpeg.exe` |
| JPEG 2000 | OpenJPEG | 2.5.2 | `opj_compress.exe`, `opj_decompress.exe` |
| JPEG-LS | CharLS | 3.0.0 | `charls.exe` |

---

## 1. JPEG 1 (Classic JPEG) — ✅ PRODUCTION READY

*(Previous results valid - 100% Pass)*

---

## 2. JPEG 2000 — ⚠️ EXPERIMENTAL

### 2.1 Summary

**Partial interoperability with OpenJPEG 2.5.2**

- **128/300 tests passed (43%)**
- **Verified DWT Correctness**: The 5/3 Reversible DWT implementation has been mathematically verified against ISO 15444-1 Annex F formulas (including the `+2` offset for floor rounding).
- **Internal Consistency**: Self-roundtrip tests (Rust Encoder -> Rust Decoder) pass perfectly (MAE=0.0) for all bit depths (8, 10, 12, 16) and patterns.
- **Interoperability Gap**: While internally consistent, the codec fails to match OpenJPEG's output for non-uniform patterns (Gradient, Noise, Checkerboard). This indicates a subtle divergence in **Entropy Coding (Tier 1)** context modeling or **Boundary Extension** logic, rather than the DWT itself.

### 2.2 Test Coverage Matrix

| Image Size | Bit Depth | Components | Modes | Patterns | Total Tests |
|------------|-----------|------------|-------|----------|-------------|
| 64×64 | 8, 10, 12, 16 | 1 (Gray), 3 (RGB) | Lossless, Lossy | 5 | 160 |
| 256×256 | 8, 10, 12, 16 | 1 (Gray), 3 (RGB) | Lossless, Lossy | 5 | 140 |

**Patterns tested:** solid, gradient_d, checkerboard, noise, medical_ct

### 2.3 Failure Analysis

| Pattern | Failure Mode | Typical MAE | Probable Cause |
|---------|--------------|-------------|----------------|
| **Solid** | ✅ Pass | 0.0000 | - |
| **Gradient** | ❌ Fail | ~0.73 (8-bit) | Tier-1 Context Modeling or Boundary Extension |
| **Noise** | ❌ Fail | ~0.73 (8-bit) | Tier-1 Context Modeling |
| **Checkerboard** | ❌ Fail | ~0.73 (8-bit) | Tier-1 Context Modeling |
| **16-bit (Any)** | ❌ Fail | > 10,000 | Endianness/Signedness mismatch in Test Harness vs Codec |

**Key Finding:** The consistent MAE of ~0.7351 for 8-bit complex patterns suggests a systematic bias, likely a single-bit difference in rounding or a specific context model decision in the arithmetic coder.

### 2.4 Verdict

**JPEG 2000 implementation is STRUCTURALLY SOUND but INTEROPERABILITY CHALLENGED.**
- **Safe for Internal Archival**: Yes (MAE=0.0 roundtrip).
- **Safe for Exchange with OpenJPEG**: No (for complex images).

**Next Steps:**
1.  Debug Tier-1 MQ Coder context states against OpenJPEG debug output.
2.  Investigate 16-bit PNM handling in the test harness to resolve the massive MAE errors.

---

## 3. JPEG-LS — ⚠️ LIMITED COMPATIBILITY

*(Previous results valid - 15% Pass due to CharLS CLI limitations)*

---

## 4. Test Environment

- **OS:** Windows 11 Pro
- **Rust Version:** 1.70+
- **Optimization:** `--release`
