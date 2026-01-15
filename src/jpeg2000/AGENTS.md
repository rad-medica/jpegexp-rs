# JPEG 2000 & HTJ2K SUB-PROJECT

## OVERVIEW
Implementation of ISO/IEC 15444-1 (J2K) and 15444-15 (HTJ2K) using Discrete Wavelet Transforms (DWT) and multi-layered bitstream encoding.

## WHERE TO LOOK
| Component | Location | Responsibility |
|-----------|----------|----------------|
| **Core Entry** | `encoder.rs`, `decoder.rs` | Main codec loops; 1000+ line monoliths (refactor targets) |
| **Transform** | `dwt.rs` | 5/3 reversible and 9/7 irreversible wavelet transforms |
| **Entropy** | `bit_plane_coder.rs`, `mq_coder.rs` | Tier-1 EBCOT bit-plane coding and MQ-arithmetic coder |
| **Quantization** | `quantization.rs` | Step size calculation and dead-zone scalar quantization |
| **Hierarchical** | `image.rs`, `packet.rs` | Tile -> Resolution -> Sub-band -> Code-block hierarchies |
| **Metadata** | `tag_tree.rs`, `jp2.rs` | Tag tree encoding and JP2 box/header management |
| **HTJ2K** | `ht_block_coder/` | Specialized MEL, VLC, and Mag-Sgn codecs (Part-15) |

## CONVENTIONS
- **Coordinate Space**: Uses the "Canvas Coordinate System" where tiles and resolutions are aligned to a global grid.
- **Fixed-Point Arithmetic**: Irreversible 9/7 DWT and quantization use fixed-point math; check `dwt.rs` for scaling factors.
- **Layered Structure**: Data is organized by quality layers; packetization logic in `packet.rs` handles progression orders (LRCP/RLCP).
- **Sub-band Indexing**: Sub-bands are ordered as LL, HL, LH, HH per resolution level.

## ANTI-PATTERNS
- **Quantization Bias**: **BUG:** >8-bit complex patterns (gradients/noise) currently fail interop. Suspect quantization step size logic in `quantization.rs`.
- **8-bit Assumptions**: MQ coder and bit-plane logic must remain depth-agnostic; avoid hardcoded 8-bit shifts.
- **HTJ2K Decoder**: **CRITICAL:** HTJ2K decoder is currently non-functional; do not attempt to use for reconstruction.
- **Monolith Bloat**: Do not add new features to `encoder.rs`. Move packetization or marker writing to `packet.rs` or `writer.rs`.
- **Unverified Gains**: Sub-band energy gains for irreversible transforms are frequently miscalculated; verify against Part-1 Annex E.
