# JPEG-LS Decoder Debug Analysis
## Test Case: tiny_8x8_gray_solid.jls

### File Structure
```
Offset  Data                Description
------  ------------------  -----------------
0x00    FF D8               SOI marker
0x02    FF E8 ...           SPIFF segment
0x20    FF E8 ...           Another segment
0x30    FF D8 FF F7         SOI + SOF55 marker
0x33    00 0B 08 00 08 ...  Frame header (8x8, 1 component, 8-bit)
0x3E    FF DA               SOS marker
0x40    00 08 01 01 ...     Scan header
0x45    00 00 01 FC 95 F3 FF 7F   Scan data (8 bytes)
0x4D    FF D9               EOI marker
```

### Scan Data: `00 00 01 FC 95 F3 FF 7F` (8 bytes = 64 bits)

Expected output: 64 pixels, all value 127

### Bitstream Breakdown

```
Binary representation:
00000000 00000000 00000001 11111100 10010101 11110011 11111111 01111111

Bits 0-63:
0000000000000000000000011111110010010101111100111111111101111111
```

### Decoder Behavior (Line 0)

**Pixel 1** (index=1):
- Mode: Run mode check
- Bit 0: 0 → No run, go to interruption
- Bits 1-25: Golomb decode with k=2
  - Unary: 22 zeros
  - Terminator: 1 (bit 23)
  - Remainder: 11 binary = 3 (bits 24-25)
  - Result: (22 << 2) | 3 = 91
  - Error: -(91+1)/2 = -46
  - Reconstruction: ra=173, 173+(-46) = 127 ✅

**Pixels 2-5** (run of 4):
- Mode: Run mode
- Bits 26-36: Run length encoding
  - Bit 26: 1 → run continues
  - Bit 27: ? → run continues  
  - Bit 28: ? → run continues
  - Bit 29: ? → run continues
  - Bit 30: ? → run interruption
- Decoded run_length: 4
- Pixels 2,3,4,5 = 127 (copied from ra) ✅

**Pixel 6** (run interruption):
- Mode: Run interruption after run of 4
- Bits used so far: ~37-38
- Context: ra=127, rb=127
- Golomb decode with k=5
  - Result: 5
  - Error: -3 (unmapped)
  - Reconstruction: 127 + (-3) = 124 ❌
- **Expected**: error=0, result=127

### Problem Analysis

The run interruption should produce error=0 for a solid image (all pixels 127), but we decode error=-3.

**Hypothesis 1: Context State Wrong**
- run_mode_contexts[1] might have incorrect nn, n, a, b values
- After the run, context should be updated correctly
- Need to log context state before decoding error

**Hypothesis 2: Run Length Wrong**
- Maybe the actual run length is 3, not 4?
- This would shift the interruption pixel position
- Need to verify bit-by-bit run length decoding

**Hypothesis 3: Error Decoding Formula**
- The unmap_error_value or decode_error_value might have bugs
- But this works correctly for pixel 1, so less likely

### Next Steps

1. Add logging for run_mode_contexts[1] state before interruption decode
2. Manually decode the run length bits 26-37 to verify it's 4
3. Check if context update after run is correct
4. Compare with CharLS source code for run interruption handling
