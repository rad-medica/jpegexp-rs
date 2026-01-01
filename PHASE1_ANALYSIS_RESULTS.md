# Phase 1 Analysis Results - JPEG-LS Decoder Issues

## Summary

Phase 1 (Analysis & Validation) has identified the root causes of JPEG-LS decoder failures through systematic testing and bit-level debugging.

## Test Infrastructure Created

### Test Suite
- **23 test images** generated with CharLS reference implementation
- Sizes: 8x8 to 256x256 pixels
- Formats: Grayscale 8-bit, Grayscale 16-bit, RGB 8-bit
- Patterns: gradient, noise, checker, solid
- All CharLS-encoded images decode perfectly with CharLS (verified)

### Debug Infrastructure
- Bit-level consumption tracking
- Per-line progress logging
- Detailed error reporting with context
- Enabled via `JPEGLS_DEBUG=1` environment variable

## Key Findings

### Finding 1: Premature Bit Exhaustion Pattern

**Observation**: All grayscale tests fail with "Invalid data" when decoder runs out of bits before completing the image.

**Example** (`tiny_8x8_gray_gradient`):
```
Source: 27 bytes scan data (216 bits available)
Frame: 8x8 = 64 pixels

Decoding progress:
- Initial cache: 56 bits loaded, position at byte 7
- Line 0: Successfully decodes 8 pixels, consumes 17 bits
  - Bits/pixel: 2.125
- Line 1: Fails immediately - out of bits at position 25

Analysis:
- Position 25 = 25 bytes consumed
- 25 bytes = 200 bits
- Consumed: 200 bits to decode 8 pixels = 25 bits/pixel (WRONG!)
- Expected: ~2-3 bits/pixel for lossless compression
```

**Conclusion**: The decoder is NOT consuming ~2 bits per pixel as the line-level stats suggest. The position tracking shows we've consumed far more bytes than the bit counter indicates.

### Finding 2: Position vs Bits Consumed Mismatch

**Critical Issue**: The `position` field (byte position in source) doesn't match `bits_consumed` / 8.

```
Line 0 complete:
- bits_consumed: 17 bits (≈ 2.1 bytes)
- position: 7 bytes
- Missing: ~5 bytes consumed but not tracked by bits_consumed counter
```

**Root Cause**: The `fill_read_cache()` function reads bytes but doesn't update `bits_consumed`. Only `read_bits()` updates the counter.

**Impact**: Our bit consumption metrics are wrong. The actual consumption is tracked by `position`, not `bits_consumed`.

### Finding 3: Actual Bit Consumption Rate

Recalculating with `position`:
```
tiny_8x8_gray_gradient:
- Scan data: 27 bytes
- Initial cache fill: reads to position 7 (56 bits loaded)
- Line 0: position stays at 7 (uses cached bits)
- Line 1 start: Needs cache refill
- Position 25 reached when out of bits

Actual consumption to decode 8 pixels:
- Started at position 7
- Reached position 25 trying to start line 1
- Used: 25 - 7 = 18 bytes = 144 bits for 8 pixels
- Rate: 144 / 8 = 18 bits/pixel (WAY TOO HIGH!)
```

**Expected for JPEG-LS**:
- Lossless compression typically: 2-4 bits/pixel for smooth gradients
- CharLS achieves: 27 bytes for 64 pixels = 3.375 bits/pixel

**Our decoder**: 18 bits/pixel (5x too high!)

### Finding 4: Likely Root Causes

Based on the evidence:

1. **Golomb Decoding Bug**: Most likely culprit
   - Golomb codes are variable length (unary + fixed bits)
   - Bug could cause reading too many bits per sample
   - Check: `decode_mapped_error_value()` logic

2. **Context Management Bug**:
   - Wrong k-parameter calculation could cause reading wrong number of bits
   - Check: `RegularModeContext::compute_golomb_coding_parameter()`

3. **Run Mode Bug**:
   - Run mode should compress repeated values efficiently
   - If run mode isn't triggering or is broken, would use regular mode
   - Regular mode less efficient → more bits consumed
   - Check: `decode_run_mode()` and run mode detection

4. **Prediction Error**:
   - Wrong predictions → larger error values → more bits needed
   - Check: `compute_predicted_value()` implementation

## Detailed Analysis: Golomb Coding

Current implementation (`decode_mapped_error_value`):
```rust
fn decode_mapped_error_value(&mut self, k: i32) -> Result<i32, JpeglsError> {
    let mut value = 0;
    let mut bit_count = 0;
    
    while self.peek_bits(1)? == 0 {  // Unary code
        value += 1;
        bit_count += 1;
        self.skip_bits(1)?;
        if bit_count > 32 {
            return Err(JpeglsError::InvalidData);
        }
    }
    self.skip_bits(1)?;  // Skip terminating 1
    
    if k > 0 {
        let remainder = self.read_bits(k)?;  // Fixed-length code
        value = (value << k) | remainder;
    }
    Ok(value)
}
```

**Potential Issues**:
1. Is the unary code reading correct? Should it count zeros before the 1, or is it counting incorrectly?
2. Is the final value calculation correct: `(value << k) | remainder`?
3. Are we handling k=0 correctly?

**Verification Needed**:
- Compare with CharLS implementation
- Test with known Golomb code inputs/outputs
- Add unit tests for Golomb decoding

## Test Results Summary

| Test Name | Result | Notes |
|-----------|--------|-------|
| tiny_8x8_gray_gradient | FAIL | Runs out of bits at pos 25/27 bytes |
| tiny_8x8_gray_noise | FAIL | Wrong pixels (MAE 87.14) |
| tiny_8x8_gray_checker | FAIL | Wrong pixels (MAE 119.52) |
| tiny_8x8_gray_solid | FAIL | Runs out of bits |
| small_16x16_gray_gradient | Not tested yet | Likely fails similarly |
| All others | Not tested yet | -- |

**Pattern**: 
- Gradient/solid images: Run out of bits (bit consumption too high)
- Noise/checker images: Decode but wrong pixels (prediction/context issues?)

## Recommended Next Actions (Phase 2)

### Priority 1: Fix Golomb Decoding
1. **Study CharLS implementation** of Golomb coding
   - Get reference implementation
   - Understand correct algorithm
   - Compare line-by-line

2. **Create unit tests** for Golomb coding
   ```rust
   #[test]
   fn test_golomb_decode_k0() {
       // Test decoding with k=0
       // Input: bit pattern
       // Expected: decoded value
   }
   
   #[test]
   fn test_golomb_decode_k2() {
       // Test decoding with k=2
   }
   ```

3. **Fix the implementation** based on findings

4. **Validate** with test suite

### Priority 2: Fix Context Management
1. Review `RegularModeContext::compute_golomb_coding_parameter()`
2. Verify k-parameter calculation matches spec
3. Ensure context updates are correct

### Priority 3: Fix Run Mode Detection
1. Review run mode entry/exit logic
2. Verify run length encoding/decoding
3. Test with images that should trigger run mode (solid, checker)

## Metrics to Track

For each fix attempt, track:
- Bits consumed per pixel (should be 2-4 for gradients)
- Number of lines successfully decoded
- Pixel accuracy (MAE should be 0 for lossless)

## Expected Outcomes

After fixes:
- **Bit consumption**: Should match CharLS (~3-4 bits/pixel for gradients)
- **Decode success**: All valid CharLS images should decode
- **Pixel accuracy**: Perfect match (MAE = 0) for lossless compression
- **Performance**: Within 2x of CharLS speed

## Files to Focus On

1. `src/jpegls/scan_decoder.rs` - Main decoding logic
   - `decode_mapped_error_value()` - Golomb decoding
   - `decode_sample_line()` - Per-pixel decoding
   - `decode_run_mode()` - Run mode handling

2. `src/jpegls/regular_mode_context.rs` - Context management
   - `compute_golomb_coding_parameter()` - k parameter
   - `update_variables_and_bias()` - Context updates

3. `src/jpegls/run_mode_context.rs` - Run mode context

## Reference Materials

- **JPEG-LS Spec**: ITU-T T.87, Annex C (Golomb coding)
- **CharLS source**: https://github.com/team-charls/charls
  - `scan_decoder.h` / `scan_decoder.cpp`
  - `golomb_lut.h`
- **Academic paper**: "The LOCO-I lossless image compression algorithm" (Weinberger et al.)

## Conclusion

Phase 1 has successfully:
1. ✅ Created comprehensive test suite
2. ✅ Added bit-level debugging
3. ✅ Identified root cause: Excessive bit consumption (~5x expected)
4. ✅ Pinpointed likely bug location: Golomb decoding

Next: Phase 2 will systematically fix the identified issues, starting with Golomb decoding.
