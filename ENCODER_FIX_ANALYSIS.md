# JPEG2000 Encoder Multi-Resolution Packet Bug - Root Cause Analysis

## Problem Statement

Encoder generates codeblocks for all resolutions (verified by debug), but achieves MAE=4.22 instead of MAE=0 for lossless encoding.

Test case: 64x64 image with 5 decomposition levels, alternating 0/255 pattern.

## Detailed Code Analysis

### 1. Packet Generation Loop (Lines 482-669)

Structure:
```rust
for res in 0..num_resolutions {  // Lines 482-669
    let mut precinct_state = PrecinctState::new(0, 0);  // Line 516
    let mut packet_header = PacketHeader { ... };  // Line 517
    let mut packet_body = Vec::new();  // Line 524
    
    for band in 0..num_bands {  // Lines 526-646
        // Process subband codeblocks, add to packet_header.included_cblks
        // Extend packet_body with encoded data
    }
    
    // Write packet header and push packet  // Lines 654-668
    packets.push(Packet {
        resolution: res as u8,
        header_data: header_data,
        body_data: packet_body,
    });
}
```

**VERIFICATION**: Each resolution creates FRESH packet_body and precinct_state.
This is CORRECT.

### 2. Subband Grid Calculations (Lines 495-513)

```rust
for band in 0..num_bands {
    let (sb_w, sb_h) = if res == 0 {
        (ll_w, ll_h)
    } else {
        let (prev_w, prev_h) = self.get_ll_size(width, height, num_levels as usize, res - 1);
        match band {
            0 => (ll_w - prev_w, prev_h), // HL
            1 => (prev_w, ll_h - prev_h), // LH
            2 => (ll_w - prev_w, ll_h - prev_h), // HH
            _ => (0, 0),
        }
    };
    let gw = (sb_w + cb_dim - 1) / cb_dim;  // Line 510
    let gh = (sb_h + cb_dim - 1) / cb_dim;  // Line 511
    subband_grids.push((gw, gh));
}
```

**POTENTIAL ISSUE**: Ceiling division formula `(sb_w + cb_dim - 1) / cb_dim` 
should be correct for positive integers, but using proper div_ceil is safer.

### 3. DWT Coefficient Layout Analysis

The forward DWT (lines 405-464) arranges coefficients as:
- After each level: LL at [0..ll_w, 0..ll_h], HL/LH/HH in remaining areas
- Layout is interleaved: rows have LL|HL, columns are split vertically

The extract_subband_coeffs (lines 800-857) extracts using:
- Resolution 0: First ll_w × ll_h elements
- Higher resolutions: Calculated offsets based on LL sizes

**This should match.**

### 4. Empty Packet Handling (Lines 650-652)

```rust
if packet_header.included_cblks.is_empty() {
    packet_header.empty = true;
}
// packet_body is not explicitly cleared if empty!
```

**ISSUE**: If packet is marked empty, packet_body might contain stale data.

## Root Cause Hypothesis

The MAE=4.22 error is relatively small, suggesting:
1. Basic structure is correct (packets are being created and written)
2. Issue is in subtle data corruption or precision loss

**Most likely causes:**

1. **Ceiling division edge cases** in grid calculation (lines 510-511)
2. **Empty packet body not cleared** when marked empty (after line 652)
3. **Subtle DWT/extraction mismatch** for edge cases (e.g., when dimensions are not powers of 2)

## Required Fixes

### Fix 1: Use explicit div_ceil for grid calculations (Priority: MEDIUM)

Location: encoder.rs lines 510-511

```rust
// Replace:
let gw = (sb_w + cb_dim - 1) / cb_dim;
let gh = (sb_h + cb_dim - 1) / cb_dim;

// With:
let gw = (sb_w + cb_dim - 1).div_ceil(cb_dim);
let gh = (sb_h + cb_dim - 1).div_ceil(cb_dim);
```

Add helper if needed:
```rust
trait DivCeil {
    fn div_ceil(&self, divisor: usize) -> usize;
}
impl DivCeil for usize {
    fn div_ceil(&self, divisor: usize) -> usize {
        (self + divisor - 1) / divisor
    }
}
```

### Fix 2: Clear packet_body for empty packets (Priority: LOW)

Location: encoder.rs after line 652

```rust
if packet_header.included_cblks.is_empty() {
    packet_header.empty = true;
    packet_body.clear();  // ADD THIS LINE
}
```

### Fix 3: Debug verification (Priority: HIGH)

Add comprehensive debug output to trace:
1. Packet count per resolution
2. Packet body sizes (header_data.len(), packet_body.len())
3. Subband grid dimensions
4. Extracted coefficient ranges

## Testing Protocol

1. Add debug output
2. Run: `cargo test --test openjpeg_compat_test test_lossless_self_roundtrip -- --nocapture`
3. Verify 6 packets are generated for 64x64 with 5 levels
4. Verify all 6 packets have non-zero body data (unless truly empty)
5. Verify MAE = 0.0 after fixes

## Expected Outcome

After applying these fixes:
- 64x64 image with 5 decomposition levels should encode/decode losslessly
- MAE should be 0.0 for 5-3 reversible transform
- All 6 resolution packets should be written correctly
