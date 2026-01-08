# RGB Checkerboard Bug Investigation

## Date
2026-01-08

## Summary

RGB checkerboard encoding has a specific failure pattern at certain DWT levels that is NOT present in grayscale encoding.

## Test Results

### RGB Encoding (128×128 image)

| Block Size | DWT=2 | DWT=3 | DWT=4 | DWT=5 |
|------------|-------|-------|-------|-------|
| 4×4        | 6.3   | ✅    | ✅    | ✅    |
| 8×8        | ✅    | **2.9**   | ✅    | ✅    |
| 16×16      | ✅    | ✅    | **2.0**   | ✅    |
| 32×32      | ✅    | ✅    | ✅    | ✅    |

**Pattern:** Failure occurs when DWT level equals `log2(image_size / block_size) - 1`
- 128÷4 = 32, log2(32) = 5, fails at DWT=2 (5-3)
- 128÷8 = 16, log2(16) = 4, fails at DWT=3 (4-1)  
- 128÷16 = 8, log2(8) = 3, fails at DWT=4 (3-1)

### Grayscale Encoding (128×128 image)

| Block Size | DWT=2 | DWT=3 | DWT=4 | DWT=5 |
|------------|-------|-------|-------|-------|
| 4×4        | ✅    | ✅    | ✅    | ✅    |
| 8×8        | ✅    | ✅    | ✅    | ✅    |
| 16×16      | ✅    | ✅    | ✅    | ✅    |
| 32×32      | ✅    | ✅    | ✅    | ✅    |

**All pass!** This confirms the issue is RGB-specific.

## Key Observations from Debug Output

### Component 0 (appears to be constant/zero for checkerboard RGB)
```
EXTRACT: Res 0 LL 16x16 at (0,0)
BLOCK[0,0] res=0 band=0 range=[-1,-1] unique=1 nonzero=true first_few=[-1, -1, -1, -1, -1, -1, -1, -1, -1, -1]
ENC: CB[0,0] res=0 band=0 orient=0 max_val=1 max_bp=0 has_nonzero=true
```

Component 0 has:
- All values = -1 (constant LL subband)
- max_val=1, max_bp=0
- All resolution 1-3 subbands are zeros
- Only the LL band has data

### Components 1 & 2 (actual R and B channels with checkerboard data)
```
EXTRACT: Res 0 LL 16x16 at (0,0)
ENC: CB[0,0] res=0 band=0 orient=0 max_val=587 max_bp=9 has_nonzero=true
```

Components 1 and 2 have:
- Complex coefficient values (max_val=587, 508, 440, 377, etc.)
- All subbands have data (HH, HL, LH at each resolution)
- Full 25-28 coding passes

### Resolution 3, Subband 2 (HH) Issue
For components 1 and 2:
```
EXTRACT: Res 3 subband 2 64x64 at (64,64)
BLOCK[0,0] res=3 band=2 range=[-255,255] unique=3 nonzero=true first_few=[0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
ENC: CB[0,0] res=3 band=2 orient=3 max_val=255 max_bp=7 has_nonzero=true
Enc CB[0,0] band=2 len=183 max_bp=7 passes=22
```

This is the **highest resolution HH subband** (the finest detail). It has:
- range=[-255,255] (full 8-bit range!)
- unique=3 (only 3 unique values: likely -255, 0, +255)
- first_few all zeros but has_nonzero=true

This subband represents the **sharpest edges** from the checkerboard pattern.

## Hypothesis

The failure pattern suggests an issue with **multi-component encoding when high-frequency content aligns with specific resolution levels**.

### Possible Root Causes

1. **Packet Ordering**: The encoder creates packets in `RPCL` order (Resolution-Position-Component-Layer). At the failing DWT level, packet sequencing for component 1 or 2 might be incorrect.

2. **Subband Size Calculation**: When the checkerboard block size aligns with a resolution level, subband dimensions might be calculated incorrectly for RGB (but not grayscale).

3. **Multi-Component Quantization**: The QCD (quantization default) parameters might not be correctly applied across components when high-frequency subbands have extreme values.

4. **Codeblock Boundary**: At the critical resolution level, codeblock boundaries in RGB might span incorrect regions of the wavelet coefficients.

5. **Component Transform**: If color transform is applied (RGB→YCbCr), the inverse might be failing at specific frequencies. However, the encoder should use RGB directly for lossless mode.

## Why Grayscale Works

Grayscale encoding:
- Single component (no component ordering issues)
- No color transform complications
- Simpler packet structure

The DWT, EBCOT, and packet encoding work correctly for grayscale at ALL DWT levels, suggesting the core algorithms are sound.

## Next Steps

1. **Verify Color Transform**: Check if `set_mct(false)` or equivalent is set for lossless RGB
2. **Compare Packets**: Save packet headers for failing vs working cases
3. **Check Component Iteration**: Review packet creation loop in encoder for off-by-one or ordering errors
4. **Test with OpenJPEG Encoder**: Encode the same RGB checkerboard with OpenJPEG and compare:
   - Packet count and sizes
   - Subband coefficient ranges
   - Codeblock organization

5. **Single Component at a Time**: Encode R, G, B channels separately as grayscale to see if any specific component fails

## Test Files Created

- `tests/test_rgb_blocksize_dwt.rs` - Matrix test showing failure pattern
- `tests/test_grayscale_checkerboard_dwt.rs` - Proves grayscale works
- `tests/test_rgb_comprehensive.rs` - Shows gradients work but checkerboards fail

## Error Metrics

Failing cases show:
- MAE ≈ 2-6 (should be 0 for lossless)
- Approximately 40-50% of pixels have errors
- Errors appear to be off-by-one or small quantization issues

This suggests data is being **read from slightly wrong locations** during decode, not completely corrupted.
