# 12-bit JPEG 2000 Encoding Bug Fix

## Problem Summary

12-bit grayscale JPEG 2000 encoding was failing for checkerboard patterns when DWT decomposition levels ≥ 1, producing Mean Absolute Error (MAE) = 2047.5 (exactly half of 4095, the constant mid-gray value). The issue affected high-frequency patterns with DWT enabled, while DWT=0 worked perfectly.

## Root Cause

The bug was in the **packet header encoding** of the number of coding passes. Specifically, in `src/jpeg2000/packet.rs`, the `write_coding_passes()` function was incomplete for passes ≥ 37:

```rust
// INCORRECT CODE (missing last line):
_ => {
    writer.write_bit(1);
    writer.write_bit(1);
    writer.write_bits(3, 2);
    writer.write_bits(31, 5);
    // MISSING: writer.write_bits((passes - 37) as u32, 5);
}
```

### Why This Caused the Problem

1. **High-frequency patterns** like checkerboards produce constant HH subbands after DWT
2. The constant value of 8190 (for a 12-bit checkerboard) requires encoding all bit-planes
3. This results in exactly **37 coding passes** (per EBCOT bit-plane coding)
4. The encoder would write the fixed value 31 in 5 bits instead of `(passes - 37) = 0`
5. The decoder would then misinterpret subsequent bits, reading:
   - **lblock_inc = 1** (instead of 0)
   - **data_len = 384** (instead of 11)
6. The decoder would try to read 384 bytes when only 11 were written → **InvalidData error**

## The Fix

Added the missing line to properly encode passes ≥ 37:

```rust
// CORRECT CODE (file: src/jpeg2000/packet.rs, lines 217-223):
_ => {
    writer.write_bit(1);
    writer.write_bit(1);
    writer.write_bits(3, 2);
    writer.write_bits(31, 5);
    writer.write_bits((passes - 37) as u32, 5);  // ← ADDED THIS LINE
}
```

This follows the JPEG 2000 standard (ISO/IEC 15444-1, Table B.4) for encoding the number of coding passes:
- 1 pass: `0`
- 2 passes: `10`
- 3-5 passes: `11` + 2 bits
- 6-36 passes: `1111` + 5 bits
- 37+ passes: `1111` + `11111` + 5 bits for (passes - 37)

## Test Results

### Before Fix
```
64x64 DWT=1: MAE=2047.500000 ❌ FAIL
64x64 DWT=2: MAE=2047.500000 ❌ FAIL
64x64 DWT=3: MAE=2047.500000 ❌ FAIL
64x64 DWT=4: MAE=2047.500000 ❌ FAIL
64x64 DWT=5: MAE=2047.500000 ❌ FAIL
```

### After Fix
```
64x64 DWT=0: MAE=0.000000 ✅ PASS
64x64 DWT=1: MAE=0.000000 ✅ PASS
64x64 DWT=2: MAE=0.000000 ✅ PASS
64x64 DWT=3: MAE=0.000000 ✅ PASS
64x64 DWT=4: MAE=0.000000 ✅ PASS
64x64 DWT=5: MAE=0.000000 ✅ PASS
```

All test sizes (8×8, 16×16, 32×32, 64×64) now pass with MAE=0 at all DWT levels.

## Investigation Process

### What We Tried (That Didn't Work)
1. ❌ Initially suspected the encoder was skipping codeblocks for near-constant subbands
2. ❌ Thought the bit-plane coder had issues with constant values
3. ❌ Investigated DWT/IDWT correctness (all were actually working fine)

### What Led to the Solution
1. ✅ Created comprehensive unit tests proving bit-plane coder works for constant blocks (including value 8190)
2. ✅ Added DWT tests proving IDWT correctly reconstructs checkerboards from constant HH subband
3. ✅ Added detailed debug logging to encoder and decoder
4. ✅ Discovered the packet header length mismatch:
   - Encoder writes: `lblock=3, lbits=8, len=11`
   - Decoder reads: `lblock=4, lbits=9, len=384`
5. ✅ Traced the issue to comma code reading 1 instead of 0
6. ✅ Found the missing line in `write_coding_passes()`

## Files Modified

1. **`src/jpeg2000/packet.rs`** (line 222)
   - Added missing write for passes ≥ 37
   - Added debug logging for encoder/decoder packet header details

2. **`src/jpeg2000/encoder.rs`** (lines 327-330, 584-644)
   - Added debug output for codeblock encoding
   - Added logic to encode constant-valued subbands

3. **`src/jpeg2000/image.rs`** (lines 159-185)
   - Added debug output for subband reconstruction statistics

4. **`src/jpeg2000/bit_plane_coder.rs`** (lines 523-695)
   - Added 6 new unit tests for constant block encoding/decoding

5. **`src/jpeg2000/dwt.rs`** (lines 398-520)
   - Added 2 new tests for checkerboard DWT/IDWT roundtrip

6. **`tests/test_12bit_debug.rs`** (new file)
   - Created focused debug tests for 4×4 and 8×8 checkerboards

## Key Insights

1. **Constant HH subbands are expected**: After DWT of a checkerboard pattern, all subbands except HH are zero, and HH is constant (value -8190). This is mathematically correct.

2. **37 passes is the trigger**: Constant blocks with max_bp=12 produce exactly 37 coding passes, which hit the edge case in the encoding table.

3. **The bug was hidden**: It only affected patterns that:
   - Use DWT (levels ≥ 1)
   - Produce constant high-valued subbands
   - Result in exactly 37+ coding passes

4. **Unit tests saved us**: Creating isolated unit tests for each component (DWT, IDWT, bit-plane coder) proved they all worked correctly, narrowing the search to the integration points.

## Prevention

To prevent similar bugs:
1. **Test edge cases**: Always test encoding with 1, 2, 3, 6, 37, and 64 passes
2. **Add packet header roundtrip tests**: Test encoding/decoding packet headers with various pass counts
3. **Enable debug mode in CI**: Run some tests with `J2K_DEBUG=1` to catch bit-stream mismatches early

## Date
January 8, 2026
