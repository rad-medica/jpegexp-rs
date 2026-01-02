# JPEG-LS Codec Fix - Technical Notes

## Problem Statement
"FIX, even if it means refactor the JPEG-LS codec"

## Issues Identified

### 1. Byte Stuffing Marker Detection (FIXED)
**Location**: `src/jpegls/scan_decoder.rs`, `fill_read_cache()` function

**Problem**: The decoder's byte stuffing logic incorrectly identified JPEG markers using `next_byte & 0x80 != 0`, which only detected bytes >= 0x80 as markers.

**Impact**: The decoder would treat invalid patterns like "FF 78" as data instead of recognizing them appropriately.

**Solution**: Implemented proper JPEG marker validation using `is_valid_jpeg_marker()` helper function that checks against actual valid JPEG/JPEG-LS marker codes (0xC0-0xCF, 0xD0-0xD9, 0xDA-0xDF, 0xE0-0xEF, 0xF0-0xFE, 0xC8).

**Status**: ✅ Fixed

### 2. Premature End-of-Data During Decoding (PARTIALLY DIAGNOSED)
**Location**: `src/jpegls/scan_decoder.rs`, decoding logic

**Problem**: When decoding a CharLS-encoded 16x16 grayscale image:
- Successfully decodes first 8 lines (50% of image)
- Runs out of bits at position 49 (where EOI marker is located)
- Only 51 bytes of scan data available, not enough for full decode

**Observations**:
- Scan data starts at offset 69 in the file
- File contains many "FF xx" patterns where xx is not 00 (not byte stuffing)
- Position tracking shows:
  - Line 0: position=7, valid_bits=56
  - Line 4: position=27, valid_bits=33
  - Line 8: position=49, valid_bits=6 (FAILS)

**Possible Root Causes**:
1. **Encoder Issue**: Our encoder may not be producing enough compressed data
2. **Decoder Issue**: Decoder may be consuming bits too quickly (incorrect Golomb decoding?)
3. **Bit Alignment Issue**: Mismatch in how bits are aligned/padded at scan end

**Status**: ⚠️ Requires further investigation

### 3. Encoder Compatibility (NOT TESTED YET)
**Location**: `src/jpegls/scan_encoder.rs`

**Problem**: According to CODEC_TEST_RESULTS.md, the encoder produces bitstreams that CharLS cannot decode.

**Status**: ❌ Not yet investigated

## Test Results

### Test Case: 16x16 Grayscale Image (0-255 sequential values)
```
Source: CharLS encoder (reference implementation)
Target: Our JPEG-LS decoder

Result: FAIL
- Header parsing: ✅ SUCCESS
- Dimensions detected: ✅ 16x16, 1 component, 8 bpp
- Scan data available: 51 bytes
- Decoding progress: 50% (8 of 16 lines)
- Error: "Invalid data" - insufficient bits at position 49
```

## Debugging Insights

### Byte Stuffing in JPEG-LS
Per JPEG-LS specification:
- In the compressed bitstream, any 0xFF byte must be escaped as "0xFF 0x00"
- When decoder sees "0xFF 0x00", it represents a single 0xFF in the data
- When decoder sees "0xFF xx" where xx ≠ 0x00:
  - If xx is a valid marker code: Stop reading, handle marker
  - If xx is invalid: Spec violation, but treat as two separate bytes

### Valid JPEG-LS Marker Codes
- SOI: 0xFFD8
- EOI: 0xFFD9
- SOS: 0xFFDA
- SOF55: 0xFFF7 (JPEG-LS specific)
- LSE: 0xFFF8 (JPEG-LS specific)
- RSTm: 0xFFD0-0xFFD7
- APPn: 0xFFE0-0xFFEF
- COM: 0xFFFE
- Other SOF: 0xFFC0-0xFFCF

## Recommended Next Steps

### Immediate (High Priority)
1. **Add comprehensive logging** to bit consumption in decoder:
   - Log every `read_bits()` call with count and context
   - Track cumulative bits consumed per line
   - Compare with expected bit consumption

2. **Create unit tests** for encoder-decoder roundtrip:
   ```rust
   #[test]
   fn test_jpegls_roundtrip_tiny() {
       let data = vec![0u8, 32, 64, 96, 128, 160, 192, 224];
       // Encode with our encoder
       // Decode with our decoder
       // Verify perfect match
   }
   ```

3. **Compare bit-level output** with CharLS:
   - Encode same image with both encoders
   - Compare bitstreams byte-by-byte
   - Identify where they diverge

### Short-term (Medium Priority)
4. **Implement encoder validation**:
   - Test if our encoder output can be decoded by CharLS
   - Test if our encoder output can be decoded by our decoder

5. **Review Golomb coding implementation**:
   - Verify `decode_mapped_error_value()` logic
   - Check context management and k-parameter calculation
   - Ensure run mode vs regular mode transitions are correct

6. **Fix multi-component handling**:
   - Current decoder returns `InvalidOperation` for components ≠ 1
   - Need to implement proper interleaved/planar mode support

### Long-term (Lower Priority)
7. **Consider architectural refactor**:
   - If issues are pervasive, consider using CharLS via FFI
   - Or rewrite codec based on proven reference implementation
   - Estimated effort: 1-2 weeks as per original assessment

## Code Locations

### Key Files
- `src/jpegls/scan_decoder.rs` - Main decoding logic
- `src/jpegls/scan_encoder.rs` - Main encoding logic
- `src/jpegls/decoder.rs` - High-level decoder interface
- `src/jpegls/encoder.rs` - High-level encoder interface
- `src/jpegls/regular_mode_context.rs` - Context for regular mode
- `src/jpegls/run_mode_context.rs` - Context for run mode

### Functions to Review
- `ScanDecoder::fill_read_cache()` - Bit reading with byte stuffing
- `ScanDecoder::decode_mapped_error_value()` - Golomb decoding
- `ScanDecoder::decode_sample_line()` - Line-by-line decoding
- `ScanEncoder::encode_sample_line()` - Line-by-line encoding

## Related Documentation
- `CODEC_TEST_RESULTS.md` - Original test results showing failures
- `SUMMARY.md` - Project assessment with estimated fix time
- `COMPLIANCE.md` - Conformance testing details

## References
- ITU-T T.87 (JPEG-LS specification)
- CharLS: https://github.com/team-charls/charls
- JPEG-LS Wikipedia: https://en.wikipedia.org/wiki/Lossless_JPEG#JPEG-LS
