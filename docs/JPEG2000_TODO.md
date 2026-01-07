# JPEG 2000 Implementation Progress

## Status Overview (Updated Jan 2026)

### Encoder
- **Core Coding**: ✅ Working
- **Color Transform**: ✅ RCT (Reversible) working for 8-bit. ⚠️ 12-bit has issues.
- **DWT**: ✅ 5-3 Reversible working.
- **Quantization**: ✅ Scalar derived.
- **Tier-1 (EBCOT)**: ✅ Working.
- **Tier-2 (Packetization)**: ✅ Working for self-decode.
- **Interoperability**: ⚠️ OpenJPEG can parse our files (No crashes), but decoded pixel values are incorrect (MAE > 0).

### Decoder
- **Parsing**: ✅ Working for self-encoded files.
- **Tier-2**: ✅ Working.
- **Tier-1**: ✅ Working.
- **Color**: ✅ 8-bit RGB working.
- **Interoperability**: ⚠️ Cannot decode OpenJPEG lossless files (Packet header length mismatch remains for reading OpenJPEG files).

## Detailed Status

| Feature | Status | Notes |
|---------|--------|-------|
| **Lossless Grayscale** | ✅ Ready | 8-bit, 12-bit working. MAE=0. |
| **Lossless RGB** | ✅ Ready | 8-bit working. 12-bit issues (ignored in tests). |
| **Large Images** | ✅ Ready | 512x512 tested. |
| **OpenJPEG Compat** | ⚠️ Partial | "Segment too long" fixed by implementing Comma Code for LBlock. Files parse, but pixel values mismatch. |
| **HTJ2K** | ⚠️ Partial | Encoder structure exists, Decoder parses CAP. |

## Known Issues

1.  **OpenJPEG Value Mismatch**: OpenJPEG decoding of our files results in MAE ~44. Likely due to `ZeroBitPlane` or `M_b` calculation mismatch causing MSB loss.
2.  **12-bit RGB**: Produces artifacts (MAE > 0). Likely due to `RCT` dynamic range expansion handling.

## Fixed Issues

1.  **Packet Header "Segment too long"**: Fixed by replacing `TagTree` encoding for `LBlock` with **Comma Code** (Unary code), as mandated by the standard and OpenJPEG implementation. This inverted the bit logic (0->0 instead of 0->1).

## Next Steps

1.  Investigate `M_b` / `Guard Bits` mismatch causing data loss in OpenJPEG decoding.
2.  Fix 12-bit RGB.
3.  Implement HTJ2K integration.
