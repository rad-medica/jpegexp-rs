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
    last_byte_was_ff: bool,
}

impl MelEncoder {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            current_byte: 0,
            bits_in_byte: 0,
            k: 0,
            last_byte_was_ff: false,
        }
    }

    /// Write a single bit
    pub fn write_bit(&mut self, bit: u8) {
        self.current_byte = (self.current_byte << 1) | (bit & 1);
        self.bits_in_byte += 1;

        let limit = if self.last_byte_was_ff { 7 } else { 8 };

        if self.bits_in_byte == limit {
            self.buffer.push(self.current_byte);
            self.last_byte_was_ff = self.current_byte == 0xFF;
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
            // If limit was 7, we shift by (7 - bits). If 8, (8 - bits).
            let limit = if self.last_byte_was_ff { 7 } else { 8 };
            let padding = limit - self.bits_in_byte;
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
    last_byte_was_ff: bool,
}

impl MagSgnEncoder {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            current_byte: 0,
            bits_in_byte: 0,
            last_byte_was_ff: false,
        }
    }

    /// Write a single bit
    pub fn write_bit(&mut self, bit: u8) {
        self.current_byte = (self.current_byte << 1) | (bit & 1);
        self.bits_in_byte += 1;

        let limit = if self.last_byte_was_ff { 7 } else { 8 };

        if self.bits_in_byte == limit {
            self.buffer.push(self.current_byte);
            self.last_byte_was_ff = self.current_byte == 0xFF;
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
            let limit = if self.last_byte_was_ff { 7 } else { 8 };
            let padding = limit - self.bits_in_byte;
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
            width,
            height,
            stripe_height: 4,
            num_quads_x,
            quad_exponents: vec![0; num_quads_x * ((height + 1) / 2)],
        }
    }

    /// Write VLC bits (encoded backwards from end of packet)
    fn write_vlc_bit(&mut self, bit: u8) {
        // VLC bits are part of the MEL/VLC interleaved stream.
        // We write them to the same mel_encoder.
        self.mel_encoder.write_bit(bit);
    }

    /// Encode an entire code-block
    pub fn encode_block(&mut self, block: &J2kCodeBlock) -> Result<Vec<u8>, ()> {
        eprintln!("ENC: encode_block called for block ({},{}) {}x{}", 
                  block.x, block.y, self.width, self.height);
        
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

        // Combine streams:
        // MagSgn grows from start, MEL/VLC grows from end
        let mut output = self.magsgn_encoder.get_buffer().to_vec();

        // Append MEL/VLC (reversed)
        // Since we wrote Quads 0..N, and Decoder reads Quad 0..N from end,
        // we must reverse the byte stream so Quad 0 is at the end.
        let mel_data = self.mel_encoder.get_buffer();

        for &b in mel_data.iter().rev() {
            output.push(b);
        }
        
        eprintln!("ENC: encode_block finished for block ({},{}), output len={}", 
                  block.x, block.y, output.len());
        
        if block.x == 0 && block.y == 0 && self.width == 2 && self.height == 2 {
            eprintln!("ENC: First 2x2 block output (hex): {:02X?}", output);
        }

        Ok(output)
    }

    /// Calculate embedded bits (emb_k and emb_1) from quad coefficients
    /// 
    /// For HTJ2K encoding:
    /// - bit_k: 1 if we want to skip transmitting the MSB in MagSgn (embed it implicitly)
    /// - bit_1: The actual value of the bit at position m (where m = u_val - bit_k)
    /// 
    /// For lossless coding:
    /// - All significant samples have their MSB = 1
    /// - We set bit_k = 1 to skip transmitting this known bit
    /// - bit_1 then represents the MSB value (which is 1 for significant samples)
    /// 
    /// # Parameters
    /// - `rho`: Significance pattern (which samples are non-zero)
    /// - `u_val`: Exponent for the quad (MSB position of largest sample + 1)
    /// - `coeffs`: The 4 coefficient values in the quad
    /// 
    /// # Returns
    /// - `emb_k`: For each significant sample, 1 if MSB should be skipped
    /// - `emb_1`: For each sample, the value of the bit at position m
    fn calculate_emb_bits(&self, rho: u8, u_val: u8, coeffs: &[i32; 4]) -> (u8, u8) {
        let mut emb_k = 0u8;
        let mut emb_1 = 0u8;
        
        for i in 0..4 {
            if (rho >> i) & 1 == 1 {
                let mag = coeffs[i].unsigned_abs();
                
                // For lossless: all significant samples have MSB = 1
                // We can skip transmitting it (bit_k = 1)
                emb_k |= 1 << i;
                
                // bit_1 is the value at position m = u_val - 1 (since bit_k = 1)
                // This is the MSB itself, which is 1 for significant samples
                if u_val > 0 {
                    let bit_at_m = ((mag >> (u_val - 1)) & 1) as u8;
                    emb_1 |= bit_at_m << i;
                }
            }
        }
        
        (emb_k, emb_1)
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
        // Store u0 (reconstructed exponent), NOT e_max_actual0, to match decoder state
        self.quad_exponents[qy0 * self.num_quads_x + qx] = u0;

        // Calculate embedded bits from actual coefficient magnitudes
        let (emb_k0, emb_1_0) = self.calculate_emb_bits(rho0, u0, &quad_coeffs0);

        if qx == 0 && qy0 == 0 {
             eprintln!("ENC Q(0,0): rho={:04b} E_max={} kappa={} u={} u_q={} u_off={} emb_k={:04b} emb_1={:04b}", 
                       rho0, e_max_actual0, kappa0, u0, u_q0, u_off0, emb_k0, emb_1_0);
        }

        // Quad 1 info
        let has_q1 = y_base + 2 < self.height;
        let (rho1, e_max_actual1, quad_coeffs1) = if has_q1 {
            self.get_quad_info(x, y_base + 2, block)
        } else {
            (0, 0, [0i32; 4])
        };
        let (u1, u_q1, u_off1, emb_k1, emb_1_1) = if has_q1 {
            let gamma1 = if rho1.count_ones() > 1 { 1 } else { 0 };
            let kappa1 = self.get_kappa(qx, qy1, gamma1);
            let u1 = e_max_actual1.max(kappa1);
            let u_q1 = u1 - kappa1;
            let u_off1 = if u_q1 > 0 { 1 } else { 0 };
            // Store u1, matching decoder
            self.quad_exponents[qy1 * self.num_quads_x + qx] = u1;
            let (emb_k1, emb_1_1) = self.calculate_emb_bits(rho1, u1, &quad_coeffs1);
            (u1, u_q1, u_off1, emb_k1, emb_1_1)
        } else {
            (0, 0, 0, 0, 0)
        };

        // 1. MEL encoding Quad 0
        let context0 = self.calculate_context(x, y_base, block);
        if context0 == 0 {
            self.mel_encoder.encode(rho0 != 0);
        }

        // 2. VLC encoding Quad 0
        // Pass calculated emb_k and emb_1 to encode_vlc
        let vlc0 = vlc::encode_vlc(rho0, context0, u_off0, emb_k0, emb_1_0);
        
        if qx == 0 && qy0 == 0 {
             eprintln!("ENC Q(0,0): context={} vlc_value={:04X} vlc_bits={} coeffs={:?}", 
                       context0, vlc0.value, vlc0.bits, quad_coeffs0);
        }
        
        for i in (0..vlc0.bits).rev() {
            self.write_vlc_bit(((vlc0.value >> i) & 1) as u8);
        }

        // Context for Quad 1
        let context1 = if rho0 != 0 { 1 } else { 0 };
        
        if has_q1 {
            // 3. MEL encoding Quad 1
            if context1 == 0 {
                self.mel_encoder.encode(rho1 != 0);
            }

            // 4. VLC encoding Quad 1
            let vlc1 = vlc::encode_vlc(rho1, context1, u_off1, emb_k1, emb_1_1);
            for i in (0..vlc1.bits).rev() {
                self.write_vlc_bit(((vlc1.value >> i) & 1) as u8);
            }

            // 5. UVLC encoding
            // u_q encoded is u_q - u_off
            let uvlc = vlc::encode_uvlc(u_q0.saturating_sub(u_off0), u_q1.saturating_sub(u_off1), 0);
            if qx == 0 && qy0 == 0 {
                eprintln!("ENC Q(0,0): UVLC (2quad) uq0={} uq1={} value={:04X} bits={}", 
                          u_q0.saturating_sub(u_off0), u_q1.saturating_sub(u_off1), uvlc.value, uvlc.bits);
            }
            for i in (0..uvlc.bits).rev() {
                self.write_vlc_bit(((uvlc.value >> i) & 1) as u8);
            }

            // 6. MagSgn encoding (forward stream, independent of MEL/VLC order)
            self.emit_quad_magsgn(rho0, u0, emb_k0, &quad_coeffs0);
            self.emit_quad_magsgn(rho1, u1, emb_k1, &quad_coeffs1);
        } else {
            // Only Quad 0 UVLC
            let uvlc = vlc::encode_uvlc(u_q0.saturating_sub(u_off0), 0, 0);
            
            if qx == 0 && qy0 == 0 {
                eprintln!("ENC Q(0,0): UVLC (1quad) uq0={} value={:04X} bits={}", 
                          u_q0.saturating_sub(u_off0), uvlc.value, uvlc.bits);
            }
            
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
        
        // NE neighbor (qx+1, qy-1) availability:
        // Match decoder logic: valid only if qy is even (top of stripe)
        let ne_available = (qy % 2 == 0) && (qx + 1 < self.num_quads_x) && (qy > 0);

        let neighbors = [
            if qx > 0 && qy > 0 { Some((qx - 1, qy - 1)) } else { None }, // NW
            if qy > 0 { Some((qx, qy - 1)) } else { None },             // N
            if ne_available { Some((qx + 1, qy - 1)) } else { None },   // NE
            if qx > 0 { Some((qx - 1, qy)) } else { None },             // W
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

        // Process each sample in the quad
        for (i, &c) in coeffs.iter().enumerate() {
            if (rho >> i) & 1 == 1 {
                let mag = c.unsigned_abs();
                let bit_k = (emb_k >> i) & 1;
                let e_k = u_val.saturating_sub(bit_k);

                // Debug logging
                let coord_x = i % 2;
                let coord_y = i / 2;
                if coord_x == 0 && coord_y == 0 {
                    eprintln!("ENC sample[0,0]: coeff={} mag={} u_val={} bit_k={} e_k={} bits_to_write={}", 
                              c, mag, u_val, bit_k, e_k, e_k);
                }

                // Write Magnitude bits (MSB to LSB)
                if e_k > 0 {
                    for b in (0..e_k).rev() {
                        self.magsgn_encoder.write_bit(((mag >> b) & 1) as u8);
                    }
                }

                // Write Sign bit
                self.magsgn_encoder.write_bit(if c < 0 { 1 } else { 0 });
            }
        }
    }

    fn calculate_context(&self, x: usize, y_base: usize, block: &J2kCodeBlock) -> u8 {
        // Context based on neighbor significance
        // Must check ALL 4 pixels of the neighbor quad to match Decoder's rho!=0 check
        let width = self.width;
        let height = self.height;

        // Neighbor quads: Left (x-2) and Top (y-2)
        // We need to check (nx, ny), (nx+1, ny), (nx, ny+1), (nx+1, ny+1)
        
        let neighbor_origins = [
            if x >= 2 { Some((x - 2, y_base)) } else { None }, // Left Quad Origin
            if y_base >= 2 { Some((x, y_base - 2)) } else { None }, // Top Quad Origin
        ];

        for origin in neighbor_origins.iter().flatten() {
            let (ox, oy) = *origin;
            // Check 2x2 block at ox, oy
            for dy in 0..2 {
                for dx in 0..2 {
                    let nx = ox + dx;
                    let ny = oy + dy;
                    if nx < width && ny < height {
                        let idx = ny * width + nx;
                        if idx < block.coefficients.len() && block.coefficients[idx] != 0 {
                            return 1; // Found significant pixel in neighbor quad
                        }
                    }
                }
            }
        }
        0
    }
}

