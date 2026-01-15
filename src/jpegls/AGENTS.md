# JPEG-LS (LOCO-I) KNOWLEDGE BASE

## OVERVIEW
Implementation of ISO 14495-1 (LOCO-I), a low-complexity predictor-based codec supporting lossless and near-lossless compression with a dual-mode state machine.

## WHERE TO LOOK
| File | Role | Notes |
|------|------|-------|
| `scan_decoder.rs` | Core Decoder (953 lines) | Extreme complexity; handles Run/Regular modes, bitstream parsing, and pixel reconstruction. |
| `scan_encoder.rs` | Core Encoder (700 lines) | Edge-case heavy; handles MED prediction, error quantization, and context-adaptive entropy coding. |
| `traits.rs` | `JpeglsSample` Trait | Essential bit depth abstraction for supporting 2-16 bit `u8`/`u16` samples without code duplication. |
| `regular_mode_context.rs` | Regular Mode State | Manages $A, B, C, N$ variables for gradient-based prediction and adaptive Golomb-Rice coding. |
| `run_mode_context.rs` | Run Mode State | Tracks run-length state machine for high-efficiency coding of identical/near-identical flat regions. |
| `golomb_lut.rs` | Golomb-Rice Tables | Static lookup tables for fast parameter selection during entropy coding cycles. |
| `validate_spiff_header.rs` | SPIFF Support | Header validation for the Still Picture Interchange File Format (ISO 10918-3) commonly used with JLS. |
| `coding_parameters.rs` | Parameter Storage | Definitions for $T_1, T_2, T_3, RESET$ thresholds and Near-lossless error limits. |

## CONVENTIONS
- **State Machine**: Codec toggles between **Run Mode** (flat areas) and **Regular Mode** (textured areas/gradients) based on context.
- **Bit Depth Polymorphism**: All logic uses `JpeglsSample` trait; specialized implementations for 8-bit (`u8`) and 16-bit (`u16`) performance.
- **Context Modeling**: 365 regular contexts and 2 run contexts are maintained per scan to adapt Golomb-Rice parameters ($k$).
- **Sample Interleaving**: Support for Sample-interleaved (triplets), Line-interleaved, and Component-interleaved (None) modes.
- **Near-Lossless**: Controlled via the `NEAR` parameter; when `NEAR > 0`, error quantization and bias correction are active.

## ANTI-PATTERNS
- **Broken Adaptation**: Failing to update $A, B, C, N$ variables after a sample breaks subsequent entropy coding logic.
- **Range Assumptions**: Hardcoding 8-bit limits; must respect `MAXVAL` and `RANGE` from `JpeglsPcParameters`.
- **Interleave Mismatch**: Assuming component-at-a-time; RGB often arrives as interleaved triplets in a single scan.
- **Near-Lossless Leakage**: Incorrectly applying the `NEAR` quantization in lossless mode (where `NEAR=0` must be strictly enforced).
- **Run-Length Alignment**: Misaligning bitstream position after run-length sequences due to incorrect bit-stuffing handling.

## STATUS
- **Decoder**: ✅ 100% validated against CharLS and standard test suites (23/23 passing).
- **Encoder**: ⚠️ 61.3% interop (98/160); current 10/12-bit implementation has compatibility issues with CharLS CLI tool.

