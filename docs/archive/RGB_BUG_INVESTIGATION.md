# JPEG 2000 RGB Encoding Bug - Investigation Summary

## Date
2026-01-08

## Status
**BUG CONFIRMED** - RGB encoding fails for specific patterns and image sizes

## Key Findings

### What We Know

1. **RCT Transform Works Correctly**
   - Forward RCT: Y=(R+2G+B)/4, U=B-G, V=R-G ✅
   - Inverse RCT math is correct ✅
   - Simple 2x2-4x4 images work perfectly ✅

2. **Size-Dependent Failure**
   - **8×8 and smaller: PASS** ✅
   - **16×16 and larger: FAIL** ❌
   - Failure threshold is exactly at 16×16 pixels

3. **Pattern-Dependent Failure**
   - R=G=B (same data all channels): **PASS** ✅
   - Gradients (smooth transitions): **PASS** ✅
   - Different frequencies per channel: **PASS** ✅  
   - **Inverted phase** (R/B checkerboard, G inverted): **FAIL** ❌

4. **Error Characteristics**
   - MAE decreases with size: 18.5 (16×16) → 7.1 (32×32) → 4.0 (64×64) → 2.9 (128×128)
   - First pixel always decoded as R=108, G=146, B=108 (across all failing sizes)
   - Error affects **all three components** even though only G is inverted
   - Approximately 40-50% of pixels have errors

### Root Cause Hypothesis

The bug is **NOT**:
- RCT forward transform (verified correct)
- RCT inverse transform (verified correct)
- DWT implementation (grayscale works perfectly)
- EBCOT bit-plane coding (grayscale works perfectly)
- MCT flag (setting MCT=0 makes it worse)

The bug is **LIKELY**:
- **Subband coefficient storage/indexing** for multi-component images
- **Packet body data extraction** when components have opposite phase
- **Codeblock boundary handling** starting at 16×16 images
- **Resolution level grid calculations** for RGB vs grayscale

### Critical Clue

At 8×8, after 3 DWT levels:
- 8 → 4 → 2 → **1×1 LL subband** → PASS ✅

At 16×16, after 3 DWT levels:
- 16 → 8 → 4 → **2×2 LL subband** → FAIL ❌

The transition from 1×1 to 2×2 LL subband triggers the bug. This suggests:
1. **Single-pixel subbands** are handled correctly
2. **Multi-pixel subbands** have an indexing or storage issue for RGB

### Decoder Behavior

The decoder produces **consistent wrong values**:
- Expected: R=255, G=0, B=255 (magenta)
- Got: R=108, G=146, B=108 (grayish-green)

This suggests data is being read from the **wrong location** or **wrong component** during decoding, not random corruption.

## Test Results Matrix

| Size | DWT=3 | Pattern | Result |
|------|-------|---------|--------|
| 2×2 | N/A | G inverted | Skipped (too small) |
| 4×4 | N/A | G inverted | Skipped (too small) |
| 8×8 | 3 | G inverted | ✅ PASS (MAE=0) |
| 16×16 | 3 | G inverted | ❌ FAIL (MAE=18.5) |
| 32×32 | 3 | G inverted | ❌ FAIL (MAE=7.1) |
| 64×64 | 3 | G inverted | ❌ FAIL (MAE=4.0) |
| 128×128 | 3 | G inverted | ❌ FAIL (MAE=2.9) |
| 256×256 | 3 | G inverted | ❌ FAIL (MAE=2.4) |
| 512×512 | 3 | G inverted | ❌ FAIL (MAE=2.2) |

| Pattern | 128×128 DWT=3 | Result |
|---------|---------------|--------|
| R=G=B checkerboard | ✅ PASS (MAE=0) |
| R/G/B gradients | ✅ PASS (MAE=0) |
| R 8×8, G 16×16, B solid | ✅ PASS (MAE=0) |
| R=B checkerboard, G inverted | ❌ FAIL (MAE=2.9) |

## Next Investigation Steps

1. **Compare packet data for 8×8 vs 16×16**
   - Dump packet headers and body sizes
   - Check if component ordering is correct
   - Verify subband dimensions match expectations

2. **Add debug output to encoder subband extraction**
   - Print LL/HL/LH/HH subband sizes for each component
   - Check if coefficients are being stored in correct component buffers
   - Verify codeblock grid calculations

3. **Test with OpenJPEG encoder**
   - Encode same test pattern with OpenJPEG
   - Compare:
     - Codestream structure
     - Packet count and sizes
     - Subband coefficient values
   - Decode our encoder's output with OpenJPEG to isolate encoder vs decoder

4. **Examine component loop in encoder**
   - Line 281-282 in encoder.rs: `for (comp_idx, mut comp_data) in component_data.into_iter().enumerate()`
   - Check if RCT-transformed data is correctly assigned to component buffers
   - Verify Y/U/V are in the right order after transform

5. **Check decoder's component reconstruction**
   - Verify inverse RCT is reading Y/U/V from correct component indices
   - Check if IDWT is being applied to the correct data

## Files Created

### Test Files
- `tests/test_rgb_blocksize_dwt.rs` - Block size vs DWT level matrix
- `tests/test_grayscale_checkerboard_dwt.rs` - Proves grayscale works
- `tests/test_rgb_component_isolation.rs` - Tests each RGB channel separately  
- `tests/test_rgb_different_channels.rs` - Tests different patterns per channel
- `tests/test_rct_debug.rs` - RCT transform validation (2×2)
- `tests/test_larger_images.rs` - Tests 64-512px images
- `tests/test_2x2_inverted_g.rs` - Tests 2×2 at all DWT levels
- `tests/test_minimum_failing_size.rs` - **Found 16×16 threshold**

### Documentation
- `docs/RGB_CHECKERBOARD_BUG.md` - Initial bug report
- `docs/RGB_BUG_INVESTIGATION.md` - This document

## Code Locations

### Encoder
- `src/jpeg2000/encoder.rs:118` - MCT flag set to 1 for 3+ components
- `src/jpeg2000/encoder.rs:261-275` - RCT forward transform applied
- `src/jpeg2000/encoder.rs:281` - Component iteration loop

### Decoder
- `src/jpeg2000/image.rs:227-244` - RCT inverse transform applied
- `src/jpeg2000/decoder.rs` - Main decoding logic

## Conclusion

We have a **reproducible, size-dependent RGB encoding bug** that only occurs when:
1. Image is 16×16 pixels or larger
2. RGB components have opposite phase relationships
3. Using DWT level 3 (or specific DWT levels related to image size)

The bug is NOT in the RCT transform itself, but in how **multi-component subband data is handled** when subbands exceed a certain size (2×2 pixels).

**Priority**: HIGH - This blocks production use of RGB JPEG 2000 encoding

**Next Step**: Add comprehensive debug logging to encoder's subband extraction and packet creation to trace where component data gets misaligned.
