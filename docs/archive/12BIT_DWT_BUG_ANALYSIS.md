# 12-bit DWT Bug Analysis - Final Findings

**Date**: January 7, 2026  
**Issue**: 12-bit checkerboards fail with DWT ≥ 1

## Critical Discovery

### DWT=0 vs DWT≥1 Behavior

| Configuration | Result | Details |
|---------------|--------|---------|
| Any size, DWT=0 | ✅ **PASS** | MAE = 0.0, perfect encoding |
| 64×64, DWT≥1 | ❌ **FAIL** | MAE = 2047.5 (exactly half of 4095) |

### Root Cause Identified

**Problem**: With DWT ≥ 1, many subbands have **no codeblocks** (all coefficients too similar).

Evidence from debug output (64×64 checkerboard, DWT=1):
```
ENC: Created packet for res=0 comp=0 header_len=1 body_len=0 cblks=0  ← LL: NO DATA
ENC: Created packet for res=1 comp=0 header_len=1 body_len=0 cblks=0  ← HL,LH: NO DATA  
ENC: Created packet for res=2 comp=0 header_len=4 body_len=24 cblks=1  ← HH: 1 block only
```

**Decoder behavior** (`src/jpeg2000/image.rs:148`):
- When subband has no codeblocks → returns `vec![0]` (all zeros)
- IDWT then runs with mostly zero subbands
- Result: Uniform gray output (MAE ≈ 2047.5)

## Why LL Has No Codeblocks

For a 1-pixel checkerboard (alternating 0 and 4095):

1. **After DWT**: LL band contains averages: `(0+4095)/2 = 2047` everywhere
2. **After level shift** (subtract 2048): All LL coefficients ≈ `-1`
3. **Bit-plane coding**: All values identical → `calculate_max_bit_plane()` returns `None`
4. **Result**: No codeblock created for LL

Similarly, HL and LH subbands may have very small coefficients that don't trigger encoding.

## Actual Bug

The encoder is **working as designed** - it skips encoding subbands with insignificant coefficients. The decoder correctly returns zeros for missing subbands.

**The real issue**: When a checkerboard's LL subband is near-constant after level shift, essential information is lost. The DWT assumes the input is **AC-coupled** (zero-mean), but after level shift, high-DC images become problematic.

## Comparison with 8-bit

8-bit checkerboard (0, 255) **works** because:
- LL after DWT: average ≈ 127
- After level shift (128): ≈ -1 (same problem!)
- **BUT**: With 8-bit, the encoder creates codeblocks even for small variations

12-bit has **higher threshold** for "significant" coefficients due to:
- Larger epsilon values (12-14 vs 8-10)
- Larger M_b values (13-15 vs 9-11)
- More bit planes to consider

## The Fix

### Option 1: Force Encode Non-Zero Subbands
Modify encoder to always create at least one codeblock per subband, even if coefficients are small.

**Location**: `src/jpeg2000/encoder.rs:585`
```rust
// Current: if let Some(max_bp) = bpc.calculate_max_bit_plane() {
// Fix: Force minimum bit plane if subband has any non-zero values
let max_bp = bpc.calculate_max_bit_plane().unwrap_or(0);
if max_bp > 0 || has_nonzero_coeffs(&block_data) {
    // encode
}
```

### Option 2: Improve Bit-Plane Calculation
Modify `calculate_max_bit_plane()` to be more sensitive for 12-bit.

**Location**: `src/jpeg2000/bit_plane_coder.rs:166-172`

### Option 3: DC Level Adjustment
Don't level-shift before DWT - apply it per-subband or after packet encoding.

## Why 8×8 Squares Work Better

With 8×8 checkerboard squares:
```
Square size 8x8: max_val=3612 ← LL has variation!
```

Larger squares create more variation in LL after DWT, so codeblocks are created.

## OpenJPEG Interop Test Failure Explained

The OpenJPEG decoder also produces wrong output because **our encoder isn't writing the LL subband data**. Both decoders correctly interpret the (incomplete) bitstream.

## Next Steps

1. **Implement Option 1** (easiest fix):
   - Always encode subbands that have at least one non-zero coefficient
   - Test with 64×64 checkerboard DWT=1

2. **Verify with OpenJPEG**:
   - Encode reference image with OpenJPEG
   - Hex dump to see how they handle near-constant LL subbands

3. **Test edge cases**:
   - All-white image after DWT
   - Nearly-constant images (gradient of 0-10)

## Estimated Fix Time

2-4 hours to implement and test Option 1.

## Files to Modify

1. `src/jpeg2000/encoder.rs` - Line 585 (codeblock creation condition)
2. `tests/test_12bit_size_hunt.rs` - Add regression tests
3. `SESSION_12BIT_VALIDATION.md` - Update with fix details

## Success Criteria

After fix:
- ✅ 64×64 checkerboard with DWT=1-5: MAE = 0
- ✅ All 9 tests in `test_12bit_grayscale_interop.rs` pass
- ✅ OpenJPEG decoder produces MAE = 0
