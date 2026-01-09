# HTJ2K Encoder/Decoder Investigation

## Problem Statement

HTJ2K roundtrip tests fail with **4089 pixel mismatches** out of 4096 pixels (64x64 8-bit grayscale image). Only the first ~10 blocks decode correctly before catastrophic failure.

## Root Cause Analysis

### Issue 1: UVLC Decoder (FIXED ✅)
- **Problem**: UVLC decoder used simple prefix match instead of longest-match
- **Impact**: Decoded wrong `u_q` values for quads
- **Fix**: Implemented proper longest-prefix matching in `decode_uvlc()`
- **Commit**: `8fb7830`

### Issue 2: VLC Table Generation (FIXED ✅)
- **Problem**: VLC decoder table didn't handle variable-length codes correctly
- **Impact**: Decoder indexed table incorrectly for codes shorter than 7 bits
- **Fix**: Populate all indices matching the prefix bits in `generate_vlc_table()`
- **Commit**: `c233617`

### Issue 3: Context Calculation Mismatch (FIXED ✅)
- **Problem**: Encoder calculated context by checking coefficient values directly, while decoder used `quad_significance[]` array tracking `rho != 0`
- **Impact**: Encoder and decoder calculated different context values for same quads
- **Fix**: 
  - Added `quad_significance: Vec<bool>` to `HTBlockEncoder`
  - Update array after encoding each quad
  - Rewrite `calculate_context()` to use array like decoder
  - Fix context1 calculation to check both North (Q0) and West neighbors
- **Commit**: `29139cd`

### Issue 4: VLC Encoder-Decoder Mismatch (OPEN ❌)

**Symptoms**:
```
ENC Q(0,0): context=0 vlc_value=00E6 vlc_bits=8
DEC Q(0,0): context=0 peek=E6FA rho=0111 u_off=1 emb_k=0111 emb_1=0101 bits=10
```

Encoder writes 8-bit codeword `0x00E6`, but decoder reads `0xE6FA` and decodes it as a 10-bit code with completely different values.

**Analysis**:

1. **VLC Table Collisions**: 93 encoder-key collisions in VLC_TBL0
   - Same `(rho, u_off, e_k, e_1)` maps to multiple `c_q` values with different codewords
   - Example: `rho=1111, u_off=1, e_k=1111, e_1=1000` has 6 different codewords (7-10 bits)
   - Encoder picks first match, but decoder may see different `c_q` prefix in bitstream

2. **Possible Causes**:
   - **Bit ordering**: MSB-first vs LSB-first mismatch
   - **MEL/VLC interleaving**: Incorrect bit packing between MEL and VLC streams
   - **VLC codeword selection**: Multiple valid encodings for same symbol (HTJ2K allows this for compression efficiency)
   - **Embedded bits (emb_k, emb_1)**: Incorrect calculation from coefficient magnitudes
   - **c_q prefix**: Encoder may be writing wrong 3-bit context prefix

3. **Validation Test**: Added `test_vlc_table_validation` showing:
   ```
   VLC_TBL0 Encoder Key Analysis:
     Total entries: 444
     Unique full keys (c_q, rho, u_off, e_k, e_1): 444
     Unique encoder keys (rho, u_off, e_k, e_1): 140
   ```

## Recommended Next Steps

### Priority 1: Verify Decoder Against Reference
Test the decoder against OpenHTJ2K-encoded bitstreams to verify our decoder works correctly:

```bash
# Encode with OpenHTJ2K
opj_compress -i test.raw -o test.j2k -OutFor HTJ2K

# Decode with our decoder
cargo run -- decode -i test.j2k -o decoded.raw

# Compare
cmp test.raw decoded.raw
```

If decoder passes → encoder is the problem
If decoder fails → decoder needs more fixes

### Priority 2: Study Reference Implementation
Compare against OpenHTJ2K source code:
- VLC encoding logic (`encode_vlc` function)
- Bit packing/ordering (MEL vs VLC bits)
- Embedded bit calculation (`emb_k`, `emb_1`)
- Context prefix (`c_q`) selection

### Priority 3: Fix VLC Encoder
Implement one of these strategies:

**Option A**: Match Reference Implementation
- Exactly replicate OpenHTJ2K's codeword selection logic
- Ensures bit-perfect interoperability

**Option B**: Shortest Codeword Strategy
- Always pick the shortest valid codeword for `(rho, u_off, e_k, e_1)`
- If tie, pick smallest `c_q` value
- Optimizes compression

**Option C**: Canonical Huffman Approach
- Define deterministic codeword assignment rules
- Ensures encoder/decoder consistency

### Priority 4: Bit-Level Debugging
Add detailed bit-level logging to both encoder and decoder:
- Log every bit written/read
- Log MEL vs VLC bit source
- Log bit positions and byte boundaries
- Compare encoder output bits vs decoder input bits

## Files Modified

### Encoder Changes
- `src/jpeg2000/ht_block_coder/encoder.rs`:
  - Added `quad_significance` tracking (line 154)
  - Update significance after each quad (lines 334, 360)
  - Fixed context calculation (lines 481-508)
  - Added UVLC debug output (lines 348-366)

### Decoder Changes
- `src/jpeg2000/ht_block_coder/coder.rs`:
  - Already had correct context logic
  - Uses `quad_significance` array properly

### VLC/UVLC Changes
- `src/jpeg2000/ht_block_coder/vlc.rs`:
  - Fixed UVLC longest-prefix matching
  - Fixed VLC table generation
  - Added `validate_vlc_tables()` diagnostic function
  - Added `test_vlc_table_validation` test

## Test Status

| Test | Status | Notes |
|------|--------|-------|
| UVLC encode/decode | ✅ PASS | Verified with unit tests |
| VLC table generation | ✅ PASS | All 1024 entries valid |
| Context calculation | ✅ PASS | Encoder matches decoder |
| HTJ2K 8-bit gray roundtrip | ❌ FAIL | 4089/4096 pixel mismatches |
| HTJ2K decode-only | 🔶 UNKNOWN | Need reference bitstreams |

## Performance Impact

Context calculation changes add minimal overhead:
- Array lookup O(1) vs checking 4 coefficient values
- Memory: +1 bool per quad (~width*height/4 bytes)
- Example: 512x512 image = 65KB extra memory

## References

- ISO/IEC 15444-15: HTJ2K specification
- OpenHTJ2K: https://github.com/osamu620/OpenHTJ2K
- VLC Tables: src/jpeg2000/ht_block_coder/vlc_tables.rs
- Validation output: vlc_validation.txt

## Conclusion

We've fixed 3 out of 4 identified issues. The remaining VLC encoder-decoder mismatch requires deeper investigation into bitstream format and possibly studying the reference implementation. The decoder may be correct; we need to verify it against known-good bitstreams before attempting more encoder fixes.

**Recommendation**: Mark HTJ2K as "experimental" in documentation until this is resolved. Focus efforts on JPEG-LS and JPEG 2000 standard paths which are working correctly.
