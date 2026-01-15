# JPEG 1 CODEC KNOWLEDGE BASE

## OVERVIEW
Classic DCT/Huffman implementation supporting Baseline, Extended (12-bit), Progressive, and Lossless (SOF3) modes with 100% interop success.

## WHERE TO LOOK
| Component | File | Responsibilities |
|-----------|------|------------------|
| **Encoder** | `encoder.rs` | Main entry point for all modes; handles 8/12-bit and planar/interleaved input. |
| **Decoder** | `decoder.rs` | Marker-driven stream reconstruction and Huffman/DCT dispatch. |
| **Entropy** | `huffman.rs` | Table generation, AC/DC coding logic, and bitstream orchestration. |
| **Transform** | `dct.rs` | Manual fixed-point FDCT and IDCT implementations for deterministic precision. |
| **Progressive** | `progressive.rs` | Successive approximation and spectral selection scan management. |
| **Lossless** | `lossless.rs` | ISO 10918-1 SOF3 predictor-based encoding (non-DCT). |
| **Quantization**| `quantization.rs` | Quantization table management and dequantization logic. |
| **Module** | `mod.rs` | Public API exports and component wiring. |

## CONVENTIONS
- **Fixed-Point Arithmetic**: All DCT and quantization logic must use fixed-point math to ensure 100% bit-identical results (MAE=0) across all supported architectures.
- **12-bit Extended Support**: Logic in `encoder.rs` and `decoder.rs` must correctly handle the shift from 8-bit to 12-bit precision as defined in the Extended Sequential (SOF1) profile.
- **Marker Dispatch**: The decoder uses a centralized marker-matching pattern; new marker handlers should be integrated into the main `decode` loop in `decoder.rs`.
- **Predictor Selection**: Lossless mode strictly follows the eight ISO 10918-1 predictors (0-7), ensuring perfect reconstruction.
- **Baseline Interoperability**: Prioritize compatibility with standard JPEG decoders (libjpeg-turbo, etc.) for Baseline and Progressive modes.
- **Signed Sample Handling**: Medical DICOM images often contain signed samples; ensure level-shifting offsets are correctly handled during DCT/IDCT.

## ANTI-PATTERNS
- **CRITICAL: Encoder Duplication**: `encoder.rs` (2468 lines) contains 3x duplicated logic for `u8`, `u16`, and `planar` buffers. **DO NOT** replicate this pattern. New features must implement a `PixelSource` trait abstraction.
- **Table Bloat**: Do not define custom Huffman or Quantization tables within `encoder.rs`. Use the centralized management in `huffman.rs` or `quantization.rs`.
- **Floating Point**: Avoid `f32`/`f64` in the hot path of DCT or quantization to maintain cross-platform bit-perfection and avoid non-deterministic rounding.
- **Unchecked Bit-stream**: Always verify bitstream alignment and byte-stuffing (0xFF00) when switching between scans or components.
- **Magic Numbers**: Avoid hardcoding block dimensions (8x8) or component limits; use constants and metadata from `FrameInfo`.
- **Marker Conflicts**: Do not use marker codes outside the centralized definitions in `src/jpeg_marker_code.rs`.
