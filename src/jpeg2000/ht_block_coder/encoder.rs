//! HTJ2K (High-Throughput JPEG 2000) Block Encoder
//! Implements encoding for ISO/IEC 15444-15

use super::vlc;
use crate::jpeg2000::image::J2kCodeBlock;

/// MEL (Magnitude Exponent Logic) encoder
/// Encodes run-lengths of insignificant quads
pub struct MelEncoder {
    buffer: Vec<u8>,
    current_byte: u8,
    bits_in_byte: u8,
    k: i32, // State index (exponent)
}

impl MelEncoder {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            current_byte: 0,
            bits_in_byte: 0,
            k: 0,
        }
    }

    /// Write a single bit
    fn write_bit(&mut self, bit: u8) {
        self.current_byte = (self.current_byte << 1) | (bit & 1);
        self.bits_in_byte += 1;

        if self.bits_in_byte == 8 {
            // Handle 0xFF byte stuffing
            if self.current_byte == 0xFF {
                self.buffer.push(0xFF);
                self.buffer.push(0x00);
            } else {
                self.buffer.push(self.current_byte);
            }
            self.current_byte = 0;
            self.bits_in_byte = 0;
        }
    }

    /// Encode a MEL symbol (significant or not)
    /// Returns false if still in a run, true if this ends a run
    pub fn encode(&mut self, is_significant: bool) {
        if is_significant {
            // End of run - write 1, decrease k
            self.write_bit(1);
            self.k = (self.k - 1).max(0);
        } else {
            // Start/continue run - write 0, increase k
            self.write_bit(0);
            self.k = (self.k + 1).min(12);
        }
    }

    /// Flush remaining bits to buffer
    pub fn flush(&mut self) {
        if self.bits_in_byte > 0 {
            // Pad with zeros
            let padding = 8 - self.bits_in_byte;
            self.current_byte <<= padding;
            self.buffer.push(self.current_byte);
        }
    }

    /// Get the encoded buffer
    pub fn get_buffer(&self) -> &[u8] {
        &self.buffer
    }
}

impl Default for MelEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// MagSgn (Magnitude and Sign) encoder
/// Encodes sign bits and magnitude refinement bits
pub struct MagSgnEncoder {
    buffer: Vec<u8>,
    current_byte: u8,
    bits_in_byte: u8,
}

impl MagSgnEncoder {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            current_byte: 0,
            bits_in_byte: 0,
        }
    }

    /// Write a single bit
    pub fn write_bit(&mut self, bit: u8) {
        self.current_byte = (self.current_byte << 1) | (bit & 1);
        self.bits_in_byte += 1;

        if self.bits_in_byte == 8 {
            self.buffer.push(self.current_byte);
            self.current_byte = 0;
            self.bits_in_byte = 0;
        }
    }

    /// Write multiple bits (MSB first)
    pub fn write_bits(&mut self, value: u32, count: u8) {
        for i in (0..count).rev() {
            self.write_bit(((value >> i) & 1) as u8);
        }
    }

    /// Flush remaining bits to buffer
    pub fn flush(&mut self) {
        if self.bits_in_byte > 0 {
            let padding = 8 - self.bits_in_byte;
            self.current_byte <<= padding;
            self.buffer.push(self.current_byte);
        }
    }

    /// Get the encoded buffer
    pub fn get_buffer(&self) -> &[u8] {
        &self.buffer
    }
}

impl Default for MagSgnEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// High Throughput Block Encoder (HTJ2K Part 15)
/// Encodes code-blocks using non-iterative entropy coding
pub struct HTBlockEncoder {
    mel_encoder: MelEncoder,
    magsgn_encoder: MagSgnEncoder,
    vlc_buffer: Vec<u8>,
    vlc_bits: u8,
    vlc_current: u8,
    width: usize,
    height: usize,
    stripe_height: usize,
    num_quads_x: usize,
    quad_exponents: Vec<u8>, // Max exponent E_q for each quad
}

impl HTBlockEncoder {
    pub fn new(width: usize, height: usize) -> Self {
        let num_quads_x = (width + 1) / 2;
        Self {
            mel_encoder: MelEncoder::new(),
            magsgn_encoder: MagSgnEncoder::new(),
            vlc_buffer: Vec::new(),
            vlc_bits: 0,
            vlc_current: 0,
            width,
            height,
            stripe_height: 4,
            num_quads_x,
            quad_exponents: vec![0; num_quads_x * ((height + 1) / 2)],
        }
    }

    /// Write VLC bits (encoded backwards from end of packet)
    fn write_vlc_bit(&mut self, bit: u8) {
        self.vlc_current = (self.vlc_current >> 1) | ((bit & 1) << 7);
        self.vlc_bits += 1;

        if self.vlc_bits == 8 {
            self.vlc_buffer.push(self.vlc_current);
            self.vlc_current = 0;
            self.vlc_bits = 0;
        }
    }

    /// Encode an entire code-block
    pub fn encode_block(&mut self, block: &J2kCodeBlock) -> Result<Vec<u8>, ()> {
        // HTJ2K encoding flow:
        // 1. Process quads in stripe order
        // 2. For each quad pair (within a stripe):
        //    a. Calculate E_q residual (u_q) and UVLC codeword
        //    b. Encode VLC codeword for each quad
        //    c. Encode sign/magnitude bits via MagSgn with EMB optimization

        for y_stripe in (0..self.height).step_by(self.stripe_height) {
            for x in (0..self.width).step_by(2) {
                // Process quads in pairs vertically within the 4-high stripe
                self.encode_quad_pair(x, y_stripe, block)?;
            }
        }

        // Finalize encoders
        self.mel_encoder.flush();
        self.magsgn_encoder.flush();

        // Flush VLC buffer (reversed)
        if self.vlc_bits > 0 {
            self.vlc_current >>= 8 - self.vlc_bits;
            self.vlc_buffer.push(self.vlc_current);
        }

        // Combine streams:
        // MagSgn grows from start, MEL/VLC grows from end
        let mut output = self.magsgn_encoder.get_buffer().to_vec();

        // Append VLC (reversed) and MEL (reversed)
        let mel_data = self.mel_encoder.get_buffer();
        let vlc_data = &self.vlc_buffer;

        // Interleave MEL and VLC at the end
        // Standard says: concatenated Scup consists of VLC then MEL (both reversed if needed)
        // Actually it's more like: [MagSgn...][...VLC reversed][...MEL reversed]
        for &b in vlc_data.iter().rev() {
            output.push(b);
        }
        for &b in mel_data.iter().rev() {
            output.push(b);
        }

        // Add length information at the end (Scup) as per Part 15?
        // OpenHTJ2K does:
        // fwd_buf[Lcup - 1] = Scup >> 4;
        // fwd_buf[Lcup - 2] |= Scup & 0x0F;
        // We'll skip the length suffix for now or add it if needed for decoder compatibility.
        // Most decoders expect Scup info in the packet header or suffix.

        Ok(output)
    }

    fn encode_quad_pair(&mut self, x: usize, y_base: usize, block: &J2kCodeBlock) -> Result<(), ()> {
        let qx = x / 2;
        let qy0 = y_base / 2;
        let qy1 = qy0 + 1;

        // Quad 0 info
        let (rho0, e_max_actual0, quad_coeffs0) = self.get_quad_info(x, y_base, block);
        let gamma0 = if rho0.count_ones() > 1 { 1 } else { 0 };
        let kappa0 = self.get_kappa(qx, qy0, gamma0);
        
        let u0 = e_max_actual0.max(kappa0);
        let u_q0 = u0 - kappa0;
        let u_off0 = if u_q0 > 0 { 1 } else { 0 };
        self.quad_exponents[qy0 * self.num_quads_x + qx] = e_max_actual0;

        // Quad 1 info
        let has_q1 = y_base + 2 < self.height;
        let (rho1, e_max_actual1, quad_coeffs1) = if has_q1 {
            self.get_quad_info(x, y_base + 2, block)
        } else {
            (0, 0, [0i32; 4])
        };
        let (u1, u_q1, u_off1) = if has_q1 {
            let gamma1 = if rho1.count_ones() > 1 { 1 } else { 0 };
            let kappa1 = self.get_kappa(qx, qy1, gamma1);
            let u1 = e_max_actual1.max(kappa1);
            let u_q1 = u1 - kappa1;
            let u_off1 = if u_q1 > 0 { 1 } else { 0 };
            self.quad_exponents[qy1 * self.num_quads_x + qx] = e_max_actual1;
            (u1, u_q1, u_off1)
        } else {
            (0, 0, 0)
        };

        // 1. MEL encoding
        let context0 = self.calculate_context(x, y_base, block);
        if context0 == 0 {
            self.mel_encoder.encode(rho0 != 0);
        }

        // 2. VLC encoding
        // Quad 0 VLC
        let (vlc0, emb_k0, _e1_0) = vlc::encode_vlc(rho0, context0, u_off0);
        for i in (0..vlc0.bits).rev() {
            self.write_vlc_bit(((vlc0.value >> i) & 1) as u8);
        }

        // Context for Quad 1
        let context1 = (rho0 >> 1) | (rho0 & 1);
        if has_q1 {
            // Quad 1 VLC
            let (vlc1, ek1, _e1_1) = vlc::encode_vlc(rho1, context1, u_off1);
            for i in (0..vlc1.bits).rev() {
                self.write_vlc_bit(((vlc1.value >> i) & 1) as u8);
            }

            // UVLC encoding for u_q residuals
            let uvlc = vlc::encode_uvlc(u_q0, u_q1, 0);
            for i in (0..uvlc.bits).rev() {
                self.write_vlc_bit(((uvlc.value >> i) & 1) as u8);
            }

            // 3. MEL encoding for Quad 1 (simplified)
            if context1 == 0 {
                self.mel_encoder.encode(rho1 != 0);
            }

            // 4. MagSgn encoding
            self.emit_quad_magsgn(rho0, u0, emb_k0, &quad_coeffs0);
            self.emit_quad_magsgn(rho1, u1, ek1, &quad_coeffs1);
        } else {
            // Only Quad 0 UVLC
            let uvlc = vlc::encode_uvlc(u_q0, 0, 0);
            for i in (0..uvlc.bits).rev() {
                self.write_vlc_bit(((uvlc.value >> i) & 1) as u8);
            }
            self.emit_quad_magsgn(rho0, u0, emb_k0, &quad_coeffs0);
        }

        Ok(())
    }

    fn get_kappa(&self, qx: usize, qy: usize, gamma: u8) -> u8 {
        if gamma == 0 {
            return 1;
        }
        let mut max_e = 0u8;
        let neighbors = [
            if qx > 0 && qy > 0 { Some((qx - 1, qy - 1)) } else { None },
            if qy > 0 { Some((qx, qy - 1)) } else { None },
            if qx + 1 < self.num_quads_x && qy > 0 { Some((qx + 1, qy - 1)) } else { None },
            if qx > 0 { Some((qx - 1, qy)) } else { None },
        ];

        for neighbor in neighbors.iter().flatten() {
            let (nx, ny) = *neighbor;
            max_e = max_e.max(self.quad_exponents[ny * self.num_quads_x + nx]);
        }

        max_e.saturating_sub(1).max(1)
    }


    fn get_quad_info(&self, x: usize, y: usize, block: &J2kCodeBlock) -> (u8, u8, [i32; 4]) {
        let coords = [(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)];
        let mut rho = 0u8;
        let mut max_mag = 0u32;
        let mut coeffs = [0i32; 4];

        for (i, &(px, py)) in coords.iter().enumerate() {
            if px < self.width && py < self.height {
                let idx = py * self.width + px;
                if idx < block.coefficients.len() {
                    let c = block.coefficients[idx];
                    coeffs[i] = c;
                    let mag = c.unsigned_abs();
                    if mag > 0 {
                        rho |= 1 << i;
                        max_mag = max_mag.max(mag);
                    }
                }
            }
        }
        let e_q = if max_mag > 0 {
            (32 - max_mag.leading_zeros()) as u8
        } else {
            0
        };
        (rho, e_q, coeffs)
    }

    fn emit_quad_magsgn(&mut self, rho: u8, u_val: u8, emb_k: u8, coeffs: &[i32; 4]) {
        if rho == 0 {
            return;
        }

        // Sign bits first
        for (i, &c) in coeffs.iter().enumerate() {
            if (rho >> i) & 1 == 1 {
                self.magsgn_encoder.write_bit(if c < 0 { 1 } else { 0 });
            }
        }

        // Magnitude bits with EMB optimization
        // E_k = u_val - (emb_k bit i)
        for (i, &c) in coeffs.iter().enumerate() {
            if (rho >> i) & 1 == 1 {
                let mag = c.unsigned_abs();
                let bit_k = (emb_k >> i) & 1;
                let e_k = u_val.saturating_sub(bit_k);

                if e_k > 0 {
                    // Write bits from e_k-1 down to 0 (for lossless)
                    for b in (0..e_k).rev() {
                        self.magsgn_encoder.write_bit(((mag >> b) & 1) as u8);
                    }
                }
            }
        }
    }

    fn calculate_context(&self, x: usize, y_base: usize, block: &J2kCodeBlock) -> u8 {
        // Context based on neighbor significance
        let width = self.width;
        let height = self.height;

        let neighbors = [
            if x >= 2 { Some((x - 2, y_base)) } else { None },
            if y_base >= 2 {
                Some((x, y_base - 2))
            } else {
                None
            },
        ];

        for neighbor in neighbors.iter().flatten() {
            let (nx, ny) = *neighbor;
            if nx < width && ny < height {
                let idx = ny * width + nx;
                if idx < block.coefficients.len() && block.coefficients[idx] != 0 {
                    return 1;
                }
            }
        }
        0
    }
}

