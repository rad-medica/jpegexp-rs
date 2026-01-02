# JPEG-LS Test Images

Generated using CharLS (via imagecodecs) for jpegexp-rs validation.

Total test cases: 22

## Test Results Summary

| Category | Tests | Status |
|----------|-------|--------|
| Grayscale 8-bit | 14 | ✅ All pass (MAE = 0) |
| Grayscale 16-bit | 2 | ✅ All pass (MAE = 0) |
| Edge cases | 3 | ✅ All pass (MAE = 0) |
| RGB (sample-interleave) | 6 | ⚠️ Not yet supported |

## Test Cases

### Grayscale 8-bit (All Passing ✅)

| Name | Size | Pattern | Status |
|------|------|---------|--------|
| tiny_8x8_gray_gradient | 8x8 | gradient | ✅ Lossless |
| tiny_8x8_gray_noise | 8x8 | noise | ✅ Lossless |
| tiny_8x8_gray_checker | 8x8 | checker | ✅ Lossless |
| tiny_8x8_gray_solid | 8x8 | solid | ✅ Lossless |
| small_16x16_gray_gradient | 16x16 | gradient | ✅ Lossless |
| small_32x32_gray_gradient | 32x32 | gradient | ✅ Lossless |
| medium_64x64_gray_gradient | 64x64 | gradient | ✅ Lossless |
| medium_128x128_gray_gradient | 128x128 | gradient | ✅ Lossless |
| large_256x256_gray_gradient | 256x256 | gradient | ✅ Lossless |
| rect_16x32_gray_gradient | 16x32 | gradient | ✅ Lossless |
| rect_32x16_gray_gradient | 32x16 | gradient | ✅ Lossless |

### Grayscale 16-bit (All Passing ✅)

| Name | Size | Pattern | Status |
|------|------|---------|--------|
| small_16x16_gray16_gradient | 16x16 | gradient | ✅ Lossless |
| small_32x32_gray16_gradient | 32x32 | gradient | ✅ Lossless |

### Edge Cases (All Passing ✅)

| Name | Size | Pattern | Status |
|------|------|---------|--------|
| edge_1x1_gray | 1x1 | solid | ✅ Lossless |
| edge_1x8_gray | 1x8 | gradient | ✅ Lossless |
| edge_8x1_gray | 8x1 | gradient | ✅ Lossless |

### RGB (Not Yet Supported ⚠️)

| Name | Size | Pattern | Status |
|------|------|---------|--------|
| tiny_8x8_rgb_gradient | 8x8 | gradient | ⚠️ Ignored |
| small_16x16_rgb_gradient | 16x16 | gradient | ⚠️ Ignored |
| small_32x32_rgb_gradient | 32x32 | gradient | ⚠️ Ignored |
| medium_64x64_rgb_gradient | 64x64 | gradient | ⚠️ Ignored |
| small_16x16_rgb_noise | 16x16 | noise | ⚠️ Ignored |
| small_16x16_rgb_checker | 16x16 | checker | ⚠️ Ignored |

**Note:** RGB images use sample-interleave mode which requires triplet processing.
See `src/jpegls/mod.rs` for technical details on the limitation.

## Files

Each test case includes:
- `.raw` - Raw pixel data
- `.jls` - JPEG-LS encoded (CharLS)
- `.txt` - Metadata and compression info

## Regenerating Test Images

```bash
python3 tests/generate_jpegls_test_images.py
```
