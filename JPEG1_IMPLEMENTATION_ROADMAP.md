# JPEG 1 Implementation Roadmap - Remaining Work

**Last Updated**: January 10, 2026  
**Current Compliance**: 70%  
**Target Compliance**: 90%+ (practical full implementation)

---

## ✅ Completed Features (4/8 tasks)

1. ✅ **Lossless Encoder (SOF3)** - Complete with all 7 predictors, MAE=0 reconstruction
2. ✅ **10-bit Precision** - Extended to 8-16 bit range
3. ✅ **Comprehensive Testing** - 11 new integration tests, 100% pass rate
4. ✅ **Documentation** - 3 technical guides + updated compliance matrix

---

## 🔄 Remaining Features (4/8 tasks - Deferred)

### 1. Color Subsampling Encoder (4:2:2, 4:2:0) - **HIGH PRIORITY**

**Status**: API framework complete, encoding logic pending  
**Estimated Effort**: ~6 hours  
**Complexity**: Moderate to High  
**Value**: High (web optimization, smaller files)

**What's Already Done**:
- ✅ Sampling factor fields in encoder struct
- ✅ Convenience methods: `set_subsampling_420()`, `set_subsampling_422()`, `set_subsampling_444()`
- ✅ SOF segment writing with custom sampling factors

**What's Needed**:
1. **Chroma Downsampling Logic**:
   ```rust
   // For 4:2:0: Average 2x2 pixel blocks for Cb/Cr
   fn downsample_420(cb_full: &[f32], cr_full: &[f32], width: usize, height: usize) 
       -> (Vec<f32>, Vec<f32>)
   ```

2. **MCU Reorganization**:
   - Current: 1 MCU = 1 Y block + 1 Cb block + 1 Cr block (4:4:4)
   - 4:2:2: 1 MCU = 2 Y blocks + 1 Cb block + 1 Cr block
   - 4:2:0: 1 MCU = 4 Y blocks + 1 Cb block + 1 Cr block

3. **Block Encoding Updates**:
   - Modify `encode()` loop to handle variable blocks per MCU
   - Adjust component indexing for subsampled data

**Implementation Steps**:
1. Implement downsampling functions (average pooling)
2. Modify MCU loop to encode multiple Y blocks per MCU
3. Update block indexing to match subsampling ratios
4. Write sampling factors to SOF segment
5. Add tests for 4:2:0 and 4:2:2 modes

**Testing Strategy**:
- Compare file sizes (4:2:0 should be ~50% smaller than 4:4:4)
- Validate with libjpeg-turbo decoder
- Verify visual quality (expect slight chroma degradation)

**Reference**: ISO/IEC 10918-1 Annex A (MCU definition)

---

### 2. Progressive Encoder (SOF2) - **HIGH PRIORITY**

**Status**: Not started (decoder already complete)  
**Estimated Effort**: ~12 hours  
**Complexity**: High  
**Value**: High (web optimization, progressive loading)

**What's Needed**:
1. **Spectral Selection** (SS/SE parameters):
   - DC-first scan: Encode only DC coefficients (SS=0, SE=0)
   - AC scans: Encode frequency bands (e.g., SS=1, SE=5 for low frequencies)

2. **Successive Approximation** (Ah/Al parameters):
   - Initial scan: Encode high bits (e.g., Ah=0, Al=4)
   - Refinement scans: Encode remaining bits (Ah=4, Al=0)

3. **Multi-Scan Architecture**:
   ```rust
   struct ProgressiveScan {
       component_indices: Vec<usize>,
       ss: u8,  // Spectral selection start
       se: u8,  // Spectral selection end
       ah: u8,  // Successive approx high
       al: u8,  // Successive approx low
   }
   ```

4. **Coefficient Buffering**:
   - Store all DCT coefficients before encoding
   - Encode in multiple passes based on scan definitions

**Implementation Steps**:
1. Create `ProgressiveScan` struct and scan planning logic
2. Modify encoder to buffer all DCT coefficients
3. Implement spectral selection encoding (frequency bands)
4. Implement successive approximation encoding (bit planes)
5. Write multiple SOS segments with correct parameters
6. Add tests with standard progressive profiles

**Testing Strategy**:
- Decode with existing progressive decoder
- Compare with libjpeg-turbo progressive output
- Validate progressive rendering (low-res to high-res)

**Reference**: ISO/IEC 10918-1 Annex G

---

### 3. Optimized Huffman Tables (Annex K) - **MEDIUM PRIORITY**

**Status**: Not started  
**Estimated Effort**: ~4 hours  
**Complexity**: Moderate  
**Value**: Medium (5-15% file size reduction)

**What's Needed**:
1. **Two-Pass Encoding**:
   - **Pass 1**: Collect symbol frequency statistics
   - **Pass 2**: Build optimal Huffman tables + encode

2. **Huffman Table Generation** (Annex K algorithm):
   ```rust
   fn generate_optimal_huffman_table(symbol_frequencies: &[usize]) 
       -> HuffmanTable
   ```

3. **Symbol Frequency Collection**:
   - Count DC differences by category
   - Count AC run-length/category pairs
   - Build separate tables for DC and AC

**Implementation Steps**:
1. Add symbol frequency counters to encoder
2. Implement Huffman table generation (Annex K procedure)
3. Add `set_optimize_huffman(bool)` flag to encoder
4. Modify encode loop for two-pass mode
5. Compare file sizes with standard tables

**Testing Strategy**:
- Verify decoder can read optimized tables
- Measure file size reduction (expect 5-15%)
- Ensure quality unchanged (only table optimization)

**Reference**: ISO/IEC 10918-1 Annex K

---

### 4. Arithmetic Coding (SOF9-SOF11) - **LOW PRIORITY**

**Status**: Not started  
**Estimated Effort**: ~16 hours  
**Complexity**: Very High  
**Value**: Low (rarely used, ~5-10% better compression than Huffman)

**Why Low Priority**:
- Patent-free since 2015 but still rare in practice
- Huffman coding sufficient for 99% of use cases
- High implementation complexity
- Limited ecosystem support

**What's Needed** (if implemented):
1. Arithmetic encoder/decoder (QM-coder)
2. SOF9 (baseline arithmetic), SOF10 (extended), SOF11 (lossless arithmetic)
3. Statistics tables for arithmetic coding
4. Extensive testing (hard to debug)

**Recommendation**: Defer indefinitely unless explicitly requested.

**Reference**: ISO/IEC 10918-1 Annex D

---

## 📊 Compliance Impact

### Current Status (70%)
```
Features:        ████████████████████████░░░░░░  70%
Critical:        ████████████████████████████████ 100% ✅
High Priority:   ████████████████░░░░░░░░░░░░░░░░  50%
Medium Priority: ████████████████████████░░░░░░░░  75%
Low Priority:    ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░   0%
```

### With Color Subsampling (+10%)
```
Features:        ████████████████████████████░░░░  80%
```

### With Progressive Encoder (+8%)
```
Features:        ████████████████████████████████  88%
```

### With Optimized Huffman (+2%)
```
Features:        ████████████████████████████████  90%
```

**Target**: 90% (practical full implementation, excluding arithmetic)

---

## 🎯 Recommended Implementation Order

### Phase 2 (Next 20 hours of work)

1. **Color Subsampling** (~6h) - Highest ROI
   - Unlocks web optimization workflows
   - Significant file size reduction for photos
   - API already complete

2. **Progressive Encoder** (~12h) - User demand
   - Web standard for large images
   - Decoder reference available
   - High user visibility

3. **Optimized Huffman** (~4h) - Easy win
   - Works across all modes
   - Automatic file size improvement
   - Low complexity

**Total Phase 2**: ~22 hours → 90% compliance

### Phase 3 (Optional, ~16h)

4. **Arithmetic Coding** (~16h) - Only if requested
   - Low practical value
   - High complexity
   - Limited ecosystem support

---

## 🛠️ Implementation Guidelines

### Before Starting Each Feature

1. **Read specification**: ISO/IEC 10918-1 relevant annex
2. **Study decoder**: Reference implementation exists
3. **Plan tests**: Define success criteria upfront
4. **Check interop**: Plan libjpeg-turbo validation

### During Implementation

1. **Incremental development**: Small commits, frequent testing
2. **Reuse patterns**: Follow existing encoder structure
3. **Test continuously**: Run test suite after each change
4. **Document decisions**: Comment non-obvious logic

### Before Marking Complete

1. **Pass all tests**: Including new feature tests
2. **Zero regressions**: All existing tests still pass
3. **Interop validated**: Cross-check with libjpeg-turbo
4. **Documentation updated**: Compliance matrix + guides

---

## 📚 Resources

### Standards
- **ISO/IEC 10918-1**: JPEG Part 1 specification
- **ITU-T T.81**: Identical to 10918-1
- **Annex A**: MCU definition and sampling
- **Annex G**: Progressive DCT
- **Annex K**: Huffman table optimization

### Reference Implementations
- **libjpeg-turbo**: `cjpeg`/`djpeg` for validation
- **jpegexp-rs decoder**: Progressive and lossless already implemented
- **IJG libjpeg**: Original reference implementation

### Testing Tools
- **libjpeg-turbo binaries**: In `libs/bin/`
- **ImageMagick**: For visual comparison
- **Hex viewers**: For bitstream analysis

---

## ⚠️ Known Challenges

### Color Subsampling
- **Edge handling**: Partial MCUs at image boundaries
- **Upsampling quality**: Decoder must reconstruct chroma
- **Testing**: Need visually diverse test images

### Progressive Encoder
- **Scan planning**: Complex logic for scan order
- **Memory usage**: Must buffer all coefficients
- **Validation**: Hard to verify correctness without decoder

### Optimized Huffman
- **Table size limits**: DHT segment has size constraints
- **Two-pass overhead**: Doubles encoding time
- **Quality impact**: Must ensure no quality loss

---

## 📈 Expected Outcomes

### Color Subsampling
- **File size**: 40-50% reduction for 4:2:0 photos
- **Quality**: Perceptually similar (chroma less important)
- **Use case**: Web optimization, photography

### Progressive Encoder
- **User experience**: Progressive image loading
- **File size**: Similar to sequential (maybe +5%)
- **Use case**: Web images, large photos

### Optimized Huffman
- **File size**: 5-15% reduction
- **Speed**: 2x slower (two passes)
- **Use case**: Archival, size-critical applications

---

## 🏁 Definition of Done (Per Feature)

- [ ] Feature implemented per specification
- [ ] Unit tests written and passing
- [ ] Integration tests with real images
- [ ] Cross-validated with libjpeg-turbo
- [ ] Documentation updated
- [ ] No regressions in existing tests
- [ ] Compliance matrix updated
- [ ] Performance acceptable (< 2x slower than standard)

---

## 📝 Conclusion

The foundation is solid with **70% JPEG 1 compliance** and all critical gaps closed. The remaining work is well-scoped with clear implementation paths:

- **Color Subsampling**: 6h, high value, moderate complexity
- **Progressive Encoder**: 12h, high demand, high complexity  
- **Optimized Huffman**: 4h, easy win, moderate complexity

Total **~22 hours to reach 90% compliance**, which represents practical full implementation for real-world use cases.

**Recommendation**: Implement in order listed. Each feature is independently valuable and builds on the existing solid foundation.

---

**Document Version**: 1.0  
**Last Updated**: January 10, 2026  
**Status**: Roadmap for Phase 2 implementation
