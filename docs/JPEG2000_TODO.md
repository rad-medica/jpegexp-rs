# JPEG 2000 Implementation Progress

## Status Overview (Updated Jan 2026)

### Encoder
- **Core Coding**: ✅ Working
- **Color Transform**: ✅ RCT (Reversible) working for 8-bit. ⚠️ 12-bit has issues.
- **DWT**: ✅ 5-3 Reversible working.
- **Quantization**: ✅ Scalar derived.
- **Tier-1 (EBCOT)**: ✅ Working. Fixed BPC state management (VISITED flag logic) and ZC Contexts.
- **Tier-2 (Packetization)**: ✅ Working for self-decode.
- **Interoperability**: ⚠️ OpenJPEG can parse our files (No crashes), but decoded pixel values are incorrect (MAE ~1442 for 12-bit).

### Decoder
- **Parsing**: ✅ Working for self-encoded files.
- **Tier-2**: ✅ Working.
- **Tier-1**: ✅ Working.
- **Color**: ✅ 8-bit RGB working.
- **Interoperability**: ⚠️ Cannot decode OpenJPEG lossless files completely (Header works, but pixel data mismatch ~1524 MAE).

## Detailed Status

| Feature | Status | Notes |
|---------|--------|-------|
| **Lossless Grayscale** | ✅ Ready | 8-bit, 12-bit working (self-roundtrip MAE=0). |
| **Lossless RGB** | ✅ Ready | 8-bit working. 12-bit issues. |
| **Large Images** | ✅ Ready | 512x512 tested. |
| **OpenJPEG Compat** | ⚠️ Partial | Files parse, markers match, but pixel values differ. Likely Level Shift or Signedness mismatch. |
| **HTJ2K** | ⚠️ Partial | Encoder structure exists, Decoder parses CAP. |

## Fixed Issues

1.  **BPC Context State**: Fixed critical bug where `VISITED` state was cleared prematurely after `SigProp` pass.
2.  **ZC Contexts**: Fixed incorrect LH/HL orientation logic in Zero Coding context selection (swapped H/V priority).
3.  **Bit Stuffing**: Fixed `J2kBitWriter` to correctly handle `0xFF` stuffing (inserting `0` bit, not `0x00` byte).
4.  **16-bit PGM**: Added support for reading/writing 16-bit PGM (Big Endian) in CLI and tests.
5.  **12-bit Packing**: Fixed Little Endian packing/unpacking for `u16` pixel data.

## Known Issues

1.  **OpenJPEG Value Mismatch**: OpenJPEG decoding of our files results in high MAE. This occurs even though our self-roundtrip is bit-perfect. The mismatch remains after fixing BPC/ZC/Stuffing.
2.  **12-bit RGB**: Produces artifacts. Likely due to RCT dynamic range expansion handling.

## Next Steps

1.  Investigate Interop MAE issue with OpenJPEG (Level shifting logic).
2.  Fix 12-bit RGB.
3.  Implement HTJ2K integration.
