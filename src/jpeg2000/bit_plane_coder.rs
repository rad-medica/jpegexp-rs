use super::mq_coder::MqCoder;
use crate::JpeglsError;

pub struct BitPlaneCoder<'a> {
    pub width: u32,
    pub height: u32,
    pub data: &'a [i32],        // Source data (for encoder)
    pub coefficients: Vec<i32>, // Decoded magnitude * sign (for decoder)
    pub state: Vec<u8>,         // Sigma, Sigma', Eta
    pub mq: MqCoder,
    pub num_passes_decoded: u32,

    // Neighbor state grid (padded)
    // Bits: 0: Sigma (significant), 1: Sigma' (refined), 2: Visited, 3: Sign
    padded_flags: Vec<u8>,
    stride: usize,
}

impl<'a> BitPlaneCoder<'a> {
    const SIG: u8 = 1 << 0;
    const REFINE: u8 = 1 << 1;
    const VISITED: u8 = 1 << 2;
    const SIGN: u8 = 1 << 3;

    pub fn new(width: u32, height: u32, data: &'a [i32]) -> Self {
        let size = (width * height) as usize;
        let stride = width as usize + 2;
        let padded_flags = vec![0u8; (height as usize + 2) * stride];

        let mut mq = MqCoder::new();
        mq.init_contexts(19);
        // Default init (usually Index 0, MPS 0, except AGG=Index 3, UNI=Index 46, ZC0=Index 4)
        for i in 0..19 {
            mq.set_context(i, 0);
        }
        mq.set_context(17, 3 << 1);
        mq.set_context(18, 46 << 1);
        mq.set_context(0, 4 << 1);

        Self {
            width,
            height,
            data,
            coefficients: vec![0; size],
            state: vec![0; size],
            mq,
            num_passes_decoded: 0,
            padded_flags,
            stride,
        }
    }

    fn reset_flags(&mut self) {
        self.padded_flags.fill(0);
    }

    fn update_flags(&mut self, x: u32, y: u32, sig: bool, sign: Option<u8>) {
        let idx = (y as usize + 1) * self.stride + (x as usize + 1);
        if sig {
            self.padded_flags[idx] |= Self::SIG;
        }
        if let Some(s) = sign {
            if s != 0 {
                self.padded_flags[idx] |= Self::SIGN;
            } else {
                self.padded_flags[idx] &= !Self::SIGN;
            }
        }
    }

    fn get_context_zc(&self, x: u32, y: u32, orientation: u8) -> usize {
        let idx = (y as usize + 1) * self.stride + (x as usize + 1);
        let s = self.stride;

        let mut h = ((self.padded_flags[idx - 1] & Self::SIG) != 0) as u8
            + ((self.padded_flags[idx + 1] & Self::SIG) != 0) as u8;
        let mut v = ((self.padded_flags[idx - s] & Self::SIG) != 0) as u8
            + ((self.padded_flags[idx + s] & Self::SIG) != 0) as u8;
        let d = ((self.padded_flags[idx - s - 1] & Self::SIG) != 0) as u8
            + ((self.padded_flags[idx - s + 1] & Self::SIG) != 0) as u8
            + ((self.padded_flags[idx + s - 1] & Self::SIG) != 0) as u8
            + ((self.padded_flags[idx + s + 1] & Self::SIG) != 0) as u8;

        // CRITICAL FIX: Swap h and v for LH orientation (orient=2)
        // This matches OpenJPEG's t1_generate_luts.c implementation (lines 58-61)
        if orientation == 2 {
            std::mem::swap(&mut h, &mut v);
        }

        match orientation {
            0 | 1 | 2 => {
                // LL, HL, LH - All use same logic after potential h/v swap for LH
                // This matches OpenJPEG's t1_generate_luts.c lines 64-90
                if h == 0 {
                    if v == 0 {
                        if d == 0 {
                            0
                        } else if d == 1 {
                            1
                        } else {
                            2
                        }
                    } else if v == 1 {
                        3
                    } else {
                        4
                    }
                } else if h == 1 {
                    if v == 0 {
                        if d == 0 {
                            5
                        } else {
                            6
                        }
                    } else {
                        7
                    }
                } else {
                    8
                }
            }
            3 => {
                // HH - Diagonal orientation uses different logic
                // This matches OpenJPEG's t1_generate_luts.c lines 92-118
                let hv = h + v;
                if d == 0 {
                    if hv == 0 {
                        0
                    } else if hv == 1 {
                        1
                    } else {
                        2
                    }
                } else if d == 1 {
                    if hv == 0 {
                        3
                    } else if hv == 1 {
                        4
                    } else {
                        5
                    }
                } else if d == 2 {
                    if hv == 0 {
                        6
                    } else {
                        7
                    }
                } else {
                    8
                }
            }
            _ => 0,
        }
    }

    fn get_context_sc(&self, x: u32, y: u32) -> (usize, u8) {
        let idx = (y as usize + 1) * self.stride + (x as usize + 1);
        let s = self.stride;

        let get_s = |off: usize| -> i32 {
            let f = self.padded_flags[off];
            if (f & Self::SIG) != 0 {
                if (f & Self::SIGN) != 0 {
                    -1
                } else {
                    1
                }
            } else {
                0
            }
        };

        let h_sum = get_s(idx - 1) + get_s(idx + 1);
        let v_sum = get_s(idx - s) + get_s(idx + s);

        let h = h_sum.clamp(-1, 1);
        let v = v_sum.clamp(-1, 1);

        // Standard Table C.5
        match (h, v) {
            (1, 1) => (13, 0),
            (1, 0) => (12, 0),
            (1, -1) => (11, 0),
            (0, 1) => (10, 0),
            (0, 0) => (9, 0),
            (0, -1) => (10, 1),
            (-1, 1) => (11, 1),
            (-1, 0) => (12, 1),
            (-1, -1) => (13, 1),
            _ => (9, 0),
        }
    }

    fn get_context_mag(&self, x: u32, y: u32) -> usize {
        let idx = (y as usize + 1) * self.stride + (x as usize + 1);
        let s = self.stride;
        if (self.padded_flags[idx] & Self::REFINE) != 0 {
            16
        } else {
            let neighbors = (self.padded_flags[idx - 1]
                | self.padded_flags[idx + 1]
                | self.padded_flags[idx - s]
                | self.padded_flags[idx + s]
                | self.padded_flags[idx - s - 1]
                | self.padded_flags[idx - s + 1]
                | self.padded_flags[idx + s - 1]
                | self.padded_flags[idx + s + 1])
                & Self::SIG;
            if neighbors != 0 {
                15
            } else {
                14
            }
        }
    }

    pub fn get_mq_contexts(&self) -> Vec<u8> {
        self.mq.contexts.clone()
    }

    pub fn set_mq_contexts(&mut self, contexts: &[u8]) {
        self.mq.contexts = contexts.to_vec();
    }

    // --- Encoding passes ---

    pub fn calculate_max_bit_plane(&self) -> Option<u8> {
        let max_val = self.data.iter().map(|&v| v.abs()).max().unwrap_or(0);
        if max_val == 0 {
            return None;
        }
        let mut bp = 0;
        while (1 << (bp + 1)) <= max_val {
            bp += 1;
        }
        Some(bp)
    }

    pub fn encode_codeblock(&mut self, start_bp: u8, min_bp: u8, orient: u8) -> u8 {
        self.mq.init_encoder();
        self.reset_flags();
        self.state.fill(0);

        // Initial cleanup pass
        self.encode_cleanup(start_bp, orient);
        let mut passes = 1;

        if start_bp > min_bp {
            for bp in (min_bp..start_bp).rev() {
                self.encode_sigprop(bp, orient);
                self.encode_magref(bp);
                self.encode_cleanup(bp, orient);
                passes += 3;
            }
        }
        passes
    }

    fn encode_sigprop(&mut self, bp: u8, orient: u8) {
        for y_stripe in (0..self.height).step_by(4) {
            for x in 0..self.width {
                for y in y_stripe..(y_stripe + 4).min(self.height) {
                    let idx = (y * self.width + x) as usize;
                    if (self.state[idx] & Self::SIG) == 0 {
                        let (h, v, d) = self.get_neighbor_counts(x, y);
                        if h + v + d > 0 {
                            let val = self.data[idx];
                            let bit = ((val.abs() >> bp) & 1) as u8;
                            let cx = self.get_context_zc(x, y, orient);
                            self.mq.encode(bit, cx);
                            if bit != 0 {
                                let sign = (val < 0) as u8;
                                let (cx_sc, xor) = self.get_context_sc(x, y);
                                self.mq.encode(sign ^ xor, cx_sc);
                                self.state[idx] |= Self::SIG;
                                self.update_flags(x, y, true, Some(sign));
                            }
                            self.state[idx] |= Self::VISITED;
                        }
                    }
                }
            }
        }
    }

    fn encode_magref(&mut self, bp: u8) {
        for y_stripe in (0..self.height).step_by(4) {
            for x in 0..self.width {
                for y in y_stripe..(y_stripe + 4).min(self.height) {
                    let idx = (y * self.width + x) as usize;
                    if (self.state[idx] & Self::SIG) != 0 && (self.state[idx] & Self::VISITED) == 0
                    {
                        let bit = ((self.data[idx].abs() >> bp) & 1) as u8;
                        let cx = self.get_context_mag(x, y);
                        self.mq.encode(bit, cx);
                        self.state[idx] |= Self::REFINE;
                        self.update_flag_refined(x, y);
                    }
                }
            }
        }
    }

    fn encode_cleanup(&mut self, bp: u8, orient: u8) {
        for y_stripe in (0..self.height).step_by(4) {
            for x in 0..self.width {
                // Check if we can use RLC (Run-Length Coding)
                let stripe_height = (y_stripe + 4).min(self.height) - y_stripe;
                let mut all_insignificant = true;
                let mut all_no_neighbors = true;

                // Check if all pixels in this stripe column are candidates for RLC
                for y in y_stripe..(y_stripe + stripe_height).min(self.height) {
                    let idx = (y * self.width + x) as usize;
                    if (self.state[idx] & (Self::SIG | Self::VISITED)) != 0 {
                        all_insignificant = false;
                        break;
                    }
                    let (h, v, d) = self.get_neighbor_counts(x, y);
                    if h + v + d > 0 {
                        all_no_neighbors = false;
                    }
                }

                // Use RLC if all 4 pixels are insignificant with no significant neighbors
                if stripe_height == 4 && all_insignificant && all_no_neighbors {
                    // Find first significant pixel (runlen)
                    let mut runlen = 4u8;
                    for i in 0..4 {
                        let y = y_stripe + i;
                        let idx = (y * self.width + x) as usize;
                        let val = self.data[idx];
                        let bit = ((val.abs() >> bp) & 1) as u8;
                        if bit != 0 {
                            runlen = i as u8;
                            break;
                        }
                    }

                    // Encode aggregate bit (AGG context 17)
                    self.mq.encode((runlen != 4) as u8, 17);

                    if runlen < 4 {
                        // Encode runlen using 2 bits (UNI context 18)
                        self.mq.encode((runlen >> 1) & 1, 18);
                        self.mq.encode(runlen & 1, 18);

                        // Encode pixels starting from runlen
                        for i in runlen..4 {
                            let y = y_stripe + i as u32;
                            let idx = (y * self.width + x) as usize;
                            let val = self.data[idx];
                            let bit = ((val.abs() >> bp) & 1) as u8;

                            // First pixel after runlen is known to be significant
                            // (that's what runlen tells us), so skip zero-context encoding
                            if i == runlen {
                                // Pixel at runlen is significant, encode sign only
                                let sign = (val < 0) as u8;
                                let (cx_sc, xor) = self.get_context_sc(x, y);
                                self.mq.encode(sign ^ xor, cx_sc);
                                self.state[idx] |= Self::SIG;
                                self.update_flags(x, y, true, Some(sign));
                            } else {
                                // For pixels after runlen, encode normally
                                let cx = self.get_context_zc(x, y, orient);
                                self.mq.encode(bit, cx);
                                if bit != 0 {
                                    let sign = (val < 0) as u8;
                                    let (cx_sc, xor) = self.get_context_sc(x, y);
                                    self.mq.encode(sign ^ xor, cx_sc);
                                    self.state[idx] |= Self::SIG;
                                    self.update_flags(x, y, true, Some(sign));
                                }
                            }
                        }
                    }
                } else {
                    // No RLC - encode each pixel normally
                    for y in y_stripe..(y_stripe + 4).min(self.height) {
                        let idx = (y * self.width + x) as usize;
                        if (self.state[idx] & (Self::SIG | Self::VISITED)) == 0 {
                            let val = self.data[idx];
                            let bit = ((val.abs() >> bp) & 1) as u8;
                            let cx = self.get_context_zc(x, y, orient);
                            self.mq.encode(bit, cx);
                            if bit != 0 {
                                let sign = (val < 0) as u8;
                                let (cx_sc, xor) = self.get_context_sc(x, y);
                                self.mq.encode(sign ^ xor, cx_sc);
                                self.state[idx] |= Self::SIG;
                                self.update_flags(x, y, true, Some(sign));
                            }
                        }
                    }
                }
            }
        }
        for s in &mut self.state {
            *s &= !Self::VISITED;
        }
    }

    // --- Decoding passes ---

    pub fn decode_codeblock(
        &mut self,
        data: &[u8],
        start_bp: u8,
        num_passes: u8,
        orient: u8,
    ) -> Result<Vec<i32>, JpeglsError> {
        self.mq.init_decoder(data);
        self.reset_flags();
        self.state.fill(0);
        self.coefficients.fill(0);

        let mut pass_idx = 0;
        if num_passes > 0 {
            // Cleanup first
            self.decode_cleanup(start_bp, orient);
            pass_idx += 1;

            let mut bp = start_bp;
            while pass_idx < num_passes {
                bp = bp.saturating_sub(1);
                self.decode_sigprop(bp, orient);
                pass_idx += 1;
                if pass_idx >= num_passes {
                    break;
                }

                self.decode_magref(bp);
                pass_idx += 1;
                if pass_idx >= num_passes {
                    break;
                }

                self.decode_cleanup(bp, orient);
                pass_idx += 1;
            }
        }

        // Finalize coefficients: return magnitude * sign
        Ok(self.coefficients.clone())
    }

    fn decode_sigprop(&mut self, bp: u8, orient: u8) {
        for y_stripe in (0..self.height).step_by(4) {
            for x in 0..self.width {
                for y in y_stripe..(y_stripe + 4).min(self.height) {
                    let idx = (y * self.width + x) as usize;
                    if (self.state[idx] & Self::SIG) == 0 {
                        let (h, v, d) = self.get_neighbor_counts(x, y);
                        if h + v + d > 0 {
                            let cx = self.get_context_zc(x, y, orient);
                            let bit = self.mq.decode_bit(cx);
                            if bit != 0 {
                                let (cx_sc, xor) = self.get_context_sc(x, y);
                                let sign = self.mq.decode_bit(cx_sc) ^ xor;
                                self.state[idx] |= Self::SIG;
                                self.coefficients[idx] = 1 << bp;
                                if sign != 0 {
                                    self.coefficients[idx] = -self.coefficients[idx];
                                }
                                self.update_flags(x, y, true, Some(sign));
                            }
                            self.state[idx] |= Self::VISITED;
                        }
                    }
                }
            }
        }
    }

    fn decode_magref(&mut self, bp: u8) {
        for y_stripe in (0..self.height).step_by(4) {
            for x in 0..self.width {
                for y in y_stripe..(y_stripe + 4).min(self.height) {
                    let idx = (y * self.width + x) as usize;
                    if (self.state[idx] & Self::SIG) != 0 && (self.state[idx] & Self::VISITED) == 0
                    {
                        let cx = self.get_context_mag(x, y);
                        let bit = self.mq.decode_bit(cx);
                        if bit != 0 {
                            if self.coefficients[idx] > 0 {
                                self.coefficients[idx] += 1 << bp;
                            } else {
                                self.coefficients[idx] -= 1 << bp;
                            }
                        }
                        self.state[idx] |= Self::REFINE;
                        self.update_flag_refined(x, y);
                    }
                }
            }
        }
    }

    fn decode_cleanup(&mut self, bp: u8, orient: u8) {
        for y_stripe in (0..self.height).step_by(4) {
            for x in 0..self.width {
                // Check if we should decode using RLC (Run-Length Coding)
                let stripe_height = (y_stripe + 4).min(self.height) - y_stripe;
                let mut all_insignificant = true;
                let mut all_no_neighbors = true;

                for y in y_stripe..(y_stripe + stripe_height).min(self.height) {
                    let idx = (y * self.width + x) as usize;
                    if (self.state[idx] & (Self::SIG | Self::VISITED)) != 0 {
                        all_insignificant = false;
                        break;
                    }
                    let (h, v, d) = self.get_neighbor_counts(x, y);
                    if h + v + d > 0 {
                        all_no_neighbors = false;
                    }
                }

                // Use RLC if all 4 pixels are insignificant with no significant neighbors
                if stripe_height == 4 && all_insignificant && all_no_neighbors {
                    // Decode aggregate bit (AGG context 17)
                    let agg = self.mq.decode_bit(17);

                    if agg != 0 {
                        // Decode runlen using 2 bits (UNI context 18)
                        let bit1 = self.mq.decode_bit(18);
                        let bit0 = self.mq.decode_bit(18);
                        let runlen = (bit1 << 1) | bit0;

                        // Decode pixels starting from runlen
                        for i in runlen..4 {
                            let y = y_stripe + i as u32;
                            let idx = (y * self.width + x) as usize;

                            // First pixel after runlen is known to be significant
                            // (that's what runlen tells us), so skip zero-context decoding
                            if i == runlen {
                                // Pixel at runlen is significant, decode sign only
                                let (cx_sc, xor) = self.get_context_sc(x, y);
                                let sign = self.mq.decode_bit(cx_sc) ^ xor;
                                self.state[idx] |= Self::SIG;
                                self.coefficients[idx] = 1 << bp;
                                if sign != 0 {
                                    self.coefficients[idx] = -self.coefficients[idx];
                                }
                                self.update_flags(x, y, true, Some(sign));
                            } else {
                                // For pixels after runlen, decode normally
                                let cx = self.get_context_zc(x, y, orient);
                                let bit = self.mq.decode_bit(cx);
                                if bit != 0 {
                                    let (cx_sc, xor) = self.get_context_sc(x, y);
                                    let sign = self.mq.decode_bit(cx_sc) ^ xor;
                                    self.state[idx] |= Self::SIG;
                                    self.coefficients[idx] = 1 << bp;
                                    if sign != 0 {
                                        self.coefficients[idx] = -self.coefficients[idx];
                                    }
                                    self.update_flags(x, y, true, Some(sign));
                                }
                            }
                        }
                    }
                } else {
                    // No RLC - decode each pixel normally
                    for y in y_stripe..(y_stripe + 4).min(self.height) {
                        let idx = (y * self.width + x) as usize;
                        if (self.state[idx] & (Self::SIG | Self::VISITED)) == 0 {
                            let cx = self.get_context_zc(x, y, orient);
                            let bit = self.mq.decode_bit(cx);
                            if bit != 0 {
                                let (cx_sc, xor) = self.get_context_sc(x, y);
                                let sign = self.mq.decode_bit(cx_sc) ^ xor;
                                self.state[idx] |= Self::SIG;
                                self.coefficients[idx] = 1 << bp;
                                if sign != 0 {
                                    self.coefficients[idx] = -self.coefficients[idx];
                                }
                                self.update_flags(x, y, true, Some(sign));
                            }
                        }
                    }
                }
            }
        }
        for s in &mut self.state {
            *s &= !Self::VISITED;
        }
    }

    // --- Helpers ---

    fn get_neighbor_counts(&self, x: u32, y: u32) -> (u8, u8, u8) {
        let idx = (y as usize + 1) * self.stride + (x as usize + 1);
        let s = self.stride;
        let h = ((self.padded_flags[idx - 1] & Self::SIG) != 0) as u8
            + ((self.padded_flags[idx + 1] & Self::SIG) != 0) as u8;
        let v = ((self.padded_flags[idx - s] & Self::SIG) != 0) as u8
            + ((self.padded_flags[idx + s] & Self::SIG) != 0) as u8;
        let d = ((self.padded_flags[idx - s - 1] & Self::SIG) != 0) as u8
            + ((self.padded_flags[idx - s + 1] & Self::SIG) != 0) as u8
            + ((self.padded_flags[idx + s - 1] & Self::SIG) != 0) as u8
            + ((self.padded_flags[idx + s + 1] & Self::SIG) != 0) as u8;
        (h, v, d)
    }

    fn update_flag_refined(&mut self, x: u32, y: u32) {
        let idx = (y as usize + 1) * self.stride + (x as usize + 1);
        self.padded_flags[idx] |= Self::REFINE;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_bpc_roundtrip() {
        let data = [0, 1, 2, 3, 4, 5, 6, 7];
        let mut bpc = BitPlaneCoder::new(8, 1, &data);
        let passes = bpc.encode_codeblock(3, 0);
        bpc.mq.flush();
        let buf = bpc.mq.get_buffer().to_vec();
        let mut dec = BitPlaneCoder::new(8, 1, &[]);
        let res = dec.decode_codeblock(&buf, 3, passes, 0).unwrap();
        assert_eq!(res, [0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn test_small_constant_block() {
        // Test small constant block first - just 8 pixels all = 7
        let data = [7, 7, 7, 7, 7, 7, 7, 7];
        let mut bpc = BitPlaneCoder::new(8, 1, &data);
        let max_bp = bpc.calculate_max_bit_plane().expect("Should have max_bp");
        println!("Encoding constant 7s with max_bp={}", max_bp);
        let passes = bpc.encode_codeblock(max_bp, 0);
        bpc.mq.flush();
        let buf = bpc.mq.get_buffer().to_vec();
        println!("Encoded to {} bytes, {} passes", buf.len(), passes);

        let mut dec = BitPlaneCoder::new(8, 1, &[]);
        let res = dec.decode_codeblock(&buf, max_bp, passes, 0).unwrap();

        println!("Small constant: {:?} -> {:?}", data, res);
        assert_eq!(res, data.to_vec(), "Small constant block should roundtrip");
    }

    #[test]
    fn test_medium_constant_block() {
        // Test medium constant block - 64 pixels (8x8) all = 255
        let width = 8;
        let height = 8;
        let data = vec![255i32; (width * height) as usize];

        let mut bpc = BitPlaneCoder::new(width as u32, height as u32, &data);
        let max_bp = bpc.calculate_max_bit_plane().expect("Should have max_bp");
        println!("Encoding 8x8 constant 255s with max_bp={}", max_bp);
        let passes = bpc.encode_codeblock(max_bp, 0);
        bpc.mq.flush();
        let buf = bpc.mq.get_buffer().to_vec();
        println!("Encoded to {} bytes, {} passes", buf.len(), passes);

        let mut dec = BitPlaneCoder::new(width as u32, height as u32, &[]);
        let res = dec.decode_codeblock(&buf, max_bp, passes, 0).unwrap();

        let mut errors = 0;
        for (i, (&orig, &dec_val)) in data.iter().zip(res.iter()).enumerate() {
            if orig != dec_val {
                if errors < 5 {
                    println!("Mismatch at [{}]: {} -> {}", i, orig, dec_val);
                }
                errors += 1;
            }
        }

        assert_eq!(errors, 0, "Medium constant block (8x8) should roundtrip");
    }

    #[test]
    fn test_large_16x16_constant_block() {
        // Test 16x16 constant block
        let width = 16;
        let height = 16;
        let data = vec![255i32; (width * height) as usize];

        let mut bpc = BitPlaneCoder::new(width as u32, height as u32, &data);
        let max_bp = bpc.calculate_max_bit_plane().expect("Should have max_bp");
        println!("Encoding 16x16 constant 255s with max_bp={}", max_bp);
        let passes = bpc.encode_codeblock(max_bp, 0);
        bpc.mq.flush();
        let buf = bpc.mq.get_buffer().to_vec();
        println!("Encoded to {} bytes, {} passes", buf.len(), passes);

        let mut dec = BitPlaneCoder::new(width as u32, height as u32, &[]);
        let res = dec.decode_codeblock(&buf, max_bp, passes, 0).unwrap();

        let mut errors = 0;
        for (i, (&orig, &dec_val)) in data.iter().zip(res.iter()).enumerate() {
            if orig != dec_val {
                if errors < 5 {
                    println!("Mismatch at [{}]: {} -> {}", i, orig, dec_val);
                }
                errors += 1;
            }
        }

        println!("16x16 test: {} errors out of {}", errors, data.len());
        assert_eq!(errors, 0, "16x16 constant block should roundtrip");
    }

    #[test]
    fn test_constant_block_roundtrip() {
        // Test various sizes to find where it breaks
        for &size in &[4, 8, 16, 32, 64] {
            let width = size;
            let height = size;
            let test_value = 255i32;
            let data = vec![test_value; (width * height) as usize];

            let mut bpc = BitPlaneCoder::new(width as u32, height as u32, &data);
            let max_bp = bpc.calculate_max_bit_plane().expect("Should have max_bp");
            let passes = bpc.encode_codeblock(max_bp, 0);
            bpc.mq.flush();
            let buf = bpc.mq.get_buffer().to_vec();

            let mut dec = BitPlaneCoder::new(width as u32, height as u32, &[]);
            let res = dec.decode_codeblock(&buf, max_bp, passes, 0).unwrap();

            let mut errors = 0;
            for (i, (&orig, &dec_val)) in data.iter().zip(res.iter()).enumerate() {
                if orig != dec_val {
                    if errors < 3 {
                        println!(
                            "{}x{}: Mismatch at [{}]: {} -> {}",
                            size, size, i, orig, dec_val
                        );
                    }
                    errors += 1;
                }
            }

            if errors > 0 {
                println!("❌ {}x{} FAILED: {} errors", size, size, errors);
            } else {
                println!("✅ {}x{} passed", size, size);
            }

            assert_eq!(
                errors, 0,
                "{}x{} constant block should roundtrip",
                size, size
            );
        }
    }

    #[test]
    fn test_constant_8190_block_roundtrip() {
        // Test the specific value 8190 that appears in 12-bit checkerboards
        let width = 32;
        let height = 32;
        let data = vec![8190i32; (width * height) as usize];

        let mut bpc = BitPlaneCoder::new(width, height, &data);
        let max_bp = bpc
            .calculate_max_bit_plane()
            .expect("Should have max_bp for 8190");
        println!("max_bp for 8190: {}", max_bp);

        let passes = bpc.encode_codeblock(max_bp, 3);
        bpc.mq.flush();
        let encoded = bpc.mq.get_buffer().to_vec();
        println!(
            "Encoded {} values into {} bytes with {} passes",
            data.len(),
            encoded.len(),
            passes
        );

        // Decode
        let mut bpc_dec = BitPlaneCoder::new(width, height, &[]);
        let decoded = bpc_dec
            .decode_codeblock(&encoded, max_bp, passes, 3)
            .unwrap();

        // Check
        let mut errors = 0;
        for (i, (&orig, &dec)) in data.iter().zip(decoded.iter()).enumerate() {
            if orig != dec {
                if errors < 10 {
                    println!("Mismatch at [{}]: {} -> {}", i, orig, dec);
                }
                errors += 1;
            }
        }

        if errors == 0 {
            println!("✅ Perfect roundtrip for 8190!");
        } else {
            println!("❌ {} mismatches out of {}", errors, data.len());
        }

        assert_eq!(
            errors, 0,
            "Should have perfect roundtrip for constant 8190 block"
        );
    }
}
