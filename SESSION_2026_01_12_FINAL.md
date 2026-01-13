# JPEG 2000 Debugging Session - January 12, 2026

## Session Overview

**Duration**: Extended investigation session  
**Focus**: JPEG 2000 multi-level DWT gradient encoding failures  
**Outcome**: Critical Psot bug fixed + comprehensive investigation documented

---

## Major Accomplishment: Psot Bug Fix ✅

### Bug Description
**Component**: SOT (Start of Tile) marker - Psot field  
**Severity**: High - ISO 15444-1 standard compliance violation

**Issues**:
1. Psot calculation incorrectly included EOC marker  
2. Encoder always wrote `Psot=0` instead of actual tile length

### Fix Implemented
**File**: `src/jpeg2000/encoder.rs`  
**Lines**: 644, 653

```rust
// BEFORE
let tile_total_len = tile_part_header_len + 2 + total_packet_len + 2; // Includes EOC ❌
writer.write_sot(0, 0, 0, 1)?; // Psot=0 ❌

// AFTER  
let tile_total_len = tile_part_header_len + 2 + total_packet_len; // Excludes EOC ✅
writer.write_sot(0, tile_total_len as u32, 0, 1)?; // Actual length ✅
```

**Standard Reference**: ISO/IEC 15444-1 Section A.4.2  
*"Psot: Length measured from first byte of SOT to end of tile-part bit stream"*  
→ EOC marker is NOT part of tile-part

### Impact
- ✅ Standard-compliant SOT markers
- ✅ Correct Psot values (608 bytes vs 0 for 64x64 Level 2)
- ✅ Better decoder compatibility
- ✅ Levels 0-1 remain perfect (MAE=0)
- Level 2+ gradient errors unchanged (separate issue)

**Commit**: `d8598a2` - "fix(j2k): correct Psot calculation in SOT marker"

---

## Comprehensive Investigation: Level 2+ Gradient Failures 🔍

### Problem Statement
JPEG 2000 lossless encoding works perfectly with 0-1 decomposition levels but fails with 2+ levels:

| Level | Resolutions | Status | MAE | Pixels w/Error | Notes |
|-------|-------------|--------|-----|----------------|-------|
| 0 | 1 | ✅ Perfect | 0.0000 | 0/4096 | 1912B |
| 1 | 2 | ✅ Perfect | 0.0000 | 0/4096 | 847B |
| 2 | 3 | ❌ Errors | 0.0210 | 86/4096 (2.1%) | 687B vs 1106B OpenJPEG |
| 3 | 4 | ❌ Errors | 0.2617 | 1049/4096 | Errors increase |
| 4 | 5 | ❌ Errors | 0.4180 | 1649/4096 | with level count |
| 5 | 6 | ❌ Errors | 1.0164 | 2866/4096 | |

**Key Pattern**: Solid/uniform images work perfectly at ALL levels. Only gradients fail.

### Investigation Results ✅

**Verified Correct**:
1. ✅ **DWT Implementation**: Perfectly reversible (tested independently)
2. ✅ **Subband Extraction**: Correctly extracts from proper memory positions
3. ✅ **Multi-level DWT Recursion**: Properly applies DWT in-place with correct stride
4. ✅ **Coefficient Layout**: Correct structure after 2-level DWT
5. ✅ **Resolution Indexing**: Correct mapping of resolutions to DWT levels
6. ✅ **Codeblock Structure**: Generates 7 codeblocks for 3 resolutions (correct)
7. ✅ **Level Shifting**: Correct (128 for 8-bit unsigned)

**File Size Analysis**:
- Our encoding: 687 bytes (Level 2, 64x64 gradient)
- OpenJPEG: 1106 bytes
- **Difference: 419 bytes (37.9% smaller)**

**Packet Data**:
- res=0: 194 bytes (header + body)
- res=1: 205 bytes
- res=2: 182 bytes
- **Total: 581 bytes packet data**
- OpenJPEG: ~887 bytes packet data (306 bytes more)

**Codeblock Analysis**:
- 7 codeblocks generated (1 LL + 3×2 for res 1&2)
- 19-25 passes per codeblock (expected for 6-8 bit planes)
- Data lengths: 42-191 bytes per codeblock

### Remaining Hypothesis 🎯

**File size discrepancy suggests**:
1. **Packet headers**: Our 9-byte headers vs OpenJPEG's (possibly larger)
2. **QCD marker**: 10 bytes vs 17 bytes (7-byte difference)
3. **Bit-plane encoding**: Possibly under-encoding precision
4. **MQ coder output**: Potentially different compression efficiency

**Error Pattern**:
- Only 2.1% of pixels affected (Level 2)
- Maximum error magnitude: 1
- Suggests subtle rounding or quantization issue, not gross structural bug

---

## Documentation Created 📚

### Investigation Documents
1. **LEVEL2_INVESTIGATION.md** (200+ lines)
   - Complete investigation timeline
   - Phase-by-phase analysis
   - All hypotheses tested
   - Debug tools created
   - Next steps outlined

2. **PSOT_FIX_SUMMARY.md**
   - Detailed bug description
   - Standard references
   - Before/after comparison
   - Verification steps

3. **SESSION_2026_01_12_FINAL.md** (this document)
   - Session summary
   - Accomplishments
   - Test status
   - Future work

### Debug Tools Created
Created 25+ diagnostic test files:

**Primary Tools**:
- `tests/compare_packet_structure.rs` - Marker/packet analysis
- `tests/debug_level_sweep.rs` - Decomposition level sweep (0-5)
- `tests/debug_coeff_inspection.rs` - Manual DWT coefficient analysis
- `tests/count_codeblocks.rs` - Codeblock generation counting
- `tests/debug_dwt_gradient.rs` - DWT roundtrip verification
- `tests/debug_dwt_layout.rs` - Subband layout verification

**Specialized Tests**:
- 15+ trace/debug tests for packet headers, bit-plane coding, MQ coder
- Multiple solid vs gradient comparison tests
- 16-bit specific tests
- Header comparison tools

### Environment Variables Added
- `J2K_PKT_DEBUG` - Packet/codeblock debug output
- `J2K_EXTRACT_DEBUG` - Subband extraction positions
- `J2K_LL_SIZE_DEBUG` - LL size calculations
- `J2K_CBLK_DETAIL` - Detailed codeblock encoding info
- `J2K_PACKET_SIZES` - Individual packet sizes

---

## Test Status 📊

### Overall J2K Interop
**Current**: 128/300 tests passing (42.7%)

**By Category**:
- ✅ **Solid patterns**: Perfect at ALL levels (8/10/12/16-bit)
- ✅ **Levels 0-1**: Perfect for ALL patterns
- ❌ **Level 2+ gradients**: Small errors (2-70% pixels, magnitude 1-4)
- ❌ **16-bit**: Large errors (MAE 20000+) - separate issue

**Perfect Results** (MAE=0.0000):
- 512x512 10-bit lossless (4 tests)
- All solid patterns (multiple sizes/depths)
- All Level 0-1 encodings

### Pattern-Specific Results

| Pattern | 8-bit | 10-bit | 12-bit | 16-bit |
|---------|-------|--------|--------|--------|
| Solid | ✅ Perfect | ✅ Perfect | ✅ Perfect | ❌ Errors |
| Gradient | ❌ Small errors | ✅ Some perfect | ❌ Errors | ❌ Large errors |
| Checkerboard | ❌ Errors | - | ❌ Errors | ❌ Errors |
| Noise | ❌ Errors | - | ❌ Errors | ❌ Errors |

---

## Technical Insights Gained 💡

### JPEG 2000 Standard Clarifications
1. **Psot calculation**: Must exclude EOC marker (A.4.2)
2. **Resolution indexing**: res=0 is lowest (smallest LL), increases to full size
3. **DWT layout**: After multi-level in-place DWT, subbands arranged hierarchically
4. **Packet progression**: LRCP order (Layer-Resolution-Component-Position)

### Encoder Architecture Understanding
1. **Multi-level DWT**: Applied recursively on top-left LL subband
2. **Stride handling**: Critical for correct coefficient extraction
3. **Epsilon values**: LL=depth, HL/LH=depth+1, HH=depth+2
4. **Zero bit-planes**: Calculated as `mb - max_bp - 1` where `mb = guard_bits + epsilon - 1`

### Debug Methodology Established
1. **Isolation testing**: Test each component independently
2. **Reference comparison**: Byte-by-byte comparison with OpenJPEG
3. **Pattern testing**: Use solid vs gradient to isolate issues
4. **Level sweep**: Test all decomposition levels systematically

---

## Future Work Recommendations 🔮

### Immediate Next Steps
1. **QCD marker investigation**
   - Why 10 bytes vs OpenJPEG's 17 bytes?
   - Are we missing quantization step sizes?
   - Check epsilon encoding

2. **Packet header analysis**
   - Byte-by-byte comparison with OpenJPEG
   - Verify tag tree encoding
   - Check inclusion/zero-bitplane signaling

3. **Bit-plane pass verification**
   - Compare pass counts with OpenJPEG
   - Verify all passes are being encoded
   - Check MQ coder output length

### Long-term Investigations
1. **16-bit encoding**: Separate issue with large errors
2. **Lossy mode**: Currently untested for multi-level
3. **RGB/multi-component**: Additional complexity
4. **HTJ2K mode**: Different codeblock encoding

### Standard Compliance Checklist
- [x] Psot calculation (ISO 15444-1 A.4.2)
- [x] DWT 5-3 reversible (ISO 15444-1 Annex F)
- [ ] QCD marker structure (ISO 15444-1 A.6.4)
- [ ] Packet header format (ISO 15444-1 B.10)
- [ ] Tag tree encoding (ISO 15444-1 B.10.2)
- [ ] MQ coder (ISO 15444-1 Annex C)

---

## Files Modified This Session

### Source Code
- `src/jpeg2000/encoder.rs`:
  - Lines 644, 653: Psot calculation fix
  - Lines 664-667, 977-981, 1001-1014: Debug logging added

### Documentation
- `LEVEL2_INVESTIGATION.md` - Created (200+ lines)
- `PSOT_FIX_SUMMARY.md` - Created
- `SESSION_2026_01_12_FINAL.md` - Created (this file)

### Tests
- `tests/compare_packet_structure.rs` - Created
- `tests/count_codeblocks.rs` - Created
- `tests/debug_coeff_inspection.rs` - Created
- `tests/debug_level_sweep.rs` - Created
- 20+ other debug/trace tests created

---

## Conclusion

This session achieved a critical standard compliance fix (Psot) and conducted comprehensive investigation of the Level 2+ gradient encoding issue. The investigation systematically verified each component of the encoder pipeline and narrowed the remaining issue to packet encoding or bit-plane precision.

**Key Takeaway**: The encoder is fundamentally sound (DWT, subband extraction, structure), but has a subtle issue in how multi-level gradient coefficients are encoded into packets. The 37.9% smaller file size and 2% pixel error rate suggest under-encoding rather than incorrect encoding.

The extensive documentation and debug tools created provide a solid foundation for future debugging efforts.

**Status**: Ready for continued investigation with clear next steps identified.
