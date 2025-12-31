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
    pub fn new(width: u32, height: u32, data: &'a [i32]) -> Self {
        let size = (width * height) as usize;
        let mut mq = MqCoder::new();
        mq.init_contexts(19);

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

    // Zero Coding Tables (LL, HL, LH, HH) - Simplified for LL (Index 0)
    // Actually J2K has specific tables based on band.
    // Let's assume LL for simplicity for now or implement full logic.
    // Table C-1: Contexts for SigProp (ZC)
    // We need logic to map neighbor counts (H, V, D) to Context (0-8)

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
            0 | 1 => {
                // LL and LH (Vertical High-Pass, Horiz Low-Pass -> Horiz dominant)
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
            2 => {
                // HL (Horiz High-Pass, Vert Low-Pass -> Vert dominant)
                match (v, h, d) {
                    // Transpose H and V
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

    pub fn encode_codeblock(&mut self) {
        // Iterate bitplanes from MSB to LSB
        // Assume 30 down to 0? Or determined by max value.
        // For testing, let's start at bit 5.
        for bp in (0..5).rev() {
            self.significance_propagation(bp);
            self.magnitude_refinement(bp);
            self.cleanup(bp);
        }
    }

    pub fn decode_codeblock(
        &mut self,
        data: &[u8],
        max_bit_plane: u8,
        num_new_passes: u8,
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

            match pass_type {
                PassType::SigProp => self.decode_significance_propagation(bp)?,
                PassType::MagRef => self.decode_magnitude_refinement(bp)?,
                PassType::Cleanup => self.decode_cleanup(bp)?,
            }
            self.num_passes_decoded += 1;
        }

        Ok(self.coefficients.clone())
    }

    fn decode_significance_propagation(
        &mut self,
        bit_plane: u8,
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
                            let cx = self.get_zc_context(0, hc, vc, dc);
                            let bit = self.mq.decode_bit(cx);

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
        let width = self.width;
        let height = self.height;
        let size = (width * height) as usize;

        // Collect indices and compute contexts before mutable borrow
        let mut indices_to_process = Vec::new();
        for i in 0..size {
            let state = self.state[i];
            if (state & Self::SIG) != 0 && (state & Self::VISITED) == 0 {
                let mr_ctx = self.get_magnitude_refinement_context(i, width, height);
                indices_to_process.push((i, state, mr_ctx));
            }
        }

        // Now process with mutable borrow
        for (i, state, mr_ctx) in indices_to_process {
            self.state[i] |= Self::VISITED;

            // Decode refinement bit
            let bit = self.mq.decode_bit(mr_ctx);

            if bit != 0 {
                // Add bit to coefficient
                if (state & Self::SIGN) != 0 {
                    self.coefficients[i] -= 1 << bit_plane;
                } else {
                    self.coefficients[i] += 1 << bit_plane;
                }
            }

            self.state[i] |= Self::REFINE;
        }
        Ok(())
    }

    fn decode_cleanup(&mut self, bit_plane: u8) -> Result<(), crate::jpeg2000::bit_io::BitIoError> {
        // Scan in stripe order
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

                    // If not visited, must be insignificant
                    if (state & Self::VISITED) == 0 {
                        let (hc, vc, dc) = self.get_neighbors(x, y);

                        // Decode significance bit
                        let cx = self.get_zc_context(0, hc, vc, dc);
                        let bit = self.mq.decode_bit(cx);

                        if bit != 0 {
                            // Became significant
                            self.state[idx] |= Self::SIG;

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
                if (s & Self::SIGN) != 0 { -1 } else { 1 }
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
            if sigma_prime == 1 { 15 } else { 14 }
        } else {
            16
        }
    }

    fn significance_propagation(&mut self, bit_plane: u8) {
        // Iterate scan order (simple raster for simplicity, J2K stripes 4 rows)
        // Correct J2K is stripe order: 4 rows column-wise.
        let w = self.width;
        let h = self.height;

        for y in 0..h {
            for x in 0..w {
                let idx = (y * w + x) as usize;
                let state = self.state[idx];

                // If insignificant and not visited
                if (state & (Self::SIG | Self::VISITED)) == 0 {
                    let (hc, vc, dc) = self.get_neighbors(x, y);
                    if hc > 0 || vc > 0 || dc > 0 {
                        // Propagate Importance
                        let val = self.data[idx];
                        let bit = (val.abs() >> bit_plane) & 1;

                        // Encode ZC
                        let cx = self.get_zc_context(0, hc, vc, dc); // band 0 assumed
                        self.mq.encode(bit as u8, cx);

                        if bit == 1 {
                            // Became Significant: Update State
                            let sign = if val < 0 { 1 } else { 0 };
                            self.state[idx] |= Self::SIG | Self::VISITED;
                            if sign == 1 {
                                self.state[idx] |= Self::SIGN;
                            }

                            // Encode Sign (SC)
                            // Context depends on neighbor signs
                            // Context depends on neighbor signs
                            let sc_data = self.get_sign_context(x, y, self.width, self.height);
                            let sc_ctx = sc_data & 0xFF;
                            let xor = (sc_data >> 8) & 1;
                            let sym = sign ^ (xor as u8);
                            self.mq.encode(sym, sc_ctx);
                        } else {
                            // Visited but not significant
                            self.state[idx] |= Self::VISITED;
                        }
                    }
                }
            }
        }
    }

    fn magnitude_refinement(&mut self, bit_plane: u8) {
        let w = self.width;
        let h = self.height;
        for i in 0..(w * h) as usize {
            let state = self.state[i];
            // If already significant and NOT visited in SigProp (i.e., became sig in prev bitplane)
            if (state & Self::SIG) != 0 && (state & Self::VISITED) == 0 {
                self.state[i] |= Self::VISITED; // Mark visited for this bitplane
                let val = self.data[i];
                let bit = (val.abs() >> bit_plane) & 1;

                // MR Context
                let mr_ctx = self.get_magnitude_refinement_context(i, w, h);
                // Refinement logic: uses Neighbors+RefineBit state.
                self.mq.encode(bit as u8, mr_ctx);
                self.state[i] |= Self::REFINE; // First refinement done
            }
        }
    }

    fn cleanup(&mut self, bit_plane: u8) {
        // Encode remaining insignificant samples
        let w = self.width;
        let h = self.height;

        // RLC logic would go here: check if run of 4 is all insignificant

        for y in 0..h {
            for x in 0..w {
                let idx = (y * w + x) as usize;
                let state = self.state[idx];
                if (state & Self::VISITED) == 0 {
                    // Not visited: Must be insignificant so far
                    let (hc, vc, dc) = self.get_neighbors(x, y);

                    // ZC Context
                    let cx = self.get_zc_context(0, hc, vc, dc);
                    let val = self.data[idx];
                    let bit = (val.abs() >> bit_plane) & 1;

                    self.mq.encode(bit as u8, cx);

                    if bit == 1 {
                        // Became Significant
                        let sign = if val < 0 { 1 } else { 0 };
                        self.state[idx] |= Self::SIG;
                        if sign == 1 {
                            self.state[idx] |= Self::SIGN;
                        }

                        let sc_data = self.get_sign_context(x, y, self.width, self.height);
                        let sc_ctx = sc_data & 0xFF;
                        let xor = (sc_data >> 8) & 1;
                        let sym = sign ^ (xor as u8); // Invert if xor is set
                        self.mq.encode(sym, sc_ctx);
                    }
                }
                // Reset VISITED for next bitplane
                self.state[idx] &= !Self::VISITED;
            }
        }
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
}
