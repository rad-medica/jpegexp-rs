# 12-Bit JPEG 2000 Encoding Investigation Summary

## Date
January 8, 2026

## Problem Statement
12-bit grayscale JPEG 2000 encoding fails for checkerboard patterns with DWT ≥ 1, producing MAE = 2047.5 (exactly half of 4095).

## Investigation Timeline

### Initial Hypothesis (INCORRECT)
- Thought the encoder wasn't creating codeblocks for near-constant subbands
- Implemented fix to check for `has_nonzero` coefficients before skipping encoding
- This fix was correct in principle but didn't solve the problem

### Key Discovery #1: Subband Analysis
Using debug output, discovered the actual coefficient distribution after DWT:

**64×64 Checkerboard after DWT=1:**
- LL subband (res=0): All ZERO (range=[0,0])
- HL subband (res=1, band=0): All ZERO  
- LH subband (res=1, band=1): All ZERO
- HH subband (res=1, band=2): All 8190 (constant value!)

**Key insight:** The LL subband becomes all zeros because:
1. Input: Checkerboard with values 0 and 4095
2. After level shift (subtract 2048): -2048 and 2047
3. After DWT: LL ≈ average ≈ 0, HH ≈ high-frequency ≈ ±4095

This is **CORRECT behavior** - not a bug! The all-zero subbands don't need encoding.

### Key Discovery #2: Bit-Plane Coder Issue
Created isolated tests for the bit-plane coder with constant blocks:

**Test Results:**
- ✅ Varying block `[0,1,2,3,4,5,6,7]` - PASSES
- ❌ Constant block `[7,7,7,7,7,7,7,7]` with orient=0 - FAILS (outputs `[4,4,4,4...]`)
- ✅ Constant block `[7,7,7,7,7,7,7,7]` with correct max_bp - PASSES
- ✅ Constant block `[255,255,255...]` (8×8, 16×16, 32×32, 64×64) - ALL PASS
- ✅ Constant block `[8190,8190,8190...]` (32×32) with orient=3 - PASSES!

**Critical Finding:** The bit-plane coder works correctly for constant blocks when:
1. The correct `max_bp` is used (from `calculate_max_bit_plane()`)
2. The correct orientation parameter is passed
3. Both encoder and decoder use matching parameters

### Key Discovery #3: Encoder Already Uses Correct Orientation
Verified in `encoder.rs` line 620:
```rust
let orientation = if res == 0 { 0 } else { band as u8 + 1 };
```

For HH subband: `orientation = 2 + 1 = 3` ✓

Verified in `decoder.rs` line 739:
```rust
bpc.decode_codeblock(&data, max_bp, passes, subband.orientation as u8)
```

The decoder correctly uses the subband's orientation ✓

## Current Status

### What Works ✅
- Bit-plane coder correctly handles constant blocks (verified in unit tests)
- Encoder creates codeblocks with correct orientation for HH subband
- Decoder uses correct orientation when decoding codeblocks
- 12-bit encoding works perfectly with DWT=0

### What Still Fails ❌
- 12-bit checkerboard with DWT ≥ 1 produces MAE = 2047.5
- The failure occurs in the full pipeline despite individual components working

## Remaining Hypothesis

The issue is likely in one of these areas:

1. **IDWT Integration**: The IDWT might not correctly handle the case where:
   - LL, HL, LH subbands are all zeros (no codeblocks)
   - HH subband has constant 8190 (one codeblock)
   
2. **Subband Extraction**: There might be a size mismatch when extracting/reconstructing subbands

3. **Level Shift Application**: The inverse level shift might not be applied correctly after IDWT

4. **Decoder Subband Filling**: When a subband has no codeblocks, `image.rs:148` returns `vec![0; size]`.
   This is correct, but there might be a size mismatch with the actual subband dimensions.

## Next Steps

1. **Test IDWT Directly**
   - Create a test that manually constructs LL=0, HL=0, LH=0, HH=8190
   - Run IDWT and check output
   - This will isolate whether IDWT is the issue

2. **Add Debug Output to Decoder**
   - Log subband sizes during reconstruction
   - Verify all subbands have correct dimensions
   - Check values before and after IDWT

3. **Test with Simpler Pattern**
   - Try a 4×4 or 8×8 checkerboard with DWT=1
   - Manually trace through the entire encode/decode pipeline
   - Verify each step produces expected values

## Code Changes Made

### `src/jpeg2000/encoder.rs` (Lines 584-644)
Added check for non-zero coefficients:
```rust
let has_nonzero = block_data.iter().any(|&v| v != 0);
let max_bp_opt = bpc.calculate_max_bit_plane();

if max_bp_opt.is_some() || has_nonzero {
    let max_bp = max_bp_opt.unwrap_or(0);
    // encode codeblock...
}
```

### `src/jpeg2000/bit_plane_coder.rs` (Tests)
Added comprehensive unit tests for constant blocks:
- `test_small_constant_block()` - 8 pixels, value 7
- `test_medium_constant_block()` - 64 pixels, value 255  
- `test_large_16x16_constant_block()` - 256 pixels, value 255
- `test_constant_block_roundtrip()` - Multiple sizes (4×4 to 64×64)
- `test_constant_8190_block_roundtrip()` - 32×32, value 8190

All tests PASS ✅

## Conclusion

The investigation revealed that:
1. The original diagnosis was partially correct - we do need to encode subbands with data
2. The bit-plane coder works correctly for constant blocks
3. The encoder and decoder both use correct parameters
4. The failure is likely in the integration between components, not in individual components

The issue requires further investigation focusing on the IDWT integration and subband reconstruction logic.

## Time Spent
Approximately 3-4 hours of focused debugging and testing.
