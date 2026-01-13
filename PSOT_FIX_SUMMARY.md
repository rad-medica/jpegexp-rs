# JPEG 2000 Psot Bug Fix - 2026-01-12

## Critical Bug Fixed

**Component**: SOT (Start of Tile) marker - Psot field  
**File**: `src/jpeg2000/encoder.rs` lines 644, 653  
**Severity**: High - Standard compliance violation

### The Bug

1. **Incorrect Psot calculation**: Included EOC marker in tile length
   ```rust
   // BEFORE (incorrect)
   let tile_total_len = tile_part_header_len + 2 + total_packet_len + 2; // Includes EOC
   ```

2. **Psot always zero**: Encoder wrote `Psot=0` instead of actual length
   ```rust
   // BEFORE (incomplete)
   writer.write_sot(0, 0, 0, 1)?; // Psot=0
   ```

### The Fix

```rust
// Calculate Psot WITHOUT EOC (per ISO 15444-1 A.4.2)
let tile_total_len = tile_part_header_len + 2 + total_packet_len;

// Write actual tile length
writer.write_sot(0, tile_total_len as u32, 0, 1)?;
```

### Standard Reference

ISO/IEC 15444-1 (JPEG 2000 Part 1) Section A.4.2:
> "Psot: Length of this tile-part... measured from the first byte of the 
> SOT marker segment to the end of the bit stream of this tile-part."

**Key point**: EOC marker is NOT part of the tile-part.

### Impact

**Before Fix**:
- All encodings had `Psot=0`
- Decoders couldn't validate tile length
- Some decoders might fail or misbehave

**After Fix**:
- Correct Psot values (e.g., 608 bytes for 64x64 Level 2)
- Standard-compliant SOT markers
- Better decoder compatibility
- ✅ Levels 0-1 still perfect (MAE=0)
- Level 2+ gradient errors unchanged (separate issue)

### Test Results

**Before**: Psot=0 for all files  
**After**: 
- Level 0 (64x64): Psot~1800 bytes
- Level 1 (64x64): Psot~800 bytes  
- Level 2 (64x64): Psot=608 bytes

**Interop Tests**: 128/300 passing (43%) - unchanged  
(Level 2+ gradient issue is separate, under investigation)

### Related Investigation

The Psot fix was discovered during investigation of Level 2+ gradient encoding errors.
See `LEVEL2_INVESTIGATION.md` for full analysis.

### Files Modified

- `src/jpeg2000/encoder.rs`:
  - Line 644: Removed EOC from Psot calculation
  - Line 653: Write actual tile length instead of 0
  
### Verification

```bash
cargo test --release --test compare_packet_structure -- --ignored --nocapture
```

Output shows correct Psot values matching calculated tile lengths.
