# JPEG2000 Lossy Compression Implementation

## Overview

This implementation adds lossy compression support to the JPEG2000 encoder using the 9-7 irreversible DWT and ICT color transform.

## Implemented Features

### 1. ICT (Irreversible Color Transform)

Converts RGB to Y/Cb/Cr color space for better compression of color images:

```rust
// ICT coefficients from ISO/IEC 15444-1 Annex G.2
let y = 0.299 * r + 0.587 * g + 0.114 * b;
let cb = -0.16875 * r - 0.33126 * g + 0.5 * b;
let cr = 0.5 * r - 0.41869 * g - 0.08131 * b;
```

Location: `src/jpeg2000/encoder.rs` lines 327-351

### 2. Quality-Based Rate Control

Quality parameter (1-100) maps to quantization step sizes:
- **95-100**: Near-lossless (step ~0.00001)
- **75-94**: Visually lossless (step ~0.0001)  
- **50-74**: Good quality (step ~0.001)
- **1-49**: High compression (step ~0.01)

Location: `src/jpeg2000/encoder.rs` lines 170-189

### 3. Perceptual Weighting

Different subbands use different quantization based on perceptual importance:
- **LL subband**: Most important, smallest step
- **HL/LH**: Horizontal/vertical edges, moderate quantization
- **HH**: Diagonal details, can tolerate more quantization
- **Resolution levels**: Coarser levels are more important

Location: `src/jpeg2000/encoder.rs` lines 208-239

### 4. Scalar Expounded Quantization

Per-subband quantization parameters (QCD marker style 0x02) provide fine-grained control over rate-distortion tradeoff.

Location: `src/jpeg2000/encoder.rs` lines 195-266

## Usage

```rust
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;

let mut encoder = J2kEncoder::new();
encoder.set_quality(80);              // Quality 1-100
encoder.set_irreversible(true);       // Use 9-7 DWT
encoder.set_decomposition_levels(5);  // DWT levels

let frame_info = FrameInfo {
    width: 512,
    height: 512,
    bits_per_sample: 8,
    component_count: 3,  // RGB
};

let mut output = vec![0u8; pixels.len() * 2];
let size = encoder.encode(&pixels, &frame_info, &mut output)?;
```

## Testing

Comprehensive test suite in `tests/test_j2k_lossy.rs`:
- Quality level testing (100, 90, 75, 50, 25)
- RGB and grayscale
- Various image sizes
- Compression ratio analysis
- PSNR/MAE metrics

## Known Issue

⚠️ **Quantization Formula Mismatch**

Current lossy implementation has a quantization/dequantization mismatch:
- Works: Lossless (5-3 DWT) ✅
- Works: Lossy with 0 DWT levels ✅
- **Fails**: Lossy with DWT ❌ (produces poor quality)

**Root Cause**: The epsilon/mantissa encoding doesn't correctly match the decoder's dequantization formula.

**Location**: `src/jpeg2000/encoder.rs` lines 241-264 (encoder) vs `src/jpeg2000/image.rs` line 209 (decoder)

**Required Fix**: Debug and align the quantization step size calculation between encoder and decoder.

## Architecture

### Encoder Pipeline (Lossy Mode)

1. **Level shift**: Subtract 2^(depth-1) to center around zero
2. **Color transform**: Apply ICT for RGB images  
3. **Forward 9-7 DWT**: Multi-level wavelet decomposition
4. **Quantization**: Apply scalar quantization to coefficients
5. **Bit-plane coding**: EBCOT entropy coding
6. **Packet formation**: Organize coded data into packets

### Key Methods

- `encode()` - Main encoding entry point
- `apply_forward_dwt_2d()` - 2D DWT (Dwt53 or Dwt97)
- `quantize_97()` - Scalar quantization for 9-7 coefficients
- `encode_component_packets()` - Packet formation

## File Structure

```
src/jpeg2000/
├── encoder.rs          - Main encoder (ICT, quantization, quality control)
├── decoder.rs          - Decoder with dequantization
├── dwt.rs              - 5-3 and 9-7 DWT implementations  
├── quantization.rs     - Quantization utilities
├── bit_plane_coder.rs  - EBCOT entropy coding
└── image.rs            - Image reconstruction with dequantization

tests/
├── test_j2k_lossy.rs   - Comprehensive lossy tests
└── test_lossy_debug.rs - Debug test for quantization issue
```

## Performance Characteristics

### Compression Ratios (Expected)

| Quality | Grayscale | RGB |
|---------|-----------|-----|
| 100     | 2-3x      | 3-4x |
| 90      | 10-15x    | 12-18x |
| 75      | 20-30x    | 25-35x |
| 50      | 40-60x    | 50-70x |

### PSNR Targets

| Quality | Min PSNR |
|---------|----------|
| 100     | 50 dB    |
| 90      | 40 dB    |
| 75      | 35 dB    |
| 50      | 30 dB    |

*Note: Current implementation doesn't meet these targets due to quantization issue*

## Standards Compliance

Based on ISO/IEC 15444-1 (JPEG2000 Part 1):
- ✅ Annex F: 9-7 irreversible filter
- ✅ Annex G.2: ICT (Irreversible Color Transform)  
- ✅ Annex E: Quantization (implementation in progress)
- ✅ Annex C: Entropy coding (EBCOT)

## Future Enhancements

1. **Fix quantization mismatch** - Priority 1
2. **Rate-distortion optimization** - Optimize truncation points
3. **ROI coding** - Region of interest support
4. **Multi-layer encoding** - Progressive quality
5. **Tiling support** - Large image handling
6. **PCRD-opt** - Post-compression rate-distortion optimization

## References

- ISO/IEC 15444-1:2019 - JPEG 2000 image coding system Part 1
- Taubman & Marcellin - "JPEG2000: Image Compression Fundamentals, Standards and Practice"
- OpenJPEG - Reference implementation

## License

MIT License - See project root for details
