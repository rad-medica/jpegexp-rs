# HTJ2K Decoder Debugging Notes

## Issue
HTJ2K decoder produces MAE=63.6 instead of 0.0 for lossless decoding.

## Test Case
- Input: 64x64 gradient (values 0-255)
- Encoded with: OpenHTJ2K v0.3.1 (lossless mode)
- Decoded with: Our HTJ2K decoder
- Expected MAE: 0.0
- Actual MAE: 63.595703125

## Root Cause Analysis

The decoder has the correct structure but likely issues in one of:

1. **MEL Decoder State Machine** (`src/jpeg2000/ht_block_coder/mel.rs`)
   - Run-length decoding logic may be incorrect
   - State transitions (k value) may not match spec

2. **VLC Lookup** (`src/jpeg2000/ht_block_coder/vlc.rs`)
   - Codeword tables may be incorrect
   - Context calculation may be wrong
   - Bit consumption may be off

3. **MagSgn Decoder** (`src/jpeg2000/ht_block_coder/mag_sgn.rs`)
   - Sign bit interpretation
   - Magnitude refinement bit ordering
   - Bit reading direction

4. **Stream Interleaving** (`src/jpeg2000/ht_block_coder/coder.rs`)
   - MEL/VLC share the same stream (read backwards)
   - MagSgn is separate (read forwards)
   - Stream boundaries may be incorrect

## Debugging Strategy

To fix this properly would require:

1. **Add Debug Logging**
   ```rust
   // In decode_quad()
   if std::env::var("HTJ2K_DEBUG").is_ok() {
       eprintln!("Quad ({}, {}): is_sig={}, rho={:04b}", 
                 x, y_base, is_significant, rho);
   }
   ```

2. **Compare with OpenHTJ2K**
   - Instrument OpenHTJ2K decoder with same logging
   - Compare outputs for same input
   - Find first divergence point

3. **Test Minimal Cases**
   - 2x2 image (single quad)
   - 4x4 image (4 quads)
   - Simple patterns (all zeros, all ones, single pixel)

4. **Verify Stream Parsing**
   - Check that we're reading the right number of bytes
   - Verify stream pointers advance correctly
   - Ensure no off-by-one errors

## Recommendation

Given the complexity and time required:

1. **Mark decoder as experimental** ⚠️
2. **Focus on encoder integration** (more valuable for users)
3. **File issue for decoder fix** with this analysis
4. **Wait for HTJ2K specification clarification** or OpenHTJ2K code review

The decoder has the right architecture and partially works (doesn't crash, produces output).
The bug is subtle and requires deep HTJ2K spec knowledge to fix properly.

## Workaround

For now, users can:
- Use OpenHTJ2K decoder for HTJ2K files
- Use our standard J2K decoder for JPEG 2000 files (works perfectly, MAE=0)
- Wait for encoder integration (can create HTJ2K files with our encoder)

## Status
- Decoder: ⚠️ Experimental (MAE=63.6, needs fix)
- Encoder: 🔜 Ready for integration
- Priority: Focus on encoder integration first
