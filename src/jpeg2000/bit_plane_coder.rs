use super::mq_coder::MqCoder;

pub struct BitPlaneCoder<'a> {
    pub width: u32,
    pub height: u32,
    pub data: &'a [i32], // Quantized coefficients
    pub state: Vec<u8>,
    pub mq: MqCoder,
    pub coefficients: Vec<i32>,
    pub num_passes_decoded: u32,
}

impl<'a> BitPlaneCoder<'a> {
    /// Create a new BitPlaneCoder for encoding (uses our internal context states)
    pub fn new(width: u32, height: u32, data: &'a [i32]) -> Self {
        Self::with_context_mode(width, height, data, false)
    }
    
    /// Create a BitPlaneCoder with configurable context initialization
    /// If openjpeg_compat is true, uses OpenJPEG-compatible context states
    pub fn with_context_mode(width: u32, height: u32, data: &'a [i32], openjpeg_compat: bool) -> Self {
        let size = (width * height) as usize;
        let mut mq = MqCoder::new();
        mq.init_contexts(19);
        
        // UNIFORM context (18): always state 46 for 50% probability
        mq.set_context(Self::CTX_UNIFORM, 46, 0);
        
        if openjpeg_compat {
            // OpenJPEG-compatible: RUN context at state 0 (default)
            // State 0 has Qe=0x5601 (~33.6% LPS probability)
            // Don't set - let it stay at default state 0
        } else {
            // Our internal mode: RUN context at state 3
            // State 3 has Qe=0x0AC1 (~4.2% LPS probability)
            // This is what our encoder expects
            mq.set_context(Self::CTX_RUN, 3, 0);
        }

        // Load coefficients if provided usually
        // But for standard new, init to zero if not reusing
        let coefficients = if !data.is_empty() && data.len() == size {
            data.to_vec()
        } else {
            vec![0; size]
        };

        Self {
            width,
            height,
            data,
            state: vec![0; size],
            mq,
            coefficients,
            num_passes_decoded: 0,
        }
    }

    // State Bit Definitions
    const SIG: u8 = 1 << 0;
    const VISITED: u8 = 1 << 1;
    const REFINE: u8 = 1 << 2;
    const SIGN: u8 = 1 << 3; // 0=pos, 1=neg

    // Zero Coding Tables (LL, HL, LH, HH)
    // Table C-1: Contexts for SigProp (ZC)

    pub fn get_neighbors(&self, x: u32, y: u32) -> (u8, u8, u8) {
        // Returns count of significant neighbors (H, V, D)
        let w = self.width as i32;
        let h = self.height as i32;
        let ix = x as i32;
        let iy = y as i32;

        let mut h_cnt = 0;
        let mut v_cnt = 0;
        let mut d_cnt = 0;

        let idx = |cnx, cny| (cny * w + cnx) as usize;

        // H: (x-1, y), (x+1, y)
        if ix > 0 && (self.state[idx(ix - 1, iy)] & Self::SIG) != 0 {
            h_cnt += 1;
        }
        if ix < w - 1 && (self.state[idx(ix + 1, iy)] & Self::SIG) != 0 {
            h_cnt += 1;
        }

        // V: (x, y-1), (x, y+1)
        if iy > 0 && (self.state[idx(ix, iy - 1)] & Self::SIG) != 0 {
            v_cnt += 1;
        }
        if iy < h - 1 && (self.state[idx(ix, iy + 1)] & Self::SIG) != 0 {
            v_cnt += 1;
        }

        // D: Diagonals
        if ix > 0 && iy > 0 && (self.state[idx(ix - 1, iy - 1)] & Self::SIG) != 0 {
            d_cnt += 1;
        }
        if ix < w - 1 && iy > 0 && (self.state[idx(ix + 1, iy - 1)] & Self::SIG) != 0 {
            d_cnt += 1;
        }
        if ix > 0 && iy < h - 1 && (self.state[idx(ix - 1, iy + 1)] & Self::SIG) != 0 {
            d_cnt += 1;
        }
        if ix < w - 1 && iy < h - 1 && (self.state[idx(ix + 1, iy + 1)] & Self::SIG) != 0 {
            d_cnt += 1;
        }

        (h_cnt, v_cnt, d_cnt)
    }

    fn get_zc_context(&self, band: u8, h: u8, v: u8, d: u8) -> usize {
        // Table C-2: Contexts for the significance propagation and cleanup passes
        match band {
            0 | 2 => {
                // LL (0) and LH (2) - Vertical High Pass
                match (h, v, d) {
                    (2, _, _) => 8,
                    (1, v, _) if v >= 1 => 7,
                    (1, 0, d) if d >= 1 => 6,
                    (1, 0, 0) => 5,
                    (0, 2, _) => 4,
                    (0, 1, _) => 3,
                    (0, 0, d) if d >= 2 => 2,
                    (0, 0, 1) => 1,
                    _ => 0,
                }
            }
            1 => {
                // HL (1) - Horizontal High Pass
                match (v, h, d) {
                    (2, _, _) => 8,
                    (1, h, _) if h >= 1 => 7,
                    (1, 0, d) if d >= 1 => 6,
                    (1, 0, 0) => 5,
                    (0, 2, _) => 4,
                    (0, 1, _) => 3,
                    (0, 0, d) if d >= 2 => 2,
                    (0, 0, 1) => 1,
                    _ => 0,
                }
            }
            3 => {
                // HH (Diagonal High-Pass -> Diagonal dominant)
                match (d, h + v) {
                    (d, _) if d >= 3 => 8,
                    (2, hv) if hv >= 1 => 7,
                    (2, 0) => 6,
                    (1, hv) if hv >= 2 => 5,
                    (1, 1) => 4,
                    (1, 0) => 3,
                    (0, hv) if hv >= 2 => 2,
                    (0, 1) => 1,
                    _ => 0,
                }
            }
            _ => 0,
        }
    }

    /// Calculate the maximum bit-plane needed based on the coefficient magnitudes
    fn calculate_max_bit_plane(&self) -> u8 {
        let max_val = self.data.iter().map(|v| v.abs()).max().unwrap_or(0);
        if max_val == 0 {
            return 0;
        }
        // Find the highest bit position (0-indexed)
        // e.g., 128 = 0b10000000, highest bit is position 7
        (32 - max_val.leading_zeros()) as u8 - 1
    }

    pub fn encode_codeblock(&mut self) {
        // Calculate max bit-plane from actual coefficient magnitudes
        let max_bit_plane = self.calculate_max_bit_plane();
        
        if max_bit_plane == 0 && self.data.iter().all(|&v| v == 0) {
            // All zeros - nothing to encode
            return;
        }
        
        // JPEG 2000 encoding pass order:
        // 1. First pass: Cleanup at max_bit_plane
        // 2. Then for each bit-plane from (max_bit_plane - 1) down to 0:
        //    - Significance Propagation Pass
        //    - Magnitude Refinement Pass  
        //    - Cleanup Pass
        
        // First pass: Cleanup at max bit-plane
        self.encode_cleanup(max_bit_plane);
        
        // Subsequent passes for remaining bit-planes
        for bp in (0..max_bit_plane).rev() {
            // Reset VISITED flags before SigProp
            for v in &mut self.state {
                *v &= !Self::VISITED;
            }
            
            self.encode_significance_propagation(bp);
            self.encode_magnitude_refinement(bp);
            self.encode_cleanup(bp);
        }
    }

    pub fn decode_codeblock(
        &mut self,
        data: &[u8],
        max_bit_plane: u8,
        num_new_passes: u8,
        orientation: u8,
    ) -> Result<Vec<i32>, crate::jpeg2000::bit_io::BitIoError> {
        if num_new_passes == 0 {
            return Ok(self.coefficients.clone());
        }

        self.mq.init_decoder(data);

        #[derive(Debug)]
        enum PassType {
            SigProp,
            MagRef,
            Cleanup,
        }

        for _i in 0..num_new_passes {
            let pass_idx = self.num_passes_decoded;

            let (bp, pass_type) = if pass_idx == 0 {
                (max_bit_plane, PassType::Cleanup)
            } else {
                let plane_offset = (pass_idx - 1) / 3;
                if plane_offset as u8 >= max_bit_plane {
                    break;
                }
                let bp = max_bit_plane - 1 - plane_offset as u8;
                let rem = (pass_idx - 1) % 3;
                match rem {
                    0 => (bp, PassType::SigProp),
                    1 => (bp, PassType::MagRef),
                    2 => (bp, PassType::Cleanup),
                    _ => unreachable!(),
                }
            };

            // Reset VISITED at start of SigProp
            if let PassType::SigProp = pass_type {
                for v in &mut self.state {
                    *v &= !Self::VISITED;
                }
            }

            if std::env::var("BPC_TRACE").is_ok() {
                eprintln!("  Pass {}: {:?} at bit-plane {}", pass_idx, pass_type, bp);
            }
            match pass_type {
                PassType::SigProp => self.decode_significance_propagation(bp, orientation)?,
                PassType::MagRef => self.decode_magnitude_refinement(bp)?,
                PassType::Cleanup => self.decode_cleanup(bp, orientation)?,
            }
            self.num_passes_decoded += 1;
        }

        Ok(self.coefficients.clone())
    }

    fn decode_significance_propagation(
        &mut self,
        bit_plane: u8,
        orientation: u8,
    ) -> Result<(), crate::jpeg2000::bit_io::BitIoError> {
        // Scan in stripe order (4 rows at a time)
        let stripe_height = 4;
        let width = self.width;
        let height = self.height;

        for y_stripe in (0..height).step_by(stripe_height as usize) {
            for x in 0..width {
                for y_offset in 0..stripe_height.min(height - y_stripe) {
                    let y = y_stripe + y_offset;
                    let idx = (y * width + x) as usize;

                    if idx >= self.state.len() {
                        continue;
                    }

                    let state = self.state[idx];

                    // If insignificant and not visited, and has significant neighbors
                    if (state & (Self::SIG | Self::VISITED)) == 0 {
                        let (hc, vc, dc) = self.get_neighbors(x, y);
                        if hc > 0 || vc > 0 || dc > 0 {
                            // Decode significance bit
                            let cx = self.get_zc_context(orientation, hc, vc, dc);
                            let bit = self.mq.decode_bit(cx);
                            
                            if std::env::var("BPC_TRACE").is_ok() {
                                eprintln!("DEC: SigProp idx={} bp={}: ctx={}, bit={}", idx, bit_plane, cx, bit);
                            }

                            if bit != 0 {
                                // Became significant
                                self.state[idx] |= Self::SIG | Self::VISITED;

                                // Decode sign
                                let sc_data = self.get_sign_context(x, y, width, height);
                                let sc_ctx = sc_data & 0xFF;
                                let xor = (sc_data >> 8) & 1;
                                let sym = self.mq.decode_bit(sc_ctx);
                                let sign_bit = sym ^ (xor as u8);

                                if sign_bit != 0 {
                                    self.state[idx] |= Self::SIGN;
                                    self.coefficients[idx] = -(1 << bit_plane);
                                } else {
                                    self.coefficients[idx] = 1 << bit_plane;
                                }
                            } else {
                                // Visited but not significant
                                self.state[idx] |= Self::VISITED;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn decode_magnitude_refinement(
        &mut self,
        bit_plane: u8,
    ) -> Result<(), crate::jpeg2000::bit_io::BitIoError> {
        // Use stripe order (4 rows at a time, column-by-column) to match encoder
        let stripe_height = 4u32;
        let width = self.width;
        let height = self.height;

        for y_stripe in (0..height).step_by(stripe_height as usize) {
            for x in 0..width {
                for y_offset in 0..stripe_height.min(height - y_stripe) {
                    let y = y_stripe + y_offset;
                    let idx = (y * width + x) as usize;

                    if idx >= self.state.len() {
                        continue;
                    }

                    let state = self.state[idx];

                    // If significant and not visited in SigProp
                    if (state & Self::SIG) != 0 && (state & Self::VISITED) == 0 {
                        self.state[idx] |= Self::VISITED;

                        // Get MR context
                        let mr_ctx = self.get_magnitude_refinement_context(idx, width, height);

                        // Decode refinement bit
                        let bit = self.mq.decode_bit(mr_ctx);
                        
                        if std::env::var("BPC_TRACE").is_ok() {
                            eprintln!("DEC: MagRef idx={} bp={}: ctx={}, bit={}, coeff_before={}", 
                                idx, bit_plane, mr_ctx, bit, self.coefficients[idx]);
                        }

                        if bit != 0 {
                            // Add bit to coefficient
                            if (state & Self::SIGN) != 0 {
                                self.coefficients[idx] -= 1 << bit_plane;
                            } else {
                                self.coefficients[idx] += 1 << bit_plane;
                            }
                            if std::env::var("BPC_TRACE").is_ok() {
                                eprintln!("DEC: MagRef idx={}: coeff_after={}", idx, self.coefficients[idx]);
                            }
                        }

                        self.state[idx] |= Self::REFINE;
                    }
                }
            }
        }
        Ok(())
    }

    // Context 17 is the RUN context (for RLC mode decision)
    // Context 18 is the UNIFORM context (for position decoding in RLC)
    const CTX_RUN: usize = 17;
    const CTX_UNIFORM: usize = 18;

    fn decode_cleanup(
        &mut self,
        bit_plane: u8,
        orientation: u8,
    ) -> Result<(), crate::jpeg2000::bit_io::BitIoError> {
        // Scan in stripe order (4 rows at a time, column-by-column)
        let stripe_height = 4u32;
        let width = self.width;
        let height = self.height;

        for y_stripe in (0..height).step_by(stripe_height as usize) {
            for x in 0..width {
                let actual_stripe_height = stripe_height.min(height - y_stripe);
                
                // Check if we can use RLC for this stripe column
                // RLC is used when:
                // 1. We're at the start of a stripe column (y_offset = 0)
                // 2. All 4 samples in the column are insignificant and unvisited
                // 3. None of them have significant neighbors (context 0 for all)
                
                // Enable RLC for proper JPEG 2000 cleanup pass decoding
                let mut can_use_rlc = actual_stripe_height == 4; // Only for full stripes
                
                if can_use_rlc {
                    for y_offset in 0..4 {
                        let y = y_stripe + y_offset;
                        let idx = (y * width + x) as usize;
                        
                        if idx >= self.state.len() {
                            can_use_rlc = false;
                            break;
                        }
                        
                        let state = self.state[idx];
                        if (state & (Self::SIG | Self::VISITED)) != 0 {
                            can_use_rlc = false;
                            break;
                        }
                        
                        // Check if any neighbor is significant (context would be non-zero)
                        let (hc, vc, dc) = self.get_neighbors(x, y);
                        if hc > 0 || vc > 0 || dc > 0 {
                            can_use_rlc = false;
                            break;
                        }
                    }
                }
                
                    if can_use_rlc {
                    // Use RLC: decode a single bit with RUN context
                    let run_bit = self.mq.decode_bit(Self::CTX_RUN);
                    
                    if run_bit == 0 {
                        // All 4 samples in this column remain insignificant
                        // Nothing to do - they stay at 0
                        if std::env::var("BPC_TRACE").is_ok() {
                            eprintln!("DEC: RLC col={} bp={}: RUN=0, all insignificant", x, bit_plane);
                        }
                        continue; // Move to next column
                    } else {
                        // At least one sample becomes significant
                        // Decode 2 bits with UNIFORM context to get position (0-3)
                        // Note: Position is encoded MSB first
                        let pos_bit1 = self.mq.decode_bit(Self::CTX_UNIFORM);
                        let pos_bit0 = self.mq.decode_bit(Self::CTX_UNIFORM);
                        let pos = ((pos_bit1 as u32) << 1) | (pos_bit0 as u32);
                        if std::env::var("BPC_TRACE").is_ok() {
                            eprintln!("DEC: RLC col={} bp={}: RUN=1, pos bits={},{}, pos={}", x, bit_plane, pos_bit1, pos_bit0, pos);
                        }
                        
                        if std::env::var("BPC_DEBUG").is_ok() && bit_plane >= 6 {
                            eprintln!("    RLC pos_bits={},{} pos={}", pos_bit1, pos_bit0, pos);
                        }
                        
                        // Process samples from position 0 to pos-1 with regular cleanup
                        // (these are the samples before the first significant one)
                        for y_offset in 0..pos {
                            let y = y_stripe + y_offset;
                            let idx = (y * width + x) as usize;
                            
                            if idx >= self.state.len() {
                                continue;
                            }
                            
                            // These samples are definitely insignificant (we know from RLC)
                            // but we might need to decode their state anyway... 
                            // Actually no - in RLC mode, samples before pos are guaranteed insignificant
                        }
                        
                        // Mark the position that became significant and decode its sign
                        let y = y_stripe + pos;
                        let idx = (y * width + x) as usize;
                        
                        self.state[idx] |= Self::SIG;
                        
                        // Decode sign
                        let sc_data = self.get_sign_context(x, y, width, height);
                        let sc_ctx = sc_data & 0xFF;
                        let xor = (sc_data >> 8) & 1;
                        let sym = self.mq.decode_bit(sc_ctx);
                        let sign_bit = sym ^ (xor as u8);
                        
                        if std::env::var("BPC_TRACE").is_ok() {
                            eprintln!("DEC: Sign idx={} bp={}: ctx={}, xor={}, sym={}, sign_bit={}", idx, bit_plane, sc_ctx, xor, sym, sign_bit);
                        }
                        
                        if sign_bit != 0 {
                            self.state[idx] |= Self::SIGN;
                            self.coefficients[idx] = -(1 << bit_plane);
                            if std::env::var("BPC_TRACE").is_ok() {
                                eprintln!("DEC: set coeff[{}] = {} (negative)", idx, self.coefficients[idx]);
                            }
                        } else {
                            self.coefficients[idx] = 1 << bit_plane;
                            if std::env::var("BPC_TRACE").is_ok() {
                                eprintln!("DEC: set coeff[{}] = {} (positive)", idx, self.coefficients[idx]);
                            }
                        }
                        
                        // Continue with regular cleanup for remaining positions in this column
                        for y_offset in (pos + 1)..4 {
                            let y = y_stripe + y_offset;
                            let idx = (y * width + x) as usize;
                            
                            if idx >= self.state.len() {
                                continue;
                            }
                            
                            let state = self.state[idx];
                            if (state & Self::VISITED) == 0 {
                                let (hc, vc, dc) = self.get_neighbors(x, y);
                                let cx = self.get_zc_context(orientation, hc, vc, dc);
                                let bit = self.mq.decode_bit(cx);
                                
                                if std::env::var("BPC_TRACE").is_ok() {
                                    eprintln!("DEC: Cleanup-cont idx={} bp={}: ctx={}, bit={}", idx, bit_plane, cx, bit);
                                }
                                
                                if bit != 0 {
                                    self.state[idx] |= Self::SIG;
                                    
                                    let sc_data = self.get_sign_context(x, y, width, height);
                                    let sc_ctx = sc_data & 0xFF;
                                    let xor = (sc_data >> 8) & 1;
                                    let sym = self.mq.decode_bit(sc_ctx);
                                    let sign_bit = sym ^ (xor as u8);
                                    
                                    if sign_bit != 0 {
                                        self.state[idx] |= Self::SIGN;
                                        self.coefficients[idx] = -(1 << bit_plane);
                                    } else {
                                        self.coefficients[idx] = 1 << bit_plane;
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // Regular cleanup (no RLC)
                    for y_offset in 0..actual_stripe_height {
                        let y = y_stripe + y_offset;
                        let idx = (y * width + x) as usize;

                        if idx >= self.state.len() {
                            continue;
                        }

                        let state = self.state[idx];

                        // If not visited, must be insignificant - decode its significance
                        if (state & Self::VISITED) == 0 {
                            let (hc, vc, dc) = self.get_neighbors(x, y);
                            let cx = self.get_zc_context(orientation, hc, vc, dc);
                            let bit = self.mq.decode_bit(cx);

                            if std::env::var("BPC_TRACE").is_ok() {
                                eprintln!("DEC: Cleanup idx={} bp={}: ctx={}, bit={}", idx, bit_plane, cx, bit);
                            }

                            if bit != 0 {
                                // Became significant
                                self.state[idx] |= Self::SIG;

                                // Decode sign
                                let sc_data = self.get_sign_context(x, y, width, height);
                                let sc_ctx = sc_data & 0xFF;
                                let xor = (sc_data >> 8) & 1;
                                let sym = self.mq.decode_bit(sc_ctx);
                                let sign_bit = sym ^ (xor as u8);

                                if std::env::var("BPC_DEBUG").is_ok() && bit_plane >= 6 {
                                    eprintln!("    Sign: ctx={}, sym={}, xor={}, sign_bit={}", sc_ctx, sym, xor, sign_bit);
                                }

                                if sign_bit != 0 {
                                    self.state[idx] |= Self::SIGN;
                                    self.coefficients[idx] = -(1 << bit_plane);
                                } else {
                                    self.coefficients[idx] = 1 << bit_plane;
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn get_sign_context(&self, x: u32, y: u32, width: u32, height: u32) -> usize {
        // Table C-3: Contexts for the sign bit (SC)
        // Calculate contributions (1 for pos, -1 for neg)
        let w = width as i32;
        let h = height as i32;
        let ix = x as i32;
        let iy = y as i32;
        let idx = |cx, cy| (cy * w + cx) as usize;

        let get_sign_val = |pos: usize| -> i8 {
            let s = self.state[pos];
            if (s & Self::SIG) != 0 {
                if (s & Self::SIGN) != 0 {
                    -1
                } else {
                    1
                }
            } else {
                0
            }
        };

        let mut h_contrib = 0;
        if ix > 0 {
            h_contrib += get_sign_val(idx(ix - 1, iy));
        }
        if ix < w - 1 {
            h_contrib += get_sign_val(idx(ix + 1, iy));
        }

        let mut v_contrib = 0;
        if iy > 0 {
            v_contrib += get_sign_val(idx(ix, iy - 1));
        }
        if iy < h - 1 {
            v_contrib += get_sign_val(idx(ix, iy + 1));
        }

        // Context label 9..13
        // XOR bit (0 or 1) implies if we should invert the sign bit before coding.
        // Returns (context, xor_bit) - but mq only needs context?
        // Wait, sign coding uses xor bit!

        let (ctx_offset, xor) = match (h_contrib, v_contrib) {
            (2, 2) => (13, 1),
            (2, 1) => (12, 1),
            (2, 0) => (11, 1),
            (2, -1) => (10, 1),
            (2, -2) => (9, 1),
            (1, 2) => (12, 1),
            (1, 1) => (13, 0),
            (1, 0) => (12, 0),
            (1, -1) => (11, 0),
            (1, -2) => (10, 0),
            (0, 2) => (11, 1),
            (0, 1) => (12, 1),
            (0, 0) => (9, 0),
            (0, -1) => (12, 0),
            (0, -2) => (11, 0),
            (-1, 2) => (10, 1),
            (-1, 1) => (11, 1),
            (-1, 0) => (12, 1),
            (-1, -1) => (13, 0),
            (-1, -2) => (12, 0),
            (-2, 2) => (9, 0),
            (-2, 1) => (10, 0),
            (-2, 0) => (11, 0),
            (-2, -1) => (12, 0),
            (-2, -2) => (13, 0),
            _ => (9, 0),
        };

        // We need to return the combined context?
        // Or handle XOR outside?
        // My MqCoder doesn't handle XOR.
        // So I should return (ctx, xor).
        // But the function returns usize.
        // I'll return ctx | (xor << 8).
        ctx_offset | (xor << 8)
    }

    fn get_magnitude_refinement_context(&self, idx: usize, width: u32, _height: u32) -> usize {
        // Table C-6
        let state = self.state[idx];
        let refined = if (state & Self::REFINE) != 0 { 1 } else { 0 };

        let x = (idx % width as usize) as u32;
        let y = (idx / width as usize) as u32;
        let (hc, vc, dc) = self.get_neighbors(x, y);
        let sigma_prime = if hc + vc + dc > 0 { 1 } else { 0 };

        if refined == 0 {
            if sigma_prime == 1 {
                15
            } else {
                14
            }
        } else {
            16
        }
    }

    /// Encode significance propagation pass using stripe order (matches decoder)
    fn encode_significance_propagation(&mut self, bit_plane: u8) {
        let stripe_height = 4u32;
        let width = self.width;
        let height = self.height;

        for y_stripe in (0..height).step_by(stripe_height as usize) {
            for x in 0..width {
                for y_offset in 0..stripe_height.min(height - y_stripe) {
                    let y = y_stripe + y_offset;
                    let idx = (y * width + x) as usize;

                    if idx >= self.state.len() {
                        continue;
                    }

                    let state = self.state[idx];

                    // If insignificant and not visited, and has significant neighbors
                    if (state & (Self::SIG | Self::VISITED)) == 0 {
                        let (hc, vc, dc) = self.get_neighbors(x, y);
                        if hc > 0 || vc > 0 || dc > 0 {
                            let val = self.data[idx];
                            let bit = ((val.abs() >> bit_plane) & 1) as u8;

                            // Encode ZC
                            let cx = self.get_zc_context(0, hc, vc, dc);
                            if std::env::var("BPC_TRACE").is_ok() {
                                eprintln!("ENC: SigProp idx={} bp={}: val={}, bit={}, ctx={}", idx, bit_plane, val, bit, cx);
                            }
                            self.mq.encode(bit, cx);

                            if bit == 1 {
                                // Became Significant
                                let sign = if val < 0 { 1u8 } else { 0u8 };
                                self.state[idx] |= Self::SIG | Self::VISITED;
                                if sign == 1 {
                                    self.state[idx] |= Self::SIGN;
                                }

                                // Encode Sign
                                let sc_data = self.get_sign_context(x, y, width, height);
                                let sc_ctx = sc_data & 0xFF;
                                let xor = ((sc_data >> 8) & 1) as u8;
                                let sym = sign ^ xor;
                                self.mq.encode(sym, sc_ctx);
                            } else {
                                self.state[idx] |= Self::VISITED;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Encode magnitude refinement pass using stripe order (matches decoder)
    fn encode_magnitude_refinement(&mut self, bit_plane: u8) {
        let stripe_height = 4u32;
        let width = self.width;
        let height = self.height;

        for y_stripe in (0..height).step_by(stripe_height as usize) {
            for x in 0..width {
                for y_offset in 0..stripe_height.min(height - y_stripe) {
                    let y = y_stripe + y_offset;
                    let idx = (y * width + x) as usize;

                    if idx >= self.state.len() {
                        continue;
                    }

                    let state = self.state[idx];

                    // If significant and not visited in SigProp
                    if (state & Self::SIG) != 0 && (state & Self::VISITED) == 0 {
                        self.state[idx] |= Self::VISITED;

                        let val = self.data[idx];
                        let bit = ((val.abs() >> bit_plane) & 1) as u8;

                        // MR Context
                        let mr_ctx = self.get_magnitude_refinement_context(idx, width, height);
                        if std::env::var("BPC_TRACE").is_ok() {
                            eprintln!("ENC: MagRef idx={} bp={}: val={}, bit={}, ctx={}", 
                                idx, bit_plane, val, bit, mr_ctx);
                        }
                        self.mq.encode(bit, mr_ctx);
                        self.state[idx] |= Self::REFINE;
                    }
                }
            }
        }
    }

    /// Encode cleanup pass using stripe order with RLC (matches decoder)
    fn encode_cleanup(&mut self, bit_plane: u8) {
        let stripe_height = 4u32;
        let width = self.width;
        let height = self.height;

        for y_stripe in (0..height).step_by(stripe_height as usize) {
            for x in 0..width {
                let actual_stripe_height = stripe_height.min(height - y_stripe);

                // Check if we can use RLC for this stripe column
                let mut can_use_rlc = actual_stripe_height == 4;

                if can_use_rlc {
                    for y_offset in 0..4 {
                        let y = y_stripe + y_offset;
                        let idx = (y * width + x) as usize;

                        if idx >= self.state.len() {
                            can_use_rlc = false;
                            break;
                        }

                        let state = self.state[idx];
                        if (state & (Self::SIG | Self::VISITED)) != 0 {
                            can_use_rlc = false;
                            break;
                        }

                        let (hc, vc, dc) = self.get_neighbors(x, y);
                        if hc > 0 || vc > 0 || dc > 0 {
                            can_use_rlc = false;
                            break;
                        }
                    }
                }

                if can_use_rlc {
                    // Check if any sample in this column becomes significant
                    let mut first_sig_pos: Option<u32> = None;
                    for y_offset in 0..4u32 {
                        let y = y_stripe + y_offset;
                        let idx = (y * width + x) as usize;
                        let val = self.data[idx];
                        let bit = (val.abs() >> bit_plane) & 1;
                        if bit == 1 {
                            first_sig_pos = Some(y_offset);
                            break;
                        }
                    }

                    if let Some(pos) = first_sig_pos {
                        // At least one sample becomes significant
                        if std::env::var("BPC_TRACE").is_ok() {
                            eprintln!("ENC: RLC col={} bp={}: encode RUN=1, pos={}", x, bit_plane, pos);
                        }
                        self.mq.encode(1, Self::CTX_RUN); // run_bit = 1

                        // Encode position (2 bits, MSB first)
                        let pos_bit1 = ((pos >> 1) & 1) as u8;
                        let pos_bit0 = (pos & 1) as u8;
                        if std::env::var("BPC_TRACE").is_ok() {
                            eprintln!("ENC: pos bits = {},{}", pos_bit1, pos_bit0);
                        }
                        self.mq.encode(pos_bit1, Self::CTX_UNIFORM);
                        self.mq.encode(pos_bit0, Self::CTX_UNIFORM);

                        // Mark the first significant sample
                        let y = y_stripe + pos;
                        let idx = (y * width + x) as usize;
                        let val = self.data[idx];
                        let sign = if val < 0 { 1u8 } else { 0u8 };
                        self.state[idx] |= Self::SIG;
                        if sign == 1 {
                            self.state[idx] |= Self::SIGN;
                        }

                        // Encode sign
                        let sc_data = self.get_sign_context(x, y, width, height);
                        let sc_ctx = sc_data & 0xFF;
                        let xor = ((sc_data >> 8) & 1) as u8;
                        let sym = sign ^ xor;
                        if std::env::var("BPC_TRACE").is_ok() {
                            eprintln!("ENC: Sign idx={} bp={}: sign={}, ctx={}, xor={}, sym={}", idx, bit_plane, sign, sc_ctx, xor, sym);
                        }
                        self.mq.encode(sym, sc_ctx);

                        // Continue with regular cleanup for remaining positions
                        for y_offset in (pos + 1)..4 {
                            let y = y_stripe + y_offset;
                            let idx = (y * width + x) as usize;

                            if idx >= self.state.len() {
                                continue;
                            }

                            let state = self.state[idx];
                            if (state & Self::VISITED) == 0 {
                                let (hc, vc, dc) = self.get_neighbors(x, y);
                                let cx = self.get_zc_context(0, hc, vc, dc);
                                let val = self.data[idx];
                                let bit = ((val.abs() >> bit_plane) & 1) as u8;
                                if std::env::var("BPC_TRACE").is_ok() {
                                    eprintln!("ENC: Cleanup-cont idx={} bp={}: ctx={}, bit={}", idx, bit_plane, cx, bit);
                                }
                                self.mq.encode(bit, cx);

                                if bit == 1 {
                                    let sign = if val < 0 { 1u8 } else { 0u8 };
                                    self.state[idx] |= Self::SIG;
                                    if sign == 1 {
                                        self.state[idx] |= Self::SIGN;
                                    }

                                    let sc_data = self.get_sign_context(x, y, width, height);
                                    let sc_ctx = sc_data & 0xFF;
                                    let xor = ((sc_data >> 8) & 1) as u8;
                                    let sym = sign ^ xor;
                                    self.mq.encode(sym, sc_ctx);
                                }
                            }
                        }
                    } else {
                        // All 4 samples remain insignificant
                        if std::env::var("BPC_TRACE").is_ok() {
                            eprintln!("ENC: RLC col={} bp={}: encode RUN=0", x, bit_plane);
                        }
                        self.mq.encode(0, Self::CTX_RUN); // run_bit = 0
                    }
                } else {
                    // Regular cleanup (no RLC)
                    for y_offset in 0..actual_stripe_height {
                        let y = y_stripe + y_offset;
                        let idx = (y * width + x) as usize;

                        if idx >= self.state.len() {
                            continue;
                        }

                        let state = self.state[idx];
                        if (state & Self::VISITED) == 0 {
                            let (hc, vc, dc) = self.get_neighbors(x, y);
                            let cx = self.get_zc_context(0, hc, vc, dc);
                            let val = self.data[idx];
                            let bit = ((val.abs() >> bit_plane) & 1) as u8;
                            if std::env::var("BPC_TRACE").is_ok() {
                                eprintln!("ENC: Cleanup idx={} bp={}: ctx={}, bit={}", idx, bit_plane, cx, bit);
                            }
                            self.mq.encode(bit, cx);

                            if bit == 1 {
                                let sign = if val < 0 { 1u8 } else { 0u8 };
                                self.state[idx] |= Self::SIG;
                                if sign == 1 {
                                    self.state[idx] |= Self::SIGN;
                                }

                                let sc_data = self.get_sign_context(x, y, width, height);
                                let sc_ctx = sc_data & 0xFF;
                                let xor = ((sc_data >> 8) & 1) as u8;
                                let sym = sign ^ xor;
                                self.mq.encode(sym, sc_ctx);
                            }
                        }
                    }
                }

                // Reset VISITED for next bit-plane
                for y_offset in 0..actual_stripe_height {
                    let y = y_stripe + y_offset;
                    let idx = (y * width + x) as usize;
                    if idx < self.state.len() {
                        self.state[idx] &= !Self::VISITED;
                    }
                }
            }
        }
    }

    // Legacy functions kept for backwards compatibility but deprecated
    #[deprecated(note = "Use encode_significance_propagation instead")]
    pub fn significance_propagation(&mut self, bit_plane: u8) {
        self.encode_significance_propagation(bit_plane);
    }

    #[deprecated(note = "Use encode_magnitude_refinement instead")]
    pub fn magnitude_refinement(&mut self, bit_plane: u8) {
        self.encode_magnitude_refinement(bit_plane);
    }

    #[deprecated(note = "Use encode_cleanup instead")]
    pub fn cleanup(&mut self, bit_plane: u8) {
        self.encode_cleanup(bit_plane);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_plane_coding_simple() {
        // 4x4 block
        let data = [10, 0, 0, 0, 0, 5, 0, 0, 0, 0, -3, 0, 0, 0, 0, 1];
        let mut bpc = BitPlaneCoder::new(4, 4, &data);

        // This should not panic
        bpc.encode_codeblock();

        // We can't easily verify exact bytes without full J2K compliance check,
        // but we can check that state updated (e.g., significant samples marked)

        // At end, indices 0, 5, 10, 15 should be SIGNIFICANT
        let sig = BitPlaneCoder::SIG;
        assert_eq!(bpc.state[0] & sig, sig, "Index 0 should be significant");
        assert_eq!(bpc.state[5] & sig, sig, "Index 5 should be significant");
        // -3 is abs 3. Max bit plane was 5. 3 is binary 011. It should eventually become significant.
        // wait, we ran loop 5..0. 0..5 rev.
        // 3 is 000011.
        // Bit 1: 3>>1 & 1 = 1. Yes.
        assert_eq!(
            bpc.state[10] & sig,
            sig,
            "Index 10 (-3) should be significant"
        );
    }
    
    #[test]
    fn test_bit_plane_roundtrip() {
        // Simple 4x4 block with known values
        let original = [-128i32, -64, 32, 16, -32, 64, -16, 8, 0, -8, 4, -4, 2, -2, 1, -1];
        
        // Encode
        let mut bpc_enc = BitPlaneCoder::new(4, 4, &original);
        bpc_enc.encode_codeblock();
        bpc_enc.mq.flush();
        let encoded = bpc_enc.mq.get_buffer().to_vec();
        
        println!("Encoded {} bytes: {:02X?}", encoded.len(), &encoded);
        
        // Decode
        // max_bit_plane: highest bit is 7 (for -128)
        // num_passes: 22 (cleanup at bp7, then 3 passes for bp6-0 = 21, total 22)
        let mut bpc_dec = BitPlaneCoder::new(4, 4, &[]);
        let decoded = bpc_dec.decode_codeblock(&encoded, 7, 22, 0).expect("decode failed");
        
        println!("Original:  {:?}", original);
        println!("Decoded:   {:?}", decoded);
        
        // Compare
        for i in 0..16 {
            assert_eq!(original[i], decoded[i], "Mismatch at index {}: original={}, decoded={}", 
                i, original[i], decoded[i]);
        }
    }
}
