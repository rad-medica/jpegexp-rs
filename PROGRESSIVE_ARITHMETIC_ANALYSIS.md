# Progressive Encoder & Arithmetic Coding - Implementation Analysis

**Date**: January 10, 2026  
**Status**: Analysis Complete - Implementation Deferred  
**Reason**: Complexity & Time Requirements Exceed Single Session Scope

---

## Executive Summary

After analyzing both features, implementing **progressive encoder** and **arithmetic coding** properly requires **~28 hours of focused development**:

- **Progressive Encoder (SOF2)**: ~12 hours
- **Arithmetic Coding (SOF9-11)**: ~16 hours

This exceeds what's feasible in a single session. Below is a comprehensive implementation plan for future work.

---

## Feature 1: Progressive Encoder (SOF2)

### Complexity Assessment: HIGH

**Estimated Effort**: 12-15 hours  
**Lines of Code**: ~800-1000 lines  
**Standard Reference**: ISO/IEC 10918-1 Annex G

### Why It's Complex

1. **Multi-Scan Architecture**: Requires complete redesign of encoding pipeline
   - Current encoder: Single-pass, writes blocks immediately
   - Progressive: Must buffer ALL DCT coefficients, then encode in multiple passes

2. **Spectral Selection**: Encode different frequency bands separately
   - DC coefficients (frequency 0)
   - Low AC frequencies (1-5)
   - High AC frequencies (6-63)

3. **Successive Approximation**: Encode bit planes separately
   - First scan: High bits (MSBs)
   - Refinement scans: Low bits (LSBs)

4. **Complex Scan Planning**: Must generate valid scan sequences
   - Spectral-only scans
   - Successive approximation scans
   - Combined scans
   - Must comply with JPEG standard restrictions

5. **EOB Run Handling**: Special encoding for sequences of all-zero blocks

### Implementation Phases (12h breakdown)

#### Phase 1: Architecture & Data Structures (3h)

**Create new module**: `src/jpeg1/progressive.rs`

```rust
/// Defines a single progressive scan
pub struct ProgressiveScan {
    component_indices: Vec<usize>,  // Which components (Y, Cb, Cr)
    ss: u8,  // Spectral selection start (0-63)
    se: u8,  // Spectral selection end (0-63)
    ah: u8,  // Successive approximation high bit
    al: u8,  // Successive approximation low bit
}

/// Buffer for all DCT coefficients
pub struct CoefficientBuffer {
    // Store coefficients for entire image
    // [component][block_y][block_x][64 coefficients]
    components: Vec<Vec<Vec<[i16; 64]>>>,
}

/// Standard progressive scan scripts
pub fn standard_scans_simple() -> Vec<ProgressiveScan>;
pub fn standard_scans_sa() -> Vec<ProgressiveScan>;
```

**Tasks**:
1. Create scan definition structures
2. Implement coefficient buffer (memory-efficient storage)
3. Create standard scan scripts (3-scan, 6-scan, 10-scan modes)
4. Add scan validation (ensure scans are legal per JPEG spec)

#### Phase 2: Coefficient Collection (2h)

**Modify**: `src/jpeg1/encoder.rs`

```rust
impl Jpeg1Encoder {
    fn encode_progressive(
        &mut self,
        source: &[u8],
        frame_info: &FrameInfo,
        destination: &mut [u8],
        scans: &[ProgressiveScan],
    ) -> Result<usize, JpeglsError> {
        // 1. Convert RGB to YCbCr (existing logic)
        // 2. For each component, for each 8x8 block:
        //    - Apply DCT
        //    - Quantize
        //    - Store coefficients in buffer (DO NOT ENCODE YET)
        // 3. Once all coefficients buffered, encode scans
    }
}
```

**Tasks**:
1. Modify DCT/quantization to buffer instead of encode
2. Handle subsampling (4:2:0, 4:2:2) during collection
3. Calculate total memory requirements
4. Add memory limit checks (prevent OOM on huge images)

#### Phase 3: DC-First Scan Encoding (2h)

**Implement**: Spectral selection SS=0, SE=0

```rust
fn encode_dc_first_scan(
    &mut self,
    buffer: &CoefficientBuffer,
    scan: &ProgressiveScan,
    writer: &mut JpegStreamWriter,
) -> Result<(), JpeglsError> {
    // For each component in scan.component_indices:
    //   For each block:
    //     DC = block[0] >> scan.al  // Shift for successive approximation
    //     diff = DC - previous_DC
    //     Encode diff using Huffman DC table
}
```

**Tasks**:
1. Extract DC coefficients from buffer
2. Encode with successive approximation shift
3. Track DC predictor per component
4. Write SOS marker with SS=0, SE=0, Ah=0, Al=scan.al

#### Phase 4: AC-First Scan Encoding (3h)

**Implement**: Spectral selection SS > 0

```rust
fn encode_ac_first_scan(
    &mut self,
    buffer: &CoefficientBuffer,
    scan: &ProgressiveScan,
    writer: &mut JpegStreamWriter,
) -> Result<(), JpeglsError> {
    let mut eob_run = 0u16;
    
    // For each block:
    //   Extract AC coefficients in range [SS..SE]
    //   Shift by 'al' for successive approximation
    //   Encode using run-length coding
    //   Handle EOB runs (special encoding for all-zero blocks)
}
```

**Challenges**:
- EOB run handling (can span up to 32767 blocks)
- Zero run length encoding while respecting spectral band
- Flushing EOB runs at end of scan

#### Phase 5: Refinement Scans (2h)

**Implement**: Successive approximation Ah > 0

```rust
fn encode_dc_refinement_scan(...) -> Result<(), JpeglsError> {
    // For each DC coefficient:
    //   Extract bit at position 'al'
    //   Write single bit (no Huffman, just raw bit)
}

fn encode_ac_refinement_scan(...) -> Result<(), JpeglsError> {
    // For each AC coefficient in range [SS..SE]:
    //   If coefficient != 0:
    //     Extract bit at position 'al'
    //     Write bit
    //   Track EOB runs for all-zero blocks
}
```

**Challenges**:
- Must track which coefficients were already nonzero
- EOB run handling in refinement scans
- Correct bit extraction from signed coefficients

### Testing Strategy (Included in 12h)

1. **Unit Tests**: Test each scan type independently
2. **Integration Tests**: Encode with standard scan scripts, decode with existing decoder
3. **Interop Tests**: Compare with libjpeg-turbo progressive output
4. **Visual Validation**: Verify progressive rendering (low-res to high-res)

### Expected Outcomes

✅ **File Format**:
- SOF2 marker (Start of Frame Progressive)
- Multiple SOS segments (one per scan)
- Correct spectral selection and successive approximation parameters

✅ **Compatibility**:
- Decodable by existing jpegexp-rs decoder
- Decodable by libjpeg-turbo, Firefox, Chrome

✅ **File Size**:
- Similar to baseline JPEG (~±5%)
- Slightly larger due to scan overhead

✅ **Use Cases**:
- Web progressive loading
- Large image previews
- Bandwidth-constrained delivery

---

## Feature 2: Arithmetic Coding (SOF9-SOF11)

### Complexity Assessment: VERY HIGH

**Estimated Effort**: 16-20 hours  
**Lines of Code**: ~1200-1500 lines  
**Standard Reference**: ISO/IEC 10918-1 Annex D

### Why It's VERY Complex

1. **Binary Arithmetic Coder**: Low-level bit-precise implementation
   - Q-coder or QM-coder variants
   - Probability estimation
   - Renormalization
   - Bit stuffing (avoid 0xFF bytes)

2. **Context Modeling**: Different contexts for different coefficients
   - DC contexts
   - AC zero/nonzero contexts
   - Magnitude contexts
   - Sign contexts

3. **Two Implementations Needed**:
   - Encoder (arithmetic encoding)
   - Decoder (arithmetic decoding)

4. **Rare Usage**: Limited real-world testing available
   - Few test images
   - Few decoders support it
   - Patent history made it unpopular

5. **Precision Requirements**: Requires exact bit-level accuracy
   - Integer overflow handling
   - Carry propagation
   - Bit stuffing edge cases

### Implementation Phases (16h breakdown)

#### Phase 1: Binary Arithmetic Coder Core (5h)

**Create**: `src/jpeg1/arithmetic.rs`

```rust
/// QM-coder state machine
struct QMCoder {
    a: u32,      // Probability interval
    c: u32,      // Code register
    ct: i32,     // Bit counter
    buffer: u8,  // Byte buffer
    // ... state tables
}

impl QMCoder {
    fn encode_bit(&mut self, bit: bool, context: &mut Context) -> Result<(), Error>;
    fn decode_bit(&mut self, context: &mut Context) -> Result<bool, Error>;
    fn renormalize_encoder(&mut self) -> Result<(), Error>;
    fn renormalize_decoder(&mut self) -> Result<(), Error>;
}

/// Context for probability estimation
struct Context {
    mps: bool,    // More probable symbol
    index: u8,    // State index in probability table
}
```

**Tasks**:
1. Implement QM-coder state machine per Annex D
2. Create probability estimation tables (Qe values)
3. Implement renormalization (maintain precision)
4. Implement bit stuffing (avoid 0xFF marker conflicts)
5. Add extensive unit tests (bit-level accuracy critical)

#### Phase 2: Context Modeling (3h)

```rust
/// Context bins for arithmetic coding
struct ArithmeticContexts {
    dc_contexts: Vec<Context>,     // DC magnitude
    ac_zero_contexts: Vec<Context>,  // AC zero/nonzero
    ac_mag_contexts: Vec<Context>,   // AC magnitude
    ac_sign_contexts: Vec<Context>,  // AC sign
}

impl ArithmeticContexts {
    fn get_dc_context(&mut self, prev_dc: i16, component: usize) -> &mut Context;
    fn get_ac_context(&mut self, position: usize, neighbors: &[i16]) -> &mut Context;
}
```

**Tasks**:
1. Define context bins per standard
2. Implement context selection logic
3. Initialize probability states
4. Handle context adaptation (update probabilities)

#### Phase 3: Arithmetic Encoder Integration (4h)

**Modify**: `src/jpeg1/encoder.rs`

```rust
impl Jpeg1Encoder {
    fn encode_arithmetic(
        &mut self,
        source: &[u8],
        frame_info: &FrameInfo,
        destination: &mut [u8],
    ) -> Result<usize, JpeglsError> {
        // Similar to baseline encode, but use arithmetic coder
        // instead of Huffman
    }
    
    fn encode_block_arithmetic(
        &mut self,
        block: &[f32; 64],
        contexts: &mut ArithmeticContexts,
        coder: &mut QMCoder,
    ) -> Result<(), JpeglsError> {
        // Encode DC using arithmetic coder
        // Encode AC using arithmetic coder
        // Use context modeling for probability estimation
    }
}
```

**Tasks**:
1. Replace Huffman encoding with arithmetic encoding
2. Integrate context modeling
3. Write SOF9 marker (arithmetic sequential)
4. Handle restart markers (reset arithmetic coder state)

#### Phase 4: Arithmetic Decoder Implementation (4h)

**Modify**: `src/jpeg1/decoder.rs`

```rust
impl Jpeg1Decoder {
    fn decode_arithmetic(&mut self, destination: &mut [u8]) -> Result<(), JpeglsError> {
        // Decode using arithmetic decoder
        // Must match encoder context selection exactly
    }
    
    fn decode_block_arithmetic(
        &mut self,
        contexts: &mut ArithmeticContexts,
        coder: &mut QMCoder,
    ) -> Result<[i16; 64], JpeglsError> {
        // Decode DC
        // Decode AC
        // Inverse zigzag, dequantize, IDCT (same as baseline)
    }
}
```

**Tasks**:
1. Implement arithmetic decoder (inverse of encoder)
2. Handle bit unstuffing
3. Synchronize context selection with encoder
4. Extensive testing (encoder/decoder must match exactly)

### Testing Strategy

**Critical**: Arithmetic coding requires bit-exact correctness

1. **Unit Tests**: Test coder core with known bit sequences
2. **Roundtrip Tests**: Encode → Decode, verify MAE < threshold
3. **Reference Tests**: Compare with reference implementation (if available)
4. **Edge Cases**: Test overflow, carry, bit stuffing

### Expected Outcomes

✅ **Compression**: 5-10% better than Huffman (same visual quality)  
✅ **Compatibility**: ISO/IEC 10918-1 compliant  
❌ **Adoption**: Very limited (most decoders don't support it)

### Why Arithmetic Coding is Low Priority

1. **Limited Support**: Most JPEG decoders don't support arithmetic coding
2. **Patent History**: Historically patent-encumbered (free since 2015, but stigma remains)
3. **Complexity**: High implementation cost for marginal benefit
4. **Alternatives**: Optimized Huffman gives similar gains with better compatibility

**Recommendation**: Defer arithmetic coding unless specifically required.

---

## Alternative: Quick Wins

Instead of progressive/arithmetic (28h), consider:

### 1. Optimized Huffman Tables (~4h) - **RECOMMENDED**

**Benefit**: 5-15% automatic file size reduction  
**Complexity**: Moderate  
**Compatibility**: 100% (all decoders support it)

**Implementation**:
```rust
// Two-pass encoding:
// Pass 1: Collect symbol frequencies
// Pass 2: Build optimal Huffman tables, encode

fn collect_statistics(&mut self, ...) -> SymbolFrequencies;
fn build_optimal_huffman(freqs: &SymbolFrequencies) -> HuffmanTable;
```

### 2. Advanced Subsampling Modes (~2h)

- 4:1:1 subsampling
- 4:4:0 subsampling
- Better downsampling filters (Lanczos, Mitchell)

### 3. Quality Presets (~1h)

```rust
encoder.set_preset(JpegPreset::Web);        // Quality 85, 4:2:0, optimized Huffman
encoder.set_preset(JpegPreset::Photography); // Quality 95, 4:2:2
encoder.set_preset(JpegPreset::Archival);   // Quality 100, 4:4:4
```

---

## Conclusion

### Progressive Encoder (SOF2): Feasible but Time-Intensive

- ✅ **Technically feasible** with existing decoder as reference
- ⚠️ **Requires 12-15 hours** of focused implementation
- ✅ **High value** for web use cases
- ✅ **Well-documented** in JPEG standard

**Recommendation**: Implement as dedicated project, not quick feature addition.

### Arithmetic Coding (SOF9-11): Complex with Limited Value

- ⚠️ **Very complex** implementation (16-20 hours)
- ⚠️ **Limited adoption** in real world
- ⚠️ **Difficult to test** (few reference implementations)
- ❌ **Low ROI** (marginal compression gains, poor compatibility)

**Recommendation**: Defer unless explicitly required by user.

### Suggested Next Steps

1. **Immediate** (Next Session):
   - Implement **Optimized Huffman Tables** (4h, high ROI)
   - Create quality presets (1h, user-friendly)

2. **Near-Term** (Future Session):
   - Implement **Progressive Encoder** (12h, web optimization)

3. **Long-Term** (If Needed):
   - Arithmetic Coding (16h, low priority)

---

## Implementation Checklist (For Future Work)

### Progressive Encoder
- [ ] Create `progressive.rs` module
- [ ] Implement `ProgressiveScan` struct
- [ ] Implement `CoefficientBuffer`
- [ ] Create standard scan scripts
- [ ] Modify encoder for coefficient buffering
- [ ] Implement DC-first encoding
- [ ] Implement AC-first encoding
- [ ] Implement refinement scans
- [ ] Add SOF2 marker writing
- [ ] Create integration tests
- [ ] Validate with existing decoder
- [ ] Cross-test with libjpeg-turbo

### Arithmetic Coding
- [ ] Create `arithmetic.rs` module
- [ ] Implement QM-coder core
- [ ] Create probability tables
- [ ] Implement context modeling
- [ ] Integrate encoder
- [ ] Integrate decoder
- [ ] Add SOF9 marker support
- [ ] Create unit tests (bit-level accuracy)
- [ ] Create roundtrip tests
- [ ] Find reference test images

---

**Total Estimated Effort**: 28+ hours (Progressive + Arithmetic)  
**Recommended Approach**: Implement Optimized Huffman first (4h), then Progressive if needed (12h)  
**Arithmetic Coding**: Defer unless explicitly required
