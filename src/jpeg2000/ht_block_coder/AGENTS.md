# HTJ2K BLOCK CODER KNOWLEDGE BASE

## OVERVIEW
High-Throughput entropy coders (ISO 15444-15) implementing MEL, VLC, and Mag-Sgn for ultra-fast block coding.

## WHERE TO LOOK

| File | Role | Notes |
|------|------|-------|
| `encoder.rs` | Main Entry Point | 802 lines; coordinates the 3 entropy streams |
| `vlc_ohtj2k.rs` | VLC Tables | 2332 lines; **2000+ lines of static tables** (Critical: Needs data extraction) |
| `mel.rs` | MEL Coder | Magnitude Extension Level coder implementation |
| `vlc.rs` | VLC Coder | Variable Length Coding logic for refinement |
| `mag_sgn.rs` | Mag-Sgn Coder | Magnitude and Sign bitstream processing |
| `coder.rs` | Common Logic | Shared traits and buffer management for HT streams |

## CONVENTIONS

### HTJ2K Entropy Coding
- **Triple-Stream Architecture**: HTJ2K replaces the J2K MQ-coder with three parallel segments: MEL, VLC, and Mag-Sgn.
- **Cleanup Pass Only**: Unlike standard J2K T1 (3 passes), HTJ2K uses a single-pass "Cleanup" mode for massive speedups.
- **Bitstream Reversal**: VLC and Mag-Sgn segments are often written/read in reverse order (backwards) per spec requirements.
- **Word Alignment**: Processing is optimized for 32/64-bit words rather than bit-by-bit.

### Implementation Patterns
- **Table-Heavy**: Heavily relies on the massive LUTs in `vlc_ohtj2k.rs` for state transitions and VLC decoding.
- **Unsafe usage**: Performance-critical loops use `unsafe` for pointer arithmetic to hit HTJ2K throughput targets.
- **No MQ-Coder**: Explicitly avoids the arithmetic MQ-coder used in standard JPEG 2000.

### Technical Debt & Status
- **Status**: Encoder is **Experimental**; Decoder is **Broken** (4 tests failing on pixel reconstruction).
- **Refactoring Target**: `vlc_ohtj2k.rs` is a "data bloat" file. Move these static tables to a separate module or binary file to improve compile times and maintainability.
- **Validation**: Needs strict alignment with ISO 15444-15 Annex C for bitstream packing.
