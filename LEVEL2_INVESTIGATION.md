# JPEG 2000 Multi-Level DWT Investigation Summary
**Date**: 2026-01-12  
**Status**: Bug identified but root cause still unclear

## Problem Statement

JPEG 2000 lossless encoding works perfectly with 0-1 decomposition levels but fails starting at 2+ levels:

```
✅ Level 0 (1 resolution):  MAE=0.0000, Size=1912B
✅ Level 1 (2 resolutions): MAE=0.0000, Size=847B
❌ Level 2 (3 resolutions): MAE=0.0210, Size=687B (86/4096 pixels off by 1)
❌ Level 3+: Errors increase with level count
```

## Investigation Timeline

### Phase 1: Bit-Plane Coder ✅
**Fixed**: Removed premature LSB truncation by setting `min_bp = 0` for lossless.
**Result**: Improved from baseline to 128/300 J2K interop tests passing.

### Phase 2: Multi-Level DWT Stride Handling ✅
**Fixed**: Multi-level DWT now correctly uses temporary buffers with proper stride calculations.
**File**: `src/jpeg2000/encoder.rs` lines 676-753

### Phase 3: Test Suite Endianness Bug ✅
**Discovery**: 16-bit comprehensive tests were failing due to test harness bug (endianness), not encoder bug.
**Verification**: Our 16-bit encoder is perfect (MAE=0).

### Phase 4: Pinpointed Level 2+ Failure 🎯
**Observation**: 
- Solid/uniform patterns: Perfect at ALL levels
- Gradients with 0-1 decomp: Perfect
- Gradients with 2+ decomp: Errors start, magnitude increases

**File size anomaly**: Our files are significantly smaller than OpenJPEG:
```
Level 2: 687B (ours) vs 1106B (OpenJPEG) - 419B missing (~37% smaller)
```

### Phase 5: DWT Verification ✅
**Test**: `tests/debug_dwt_gradient.rs`
**Result**: DWT forward/inverse is perfectly reversible in isolation.
**Conclusion**: Bug is NOT in DWT itself.

### Phase 6: Coefficient Extraction Analysis ✅
**Initial hypothesis**: res=1 and res=2 extract from wrong positions
**Investigation**: Added extensive debug logging to track:
- Subband sizes and positions
- Actual array indices accessed
- Coefficient values extracted

**Key findings**:
1. Extraction positions are CORRECT:
   - res=1 (HL): extracts from [16..32, 0..16] - correct for HL2
   - res=2 (HL): extracts from [32..64, 0..32] - correct for HL1

2. DWT coefficient layout is correct (verified via `debug_coeff_inspection.rs`)

3. Different test inputs produce different coefficients (as expected)

## Current State

### What's Working ✅
- DWT implementation (perfectly reversible)
- Subband size calculations
- Extraction position calculations
- Multi-level DWT recursion
- Resolution 0 and 1 encoding (perfect)

### What's Failing ❌
- Resolution 2+ gradient encoding (small errors, ~2% of pixels off by 1)
- File size is 37% smaller than OpenJPEG reference

### Key Observations
1. **Solid patterns work, gradients fail** → Suggests issue with non-zero high-frequency coefficients
2. **Small magnitude errors** → Not a gross structural bug, but subtle encoding issue
3. **Smaller file size** → Either:
   - Not encoding some coefficients
   - Under-encoding coefficient precision
   - Missing codeblocks or bit-planes
   - Incorrect packet structure

## Hypotheses to Investigate

### H1: Zero Bit-Plane Calculation
**Location**: `encoder.rs` lines 973-978
```rust
let mb = (guard_bits + epsilon).saturating_sub(1);
let zero_bp = if max_bp < mb { mb - max_bp - 1 } else { 0 };
```

**Question**: Is `zero_bp` calculated correctly for higher resolution levels?
**Test**: Add debug logging for zero_bp values across resolutions

### H2: Epsilon Values for Multi-Level
**Location**: `encoder.rs` lines 830-855
**Formula**: LL=depth, HL/LH=depth+1, HH=depth+2

**Question**: Are epsilon values correct when res > num_decomp_levels?
**Current**: Epsilon based on QCD index seems correct
**Test**: Verify epsilon values match OpenJPEG for level 2+

### H3: Packet Header Structure
**Location**: `packet.rs` lines 269-360
**Question**: Are we correctly signaling all codeblocks in packet headers?
**Observation**: Uses tag trees for inclusion and zero bit-planes
**Test**: Compare packet header bytes with OpenJPEG for identical input

### H4: Codeblock Iteration
**Location**: `encoder.rs` lines 857-995
**Question**: Are we skipping codeblocks that should be encoded?
**Code**:
```rust
let has_nonzero = block_data.iter().any(|&v| v != 0);
if max_bp_opt.is_some() || has_nonzero { /* encode */ }
```

**Concern**: All-zero blocks might not be properly signaled

### H5: Resolution-to-Subband Mapping
**Question**: Is the JPEG 2000 resolution progression semantics correct?
**Current understanding**:
- res=0: LL from final DWT level
- res=1: HL/LH/HH from final DWT level
- res=2: HL/LH/HH from second-to-last DWT level
- etc.

**Status**: Pending Oracle consultation on standard interpretation

## Debug Tools Created

1. **`tests/debug_level_sweep.rs`**: Sweep decomposition levels 0-5
2. **`tests/debug_dwt_layout.rs`**: Verify subband layout
3. **`tests/debug_dwt_gradient.rs`**: Verify DWT roundtrip
4. **`tests/debug_coeff_inspection.rs`**: Manual DWT coefficient analysis
5. **Debug environment variables**:
   - `J2K_PKT_DEBUG`: Packet/codeblock debug output
   - `J2K_EXTRACT_DEBUG`: Subband extraction debug
   - `J2K_LL_SIZE_DEBUG`: LL size calculation debug

## Next Steps

1. **Compare with OpenJPEG byte-by-byte**:
   - Same input image
   - Same decomposition level
   - Compare:
     - Marker segments (SOT, COD, QCD)
     - Packet header structure
     - Packet body lengths
     - Codeblock data

2. **Investigate file size discrepancy**:
   - Count number of codeblocks in our encoding vs OpenJPEG
   - Check if we're encoding fewer passes per codeblock
   - Verify all subbands are being encoded

3. **Test with OpenJPEG decoder verbose mode**:
   ```bash
   opj_decompress -i our_file.j2k -o output.pgm -v
   ```
   - Check for warnings about packet structure
   - Verify all resolutions/subbands are decoded

4. **Consult JPEG 2000 Part 1 standard** (ISO/IEC 15444-1):
   - Section A.6: Resolution levels
   - Section B.10: Packet structure
   - Annex J: Example codestreams

## Files Modified

- `src/jpeg2000/encoder.rs`: Lines 676-753, 820-1022, 1150-1210
- `src/jpeg2000/packet.rs`: Lines 269-360
- `src/jpeg2000/bit_plane_coder.rs`: Lines 234-244, 255-274
- Multiple test files in `tests/`

## Reference Data

**Test image**: 64x64 gradient
```rust
pixels[y * width + x] = ((x * 4 + y * 4) % 256) as u8;
```

**DWT**: 5/3 reversible (lossless)

**OpenJPEG version**: 2.5.2

**Command line**:
```bash
opj_compress -i input.raw -o output.j2k -n 3 -r 1 -F 64,64,1,8,u -I
```
(n=3 means 2 decomposition levels, 3 resolutions)
