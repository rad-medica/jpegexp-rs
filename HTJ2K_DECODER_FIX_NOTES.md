# HTJ2K Decoder Bug Investigation

**Date**: January 9, 2026  
**Status**: Root cause identified, partial fix implemented  
**Blocking**: DICOM .201/.203 compliance  

## Problem Summary

HTJ2K decoder produces 4,087 pixel mismatches (out of 4,096 total) in lossless roundtrip tests.
All 4 comprehensive tests failing:
- `test_htj2k_8bit_gray` (64x64)
- `test_htj2k_12bit_gray` (64x64)
- `test_htj2k_16bit_gray` (64x64)
- `test_htj2k_8bit_rgb` (64x64x3)

## Root Cause Identified

### Issue 1: Missing `emb_1` (E_1) Parameter ✅ FIXED
**File**: `src/jpeg2000/ht_block_coder/vlc.rs` line 81  
**Problem**: VLC decoder ignored `e_1` field with comment:
```rust
// We ignore e_1 for now in the return signature
```

**Fix Applied**:
- Updated `decode_vlc()` to return 5 values: `(rho, u_off, e_k, e_1, bits)`
- Updated `coder.rs` to capture and use `emb_1_0` and `emb_1_1`

### Issue 2: Incorrect Magnitude Reconstruction ❌ NOT FIXED
**File**: `src/jpeg2000/ht_block_coder/coder.rs` lines 154-199  
**Problem**: Magnitude reconstruction doesn't match ISO/IEC 15444-15 formula

**Current (WRONG)**:
```rust
let m = u_val.saturating_sub(bit_k);
let mut v = 0u32;
// Read m bits
v |= (known_1 as u32) << m;
let mag = v;  // TOO SIMPLE!
```

**Expected (from OpenHTJ2K lines 607-616)**:
```cpp
// Step 1: Calculate m (bits to read from MagSgn)
m_quads[i] = sigma_quads[i] * U0 - ((emb_k_0 >> i) & 1);

// Step 2: Read m bits from MagSgn + add known_1 bit
v_quads[i] = msval[i] & ((1 << m_quads[i]) - 1U);
v_quads[i] |= known_1[Q0] << m_quads[i];

// Step 3: Reconstruct magnitude (CRITICAL FORMULA)
if (m_quads[i] != 0) {
    mu_quads[i] = v_quads[i] + 2;      // Add 2
    mu_quads[i] |= 1;                   // Force LSB to 1
    mu_quads[i] <<= pLSB - 1;           // Shift left (for lossy)
    mu_quads[i] |= (v_quads[i] & 1) << 31;  // Embed sign in bit 31
}
```

### Issue 3: Missing `pLSB` Context
**File**: `src/jpeg2000/ht_block_coder/coder.rs`  
**Problem**: No `pLSB` (plane of least significant bit) parameter passed to decoder

The OpenHTJ2K decoder signature is:
```cpp
void ht_cleanup_decode(j2k_codeblock *block, const uint8_t &pLSB, ...)
```

Our decoder doesn't have `pLSB` context, which is critical for:
- Lossy vs lossless mode differentiation
- Magnitude shift calculation (`mu <<= pLSB - 1`)

### Issue 4: Sign Bit Encoding
**Problem**: OpenHTJ2K embeds sign in bit 31 of the coefficient, not via separate multiplication

**OpenHTJ2K**:
```cpp
mu_quads[i] |= (v_quads[i] & 1) << 31;  // Sign in bit 31
*mp0++ = static_cast<int>(mu_quads[0]);  // Cast to signed int
```

**Our Code**:
```rust
let signs = [0i32; 4];
// ... read signs separately from MagSgn stream ...
block.coefficients[py * w + px] = (mag as i32) * signs[i];  // Separate multiplication
```

This might work, but it's not following the reference implementation's approach.

## Attempted Fixes

### Attempt 1: Simple offset addition
```rust
if bit_k == 1 && u_val > 0 {
    mag += 1 << (u_val - 1);
}
```
**Result**: Still 4,087 mismatches

### Attempt 2: Adding emb_1 support
```rust
let known_1 = (emb_1 >> i) & 1;
v |= (known_1 as u32) << m;
let mag = v;
```
**Result**: Still 4,087 mismatches

## What's Needed for Complete Fix

1. ✅ **Get pLSB from block/codestream context**
   - Need to pass `pLSB` down through decoder chain
   - For lossless (`pLSB == 0`), formula simplifies

2. ✅ **Implement exact magnitude reconstruction**:
   ```rust
   if m > 0 {
       let mut mu = v + 2;
       mu |= 1;
       mu <<= pLSB.saturating_sub(1);  // For lossless (pLSB=0), this is 0
       // Handle sign bit
       mu |= (v & 1) << 31;
       mag = mu as i32;  // Signed cast
   }
   ```

3. ✅ **Verify sigma_quads calculation**:
   ```cpp
   m_quads[i] = sigma_quads[i] * U0 - ((emb_k_0 >> i) & 1);
   ```
   Need to understand what `sigma_quads` represents (significance pattern?)

4. ✅ **Cross-reference with ISO/IEC 15444-15 spec**
   - Section on HTJ2K block coding
   - Magnitude/sign reconstruction formulas
   - `pLSB` usage in different modes

## Comparison Test Strategy

1. Create minimal 2x2 test case
2. Encode with our encoder (which works)
3. Decode with both OpenHTJ2K and our decoder
4. Compare intermediate values:
   - `rho`, `emb_k`, `emb_1` from VLC
   - `u_q` from UVLC
   - `m_quads` (bits to read)
   - `v_quads` (read values)
   - `mu_quads` (final magnitudes)

## Files Modified (Partial Fix)

1. `src/jpeg2000/ht_block_coder/vlc.rs`:
   - Added `e_1` to `decode_vlc()` return value

2. `src/jpeg2000/ht_block_coder/coder.rs`:
   - Added `emb_1_0` and `emb_1_1` variables
   - Updated `reconstruct_quad()` signature to accept `emb_1`
   - Added `known_1` bit handling

## Next Steps

1. Revert partial changes if they complicate future work
2. Study OpenHTJ2K `ht_cleanup_decode()` more carefully
3. Implement complete magnitude reconstruction formula
4. Add pLSB support throughout decoder chain
5. Create detailed comparison tests

## References

- **OpenHTJ2K**: `libs/openhtj2k_src/source/core/coding/ht_block_decoding.cpp`
  - Lines 590-640: Magnitude reconstruction
  - Lines 607-616: Critical formula for `mu_quads`
- **ISO/IEC 15444-15**: HTJ2K standard (not available in repo)
- **Test**: `tests/integration/test_htj2k_comprehensive.rs`

## Estimated Effort

**Time Required**: 4-8 hours of focused work
**Complexity**: High - requires deep understanding of HTJ2K standard
**Risk**: Medium - changes affect core block decoding logic

## Workaround

Use encoder in "Legacy Mode" (JPEG 2000 + CAP marker) which works correctly.
HTJ2K-specific block coding (VLC/UVLC/MEL) should be marked as experimental until fixed.
