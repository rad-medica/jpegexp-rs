# Phase 2 Progress: JPEG-LS Decoder Investigation

## Summary

Phase 2 has begun with deep investigation into the JPEG-LS decoder issues. Critical findings about byte stuffing and scan termination have been identified.

## Investigation Conducted

### 1. Analyzed CharLS-Encoded Files

**Test Case**: `tiny_8x8_gray_solid` (all pixels = 127)

**File Structure**:
```
Total size: 79 bytes
├─ Headers: 0-68 (69 bytes)
│  ├─ SOI (FFD8)
│  ├─ SPIFF header (FFE8)
│  ├─ SOF55 (FFF7) - JPEG-LS Start of Frame
│  └─ SOS (FFDA) - Start of Scan
├─ Scan data: 69-76 (8 bytes)
│  00 00 01 fc 95 f3 ff 7f
└─ EOI: 77-78 (FFD9)
```

**Compression Achieved**:
- 8 bytes for 64 pixels
- 64 bits total  
- **1 bit per pixel** (excellent for solid image with run-length encoding)

### 2. Identified Critical Pattern: FF 7F

**The Issue**:
At offset 75-76, scan data ends with `ff 7f` immediately before the EOI marker (`ff d9`).

**Analysis**:
- `0xFF 0x7F` appears at scan end
- `0x7F` is NOT a standard JPEG marker code
- Our `is_valid_jpeg_marker()` correctly identifies this (0x7F not in valid ranges)
- Current decoder behavior:
  1. Reads 0xFF into cache
  2. Peeks at next byte (0x7F)
  3. Determines it's not a marker
  4. Keeps 0xFF in cache as data
  5. Next iteration should read 0x7F

**Questions**:
1. Is `ff 7f` intentional JPEG-LS padding/termination?
2. Should decoder stop at ANY FF not followed by 00?
3. Is there special scan-end handling in JPEG-LS spec?

### 3. Byte Stuffing Rules Review

**Standard JPEG Byte Stuffing**:
- Encoder: `0xFF` data → `0xFF 0x00` in bitstream
- Decoder: `0xFF 0x00` → `0xFF` data
- Decoder: `0xFF 0xXX` (XX ≠ 00) → Marker

**Our Current Implementation**:
```rust
if next_byte == 0x00 {
    // Byte stuffing - consume 00, keep FF as data
} else if Self::is_valid_jpeg_marker(next_byte) {
    // Valid marker - stop reading
} else {
    // Non-marker code - keep FF as data, read next_byte later
}
```

**Potential Issue**:
The "else" case treats FF + non-marker as data + future-byte. But JPEG-LS might require stricter handling.

### 4. Debug Output Analysis

**From `JPEGLS_DEBUG=1` run**:

```
=== ScanDecoder Initialized ===
  Source length: 10 bytes
  Frame: 8x8, 1 components, 8 bpp
  Initial cache: 56 valid bits, position: 7

=== Starting decode_lines ===
Line 0:
  Golomb decode: k=2, unary=22, remainder=3, result=91
  Golomb decode: k=2, unary=0, remainder=3, result=3
  ... (6 more pixels)
Line 0 complete: 8 pixels decoded

Line 1:
  Golomb decode: k=2, unary=0, remainder=0, result=0
  ... (7 pixels)
  ✗ peek_bits(1) FAILED: only 0 bits available at pos 8
```

**Analysis**:
- Line 0 first pixel: k=2, unary=22 → result=91 → unmapped error=-46
  - For solid image (all 127), predicted ~173 (way off!)
  - Suggests context initialization or prediction issue
- Position 8 means we read 8 bytes from the 10-byte "source"
- But actual scan data is only 8 bytes (plus 2-byte EOI)
- Decoder should stop before EOI, not after

### 5. Root Causes Identified

**Primary Issue**: Scan Termination Handling
- Decoder receives 10 bytes (scan data + EOI marker)
- Should only process 8 bytes (scan data)
- `fill_read_cache` should stop when encountering EOI
- But `ff 7f` pattern is confusing the logic

**Secondary Issue**: First Pixel Prediction  
- First pixel gets predicted as ~173, actual is 127
- Error of -46 is too large
- Context initialization may be wrong (a=4, b=0, c=0, n=1)
- Or prediction logic has bug

**Tertiary Issue**: Bit Consumption Rate
- Should consume ~1 bit/pixel for solid image
- Actually consuming more (runs out after 8 pixels)
- May be related to scan termination

## Proposed Fixes

### Fix 1: Strict Byte Stuffing (Conservative)

Only treat `0xFF 0x00` as stuffed byte. Any other `0xFF 0xXX` stops reading:

```rust
if byte == JPEG_MARKER_START_BYTE as usize {
    if self.position < self.source.len() {
        let next_byte = self.source[self.position];
        if next_byte == 0x00 {
            // Byte stuffing - consume 00, keep FF as data
            self.position += 1;
        } else {
            // Any other pattern - treat as marker, stop reading
            self.position -= 1;
            self.valid_bits -= 8;
            self.read_cache >>= 8;
            break;
        }
    }
}
```

**Pros**: Simple, matches strict JPEG byte stuffing
**Cons**: May break if JPEG-LS uses FF + non-marker for other purposes

### Fix 2: JPEG-LS Padding Detection (Targeted)

Specifically handle `ff 7f` as scan-end padding:

```rust
if byte == JPEG_MARKER_START_BYTE as usize {
    if self.position < self.source.len() {
        let next_byte = self.source[self.position];
        if next_byte == 0x00 {
            // Byte stuffing
            self.position += 1;
        } else if next_byte == 0x7F {
            // JPEG-LS scan end padding - stop here
            self.position -= 1;
            self.valid_bits -= 8;
            self.read_cache >>= 8;
            break;
        } else if Self::is_valid_jpeg_marker(next_byte) {
            // Valid marker
            self.position -= 1;
            self.valid_bits -= 8;
            self.read_cache >>= 8;
            break;
        } else {
            // Other non-marker codes
            // Current: treat as data
            // May need adjustment
        }
    }
}
```

**Pros**: Targeted fix for observed pattern
**Cons**: May not cover all cases, needs spec verification

### Fix 3: Research-Based (Recommended)

Before implementing, research:
1. ITU-T T.87 (JPEG-LS spec) Section on byte stuffing
2. CharLS source code for scan termination handling
3. Check if `0x7F` has special meaning in JPEG-LS

**Resources**:
- ITU-T T.87: https://www.itu.int/rec/T-REC-T.87
- CharLS: https://github.com/team-charls/charls
  - Look at: `scan_decoder.cpp`, `scan_encoder.cpp`
  - Search for: "0x7F", "padding", "scan end"

## Next Actions

### Immediate (High Priority)
1. ✅ Document findings (this file)
2. ⏳ Research JPEG-LS spec for byte stuffing at scan end
3. ⏳ Study CharLS implementation of `fill_read_cache` equivalent
4. ⏳ Determine correct handling of `ff 7f` pattern

### Short-term (After Research)
5. Implement correct byte stuffing fix
6. Add unit tests for byte stuffing edge cases
7. Validate fix with all test images
8. Move to next issue (context initialization)

### Test Cases Needed

Create specific byte stuffing tests:
```rust
#[test]
fn test_byte_stuffing_ff_00() {
    // Test: ff 00 should decode as single ff byte
}

#[test]
fn test_byte_stuffing_ff_7f() {
    // Test: ff 7f behavior at scan end
}

#[test]
fn test_scan_termination() {
    // Test: decoder stops at EOI marker correctly
}
```

## Current Blockers

**Blocker 1**: Uncertain JPEG-LS scan termination rules
- **Impact**: Cannot implement correct fix without understanding spec
- **Resolution**: Research spec + CharLS implementation
- **Timeline**: Few hours of research needed

**Blocker 2**: First pixel prediction issue
- **Impact**: Even with correct byte reading, pixels may be wrong
- **Resolution**: Review prediction and context initialization
- **Timeline**: After byte stuffing fix

## Success Metrics

When fixes are complete:
- [ ] All 8x8 test images decode without "Invalid data" error
- [ ] Bit consumption matches CharLS (1-4 bits/pixel depending on content)
- [ ] Pixel values match exactly (MAE = 0)
- [ ] Decoder handles scan end gracefully
- [ ] No crashes on corrupt files with unusual byte patterns

## Conclusion

Phase 2 investigation has successfully:
1. ✅ Identified exact location of byte stuffing issue
2. ✅ Analyzed CharLS encoding patterns
3. ✅ Pinpointed `ff 7f` scan-end pattern
4. ✅ Enhanced debug logging for future investigation
5. ⏳ Documented multiple fix approaches

Next step: Research JPEG-LS spec to implement correct byte stuffing handling at scan boundaries.

---
**Status**: Phase 2 in progress - investigation complete, implementation pending spec research
**Estimated completion**: 1-2 days for byte stuffing fix + validation
