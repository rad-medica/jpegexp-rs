# JPEG-LS Codec Refactoring Plan

## Executive Summary

This document outlines a comprehensive plan to refactor/redesign the JPEG-LS codec implementation in jpegexp-rs. The current implementation has architectural issues that prevent it from correctly decoding CharLS-encoded images and producing valid output.

## Current Status Assessment

### What Works ✅
- Header parsing (SOI, SOF, SOS markers)
- Frame info extraction (width, height, components, bit depth)
- JPEG marker validation (recently fixed)
- Basic infrastructure and types

### What Doesn't Work ❌
- **Decoder**: Runs out of bits at ~50% of image decoding
- **Encoder**: Produces bitstreams that CharLS cannot decode
- **Multi-component handling**: Returns InvalidOperation error
- **Interleaved mode support**: Not properly implemented

### Key Issues Identified
1. **Byte Stuffing Logic**: Partially fixed, but may need review
2. **Bit Consumption Rate**: Decoder exhausts bitstream prematurely
3. **Golomb Coding**: Possible bugs in encode/decode logic
4. **Context Management**: May not be following JPEG-LS spec correctly
5. **Run Mode vs Regular Mode**: Transitions may be incorrect

## Testing Philosophy

### Intentionally Corrupt Test Images
Per project requirements:
- Some test images may be intentionally corrupt (similar to JPEG2000 test suite)
- **If CharLS can decode it**: We should be able to decode it too
- **If CharLS cannot decode it**: It's acceptable if we cannot decode it either
- **Priority**: Focus on correctly handling valid JPEG-LS files first

### Test Suite Approach
1. **Valid Files First**: Ensure all valid JPEG-LS files decode correctly
2. **Reference Compatibility**: Match CharLS behavior on standard test images
3. **Corrupt Files**: Test resilience but don't prioritize over valid file support
4. **Roundtrip Testing**: Our encoder output should decode correctly with both our decoder and CharLS

## Refactoring Strategy

### Phase 1: Analysis & Validation (3-5 days)

#### 1.1 Create Comprehensive Test Suite
**Goal**: Establish baseline for current behavior and target behavior

**Tasks**:
- [ ] Create test images with CharLS:
  - [ ] Grayscale: 8-bit, 16-bit
  - [ ] RGB: 8-bit, interleaved and planar
  - [ ] Various sizes: 8x8, 16x16, 64x64, 256x256
  - [ ] Edge cases: 1x1, non-square dimensions
- [ ] Document CharLS behavior on each test image
- [ ] Document current jpegexp-rs behavior
- [ ] Create validation framework:
  ```rust
  #[test]
  fn test_charls_compatibility_8x8_grayscale() {
      let charls_encoded = load_test_file("charls_8x8_gray.jls");
      let expected_pixels = load_raw_pixels("8x8_gray.raw");
      
      let mut decoder = JpeglsDecoder::new(&charls_encoded);
      decoder.read_header().unwrap();
      let mut decoded = vec![0u8; expected_pixels.len()];
      
      match decoder.decode(&mut decoded) {
          Ok(_) => assert_eq!(decoded, expected_pixels, "Pixel mismatch"),
          Err(e) => {
              // Check if CharLS can decode this file
              if charls_can_decode(&charls_encoded) {
                  panic!("We should be able to decode files CharLS can decode: {}", e);
              } else {
                  println!("CharLS also cannot decode this file (acceptable)");
              }
          }
      }
  }
  ```

#### 1.2 Bit-Level Debugging
**Goal**: Understand where and why decoder fails

**Tasks**:
- [ ] Add detailed bit-level logging:
  - [ ] Log every `read_bits()` call with context
  - [ ] Track cumulative bits consumed per line
  - [ ] Compare with CharLS bit consumption (if possible)
- [ ] Create bit consumption analysis tool:
  ```rust
  struct BitConsumptionTracker {
      bits_by_line: Vec<usize>,
      bits_by_pixel: Vec<usize>,
      total_bits: usize,
  }
  ```
- [ ] Identify specific lines/pixels where consumption diverges

#### 1.3 Reference Implementation Study
**Goal**: Understand correct JPEG-LS implementation

**Tasks**:
- [ ] Study CharLS source code:
  - [ ] Golomb coding implementation
  - [ ] Context management
  - [ ] Run mode handling
  - [ ] Prediction and error calculation
- [ ] Document key algorithms and their parameters
- [ ] Create Rust equivalents of critical functions
- [ ] Compare our implementation line-by-line

### Phase 2: Decoder Fixes (5-7 days)

#### 2.1 Fix Golomb Decoding
**Priority**: HIGH - Likely root cause of premature bit exhaustion

**Current Code**:
```rust
fn decode_mapped_error_value(&mut self, k: i32) -> Result<i32, JpeglsError> {
    let mut value = 0;
    let mut bit_count = 0;
    
    while self.peek_bits(1)? == 0 {
        value += 1;
        bit_count += 1;
        self.skip_bits(1)?;
        if bit_count > 32 {
            return Err(JpeglsError::InvalidData);
        }
    }
    self.skip_bits(1)?;
    
    if k > 0 {
        let remainder = self.read_bits(k)?;
        value = (value << k) | remainder;
    }
    Ok(value)
}
```

**Issues to Investigate**:
- [ ] Is unary code reading correct?
- [ ] Is the bit count limit appropriate?
- [ ] Is the remainder calculation correct?
- [ ] Are we handling edge cases (k=0, large values)?

**Tasks**:
- [ ] Create unit tests for Golomb decoding with known inputs/outputs
- [ ] Compare with CharLS implementation
- [ ] Add bounds checking and error handling
- [ ] Verify against JPEG-LS spec (ITU-T T.87 Annex C)

#### 2.2 Fix Context Management
**Priority**: HIGH - Affects prediction accuracy

**Tasks**:
- [ ] Review `RegularModeContext` implementation:
  - [ ] Verify `compute_golomb_coding_parameter()` logic
  - [ ] Check `update_variables_and_bias()` against spec
  - [ ] Ensure context array indexing is correct (365 contexts)
- [ ] Review `RunModeContext` implementation:
  - [ ] Verify run length encoding/decoding
  - [ ] Check run interruption handling
- [ ] Add context state validation:
  ```rust
  impl RegularModeContext {
      fn validate_state(&self) -> Result<(), String> {
          if self.n < 0 || self.n > self.reset_threshold {
              return Err(format!("Invalid N: {}", self.n));
          }
          // More validation...
          Ok(())
      }
  }
  ```

#### 2.3 Fix Prediction and Reconstruction
**Priority**: MEDIUM - May cause pixel errors

**Tasks**:
- [ ] Verify `compute_predicted_value()` logic:
  - [ ] Check neighbor pixel selection (Ra, Rb, Rc, Rd)
  - [ ] Verify prediction formula against spec
  - [ ] Ensure edge/boundary cases are handled
- [ ] Verify `compute_reconstructed_sample()`:
  - [ ] Check error value application
  - [ ] Ensure clamping to valid range
  - [ ] Handle near-lossless mode correctly

#### 2.4 Fix Multi-Component Handling
**Priority**: HIGH - Currently returns error for RGB images

**Current Issue**:
```rust
if components != 1 {
    return Err(JpeglsError::InvalidOperation);
}
```

**Tasks**:
- [ ] Implement proper interleaved mode support:
  - [ ] Line interleaved mode
  - [ ] Sample interleaved mode (if needed)
- [ ] Implement planar mode (component-by-component):
  - [ ] Handle multiple scan segments
  - [ ] Manage per-component contexts
- [ ] Add tests for:
  - [ ] RGB interleaved
  - [ ] RGB planar
  - [ ] Multi-component with different bit depths

#### 2.5 Fix Scan Termination
**Priority**: MEDIUM - Handle graceful end-of-scan

**Tasks**:
- [ ] Implement proper `end_scan()` logic:
  - [ ] Handle remaining bits in cache
  - [ ] Verify byte alignment
  - [ ] Check for expected padding
- [ ] Handle premature end-of-data gracefully:
  ```rust
  fn peek_bits(&mut self, count: i32) -> Result<i32, JpeglsError> {
      if self.valid_bits < count {
          self.fill_read_cache()?;
      }
      if self.valid_bits < count {
          // Check if we're at expected end of scan
          if self.is_at_scan_end() {
              return Err(JpeglsError::EndOfScan);  // New error type
          }
          return Err(JpeglsError::InvalidData);
      }
      // ...
  }
  ```

### Phase 3: Encoder Fixes (3-5 days)

#### 3.1 Fix Golomb Encoding
**Priority**: HIGH - Must produce valid bitstreams

**Tasks**:
- [ ] Review `encode_mapped_value()` implementation
- [ ] Ensure bit stuffing is correct (FF → FF 00)
- [ ] Verify unary code generation
- [ ] Test with CharLS decoder

#### 3.2 Fix Context Initialization
**Priority**: MEDIUM

**Tasks**:
- [ ] Verify initial context states
- [ ] Ensure per-component contexts are independent
- [ ] Match CharLS initialization

#### 3.3 Fix Bitstream Generation
**Priority**: HIGH

**Tasks**:
- [ ] Review `append_to_bit_stream()` logic
- [ ] Verify byte alignment at scan end
- [ ] Ensure proper marker insertion
- [ ] Test bitstream byte-by-byte against CharLS

### Phase 4: Integration & Testing (2-3 days)

#### 4.1 Roundtrip Testing
**Goal**: Encode with jpegexp-rs, decode with both jpegexp-rs and CharLS

**Tasks**:
- [ ] Create comprehensive roundtrip tests
- [ ] Test all combinations:
  - [ ] Grayscale 8-bit, 16-bit
  - [ ] RGB 8-bit (interleaved and planar)
  - [ ] Various image sizes
  - [ ] Near-lossless mode (if supported)

#### 4.2 Cross-Compatibility Testing
**Goal**: Ensure compatibility with CharLS

**Test Matrix**:
| Encoder | Decoder | Expected Result |
|---------|---------|----------------|
| jpegexp-rs | jpegexp-rs | ✅ Perfect match (lossless) |
| jpegexp-rs | CharLS | ✅ Perfect match (lossless) |
| CharLS | jpegexp-rs | ✅ Perfect match (lossless) |
| CharLS | CharLS | ✅ Perfect match (reference) |

#### 4.3 Performance Testing
**Goal**: Ensure reasonable performance

**Tasks**:
- [ ] Benchmark decode speed vs CharLS
- [ ] Benchmark encode speed vs CharLS
- [ ] Profile and optimize hotspots
- [ ] Target: Within 2x of CharLS performance

#### 4.4 Corrupt File Handling
**Goal**: Handle corrupt files gracefully

**Tasks**:
- [ ] Create intentionally corrupt test files:
  - [ ] Invalid markers
  - [ ] Truncated bitstreams
  - [ ] Invalid header parameters
  - [ ] Corrupted scan data
- [ ] Test behavior vs CharLS:
  - [ ] If CharLS rejects: We should reject with appropriate error
  - [ ] If CharLS accepts: We should accept and match output
  - [ ] If CharLS crashes: We should handle gracefully (no panic)
- [ ] Add comprehensive error handling:
  ```rust
  #[derive(Debug, thiserror::Error)]
  pub enum JpeglsError {
      #[error("Invalid JPEG-LS marker: expected {expected:02X}, found {found:02X}")]
      InvalidMarker { expected: u8, found: u8 },
      
      #[error("Corrupted scan data at position {position}: {reason}")]
      CorruptedScanData { position: usize, reason: String },
      
      #[error("Premature end of bitstream (needed {needed} bits, have {available})")]
      PrematureEndOfStream { needed: i32, available: i32 },
      
      // ... more specific errors
  }
  ```

### Phase 5: Documentation & Finalization (1-2 days)

#### 5.1 Update Documentation
**Tasks**:
- [ ] Update README with JPEG-LS status
- [ ] Document known limitations
- [ ] Add usage examples
- [ ] Update API documentation

#### 5.2 Code Quality
**Tasks**:
- [ ] Remove debug logging
- [ ] Add appropriate comments
- [ ] Run clippy and fix warnings
- [ ] Format code with rustfmt
- [ ] Update CHANGELOG

## Alternative Approach: FFI Integration

If refactoring proves too complex or time-consuming, consider integrating CharLS via FFI:

### Pros
- ✅ Immediate access to proven, spec-compliant implementation
- ✅ Reduced maintenance burden
- ✅ Known performance characteristics
- ✅ Extensive test coverage

### Cons
- ❌ External C++ dependency
- ❌ Build complexity (requires C++ compiler)
- ❌ Less control over implementation
- ❌ FFI overhead (likely minimal)

### Implementation Plan (3-5 days)
1. Create Rust FFI bindings to CharLS C API
2. Wrap with safe Rust interface
3. Integrate with existing codec infrastructure
4. Add build scripts for cross-platform compilation
5. Test thoroughly

## Timeline Summary

| Phase | Duration | Priority |
|-------|----------|----------|
| Phase 1: Analysis & Validation | 3-5 days | HIGH |
| Phase 2: Decoder Fixes | 5-7 days | HIGH |
| Phase 3: Encoder Fixes | 3-5 days | HIGH |
| Phase 4: Integration & Testing | 2-3 days | HIGH |
| Phase 5: Documentation | 1-2 days | MEDIUM |
| **Total (Refactor Approach)** | **14-22 days** | |
| **Alternative (FFI Integration)** | **3-5 days** | |

## Recommendation

**Option 1**: If the goal is to learn and maintain a pure Rust implementation, proceed with the refactoring plan (14-22 days).

**Option 2**: If the goal is to provide working JPEG-LS support quickly, use FFI integration (3-5 days), and consider reimplementing in pure Rust later if needed.

**Hybrid Approach**: Start with FFI integration to get working functionality, then gradually replace with pure Rust implementation module by module (encoder first, then decoder).

## Success Criteria

- [ ] **Decoder**: Successfully decodes all valid CharLS-encoded test images with perfect pixel accuracy
- [ ] **Encoder**: Produces bitstreams that CharLS can decode with perfect pixel accuracy
- [ ] **Roundtrip**: Encode → Decode produces identical output to input
- [ ] **Multi-component**: RGB and grayscale both fully supported
- [ ] **Performance**: Within 2x of CharLS speed
- [ ] **Robustness**: Handles corrupt files gracefully (no panics)
- [ ] **Compatibility**: Matches CharLS behavior on edge cases
- [ ] **Testing**: 100% pass rate on comprehensive test suite

## Resources

### JPEG-LS Specification
- ITU-T T.87 (ISO/IEC 14495-1): Lossless and near-lossless compression
- Available at: https://www.itu.int/rec/T-REC-T.87

### Reference Implementations
- **CharLS**: https://github.com/team-charls/charls (C++)
- **charls-native**: https://github.com/team-charls/charls-native (C API wrapper)

### Testing Resources
- **JPEG-LS Test Images**: http://www.hlevkin.com/06testimages.htm
- **CharLS Test Suite**: https://github.com/team-charls/charls/tree/master/test

### Academic Papers
- Weinberger, M. J., Seroussi, G., & Sapiro, G. (2000). "The LOCO-I lossless image compression algorithm: Principles and standardization into JPEG-LS"

## Next Steps

1. **Immediate**: Review this plan with stakeholders
2. **Week 1**: Execute Phase 1 (Analysis & Validation)
3. **Week 2-3**: Execute Phase 2 (Decoder Fixes)
4. **Week 3-4**: Execute Phase 3 (Encoder Fixes)
5. **Week 4**: Execute Phase 4-5 (Integration, Testing, Documentation)

## Questions for Discussion

1. **Timeline**: Is 2-4 weeks acceptable for JPEG-LS support?
2. **Pure Rust vs FFI**: Which approach is preferred?
3. **Testing**: Should we prioritize corrupt file handling or focus on valid files first?
4. **Features**: Are there specific JPEG-LS features (near-lossless, mapping tables) that are must-have vs nice-to-have?
5. **Performance**: What performance targets are acceptable?
