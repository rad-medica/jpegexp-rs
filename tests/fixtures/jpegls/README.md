# JPEG-LS Test Images

Generated using CharLS (via imagecodecs) for jpegexp-rs validation.

Total test cases: 22

## Test Cases

| Name | Size | Channels | Bit Depth | Pattern |
|------|------|----------|-----------|----------|
| tiny_8x8_gray_gradient | 8x8 | 1 | 8 | gradient |
| tiny_8x8_gray_noise | 8x8 | 1 | 8 | noise |
| tiny_8x8_gray_checker | 8x8 | 1 | 8 | checker |
| tiny_8x8_gray_solid | 8x8 | 1 | 8 | solid |
| small_16x16_gray_gradient | 16x16 | 1 | 8 | gradient |
| small_32x32_gray_gradient | 32x32 | 1 | 8 | gradient |
| medium_64x64_gray_gradient | 64x64 | 1 | 8 | gradient |
| medium_128x128_gray_gradient | 128x128 | 1 | 8 | gradient |
| large_256x256_gray_gradient | 256x256 | 1 | 8 | gradient |
| rect_16x32_gray_gradient | 16x32 | 1 | 8 | gradient |
| rect_32x16_gray_gradient | 32x16 | 1 | 8 | gradient |
| tiny_8x8_rgb_gradient | 8x8 | 3 | 8 | gradient |
| small_16x16_rgb_gradient | 16x16 | 3 | 8 | gradient |
| small_32x32_rgb_gradient | 32x32 | 3 | 8 | gradient |
| medium_64x64_rgb_gradient | 64x64 | 3 | 8 | gradient |
| small_16x16_rgb_noise | 16x16 | 3 | 8 | noise |
| small_16x16_rgb_checker | 16x16 | 3 | 8 | checker |
| small_16x16_gray16_gradient | 16x16 | 1 | 16 | gradient |
| small_32x32_gray16_gradient | 32x32 | 1 | 16 | gradient |
| edge_1x1_gray | 1x1 | 1 | 8 | solid |
| edge_1x8_gray | 1x8 | 1 | 8 | gradient |
| edge_8x1_gray | 8x1 | 1 | 8 | gradient |

## Files

Each test case includes:
- `.raw` - Raw pixel data
- `.jls` - JPEG-LS encoded (CharLS)
- `.txt` - Metadata and compression info
