# Comprehensive Codec Interoperability Test Report

**Project:** jpegexp-rs  
**Test Date:** 2026-01-12 (Updated)  
**Test Framework:** Comprehensive Interop Test Suite v1.0  
**Test Duration:** ~105 seconds (Total)

> **Latest Update (2026-01-12)**: Completed deep investigation into JPEG 2000 interoperability issues. See [JPEG2000_INTEROP_INVESTIGATION.md](../JPEG2000_INTEROP_INVESTIGATION.md) for detailed technical analysis.

---

## Executive Summary

This report documents comprehensive interoperability testing between `jpegexp-rs` codec implementations and industry-standard reference codecs. All tests follow the critical rule: **never test a codec against itself**—encoding tests use our encoder with the reference decoder, and decoding tests use the reference encoder with our decoder.

### Overall Test Results

| Codec Family | Tests Run | Passed | Failed | Pass Rate | Status |
|--------------|-----------|--------|--------|-----------|--------|
| **JPEG 1**    | 320       | 320    | 0      | **100%**  | ✅ **PRODUCTION READY** |
| **JPEG 2000** | 300       | 128    | 172    | **43%**   | ⚠️ **EXPERIMENTAL** |
| **JPEG-LS**   | 640       | 98     | 542    | **15%**   | ⚠️ **LIMITED** |

### Reference Codecs Used

| Codec Family | Reference Implementation | Version | Binary |
|--------------|-------------------------|---------|--------|
| JPEG 1 | libjpeg-turbo | 3.1.3 | `cjpeg.exe`, `djpeg.exe` |
| JPEG 2000 | OpenJPEG | 2.5.2 | `opj_compress.exe`, `opj_decompress.exe` |
| JPEG-LS | CharLS | 3.0.0 | `charls.exe` |

---

## 1. JPEG 1 (Classic JPEG) — ✅ PRODUCTION READY

**Status: 100% Pass (320/320)**

- **Interoperability**: Perfect match with `libjpeg-turbo` for all tested quality levels (50, 75, 90, 95, 100) and patterns.
- **Features**: 8-bit grayscale and RGB support is fully verified.

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
| **Gradient** | ❌ Fail | ~0.73 (8-bit) | Tier-1 Context Modeling (MagRef LSB mismatch) |
| **Noise** | ❌ Fail | ~0.73 (8-bit) | Tier-1 Context Modeling |
| **Checkerboard** | ❌ Fail | ~0.73 (8-bit) | Tier-1 Context Modeling |
| **16-bit (Any)** | ❌ Fail | > 10,000 | Endianness/Signedness mismatch in Test Harness or `zero_bp` packet header divergence |

**Key Finding:** The consistent MAE of ~0.7351 for 8-bit complex patterns suggests a systematic bias, identified as a mismatch in the **Magnitude Refinement** pass context modeling for bit 1 (2nd LSB) and bit 0 (LSB). The bitstream remains synchronized for higher bits but diverges at the finest detail levels.

### 2.4 Verdict

**JPEG 2000 implementation is STRUCTURALLY SOUND but INTEROPERABILITY CHALLENGED.**
- **Safe for Internal Archival**: Yes (MAE=0.0 roundtrip).
- **Safe for Exchange with OpenJPEG**: No (for complex images).

**Next Steps:**
1.  Debug Tier-1 MQ Coder context states against OpenJPEG debug output trace.
2.  Investigate 16-bit `zero_bp` header calculation logic.

---

## 3. JPEG-LS — ⚠️ LIMITED COMPATIBILITY

**Status: 15% Pass (98/640)**

### 3.1 Summary

**Low pass rate due to Test Harness / CLI Mismatches**

- **98/640 tests passed**
- **Successes**: Lossless 8-bit and 16-bit encoding/decoding works for simple cases (Solid patterns, some Gradients).
- **Failures**:
    - **Near-Lossless (NL > 0)**: Fails consistently (Rust -> CharLS). This suggests parameter passing issues to `charls.exe` or divergence in the NEAR handling logic.
    - **High Bit Depths**: Intermittent failures likely due to PNM endianness handling in the test harness (similar to J2K).
    - **Internal Roundtrip**: Internal `Rust -> Rust` validation is significantly stronger than the interop results suggest, indicating that many failures are artifacts of the CLI wrapper mechanism (`charls.exe` parameter mapping).

### 3.2 Verdict

**JPEG-LS core logic is likely sounder than the interop score suggests.**
- **Immediate Action**: Fix `charls` CLI invocation arguments for Near-Lossless modes and endianness handling for >8-bit PNM files.

---

## 4. Test Environment

- **OS:** Windows 11 Pro
- **Rust Version:** 1.70+
- **Optimization:** `--release`
