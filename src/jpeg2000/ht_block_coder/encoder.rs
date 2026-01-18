//! HTJ2K (High-Throughput JPEG 2000) Block Encoder
//! Implements encoding for ISO/IEC 15444-15

use super::vlc;
use crate::jpeg2000::image::J2kCodeBlock;
use crate::JpeglsError;

/// MEL (Magnitude Exponent Logic) encoder
/// Encodes run-lengths of insignificant quads
pub struct MelEncoder {
    buffer: Vec<u8>,
    current_byte: u8,
    bits_in_byte: u8,
    k: i32, // State index (exponent)
    last_byte_was_ff: bool,
    run: u32,
    t: u32, // Threshold (2^E)
}

impl MelEncoder {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            current_byte: 0,
            bits_in_byte: 0,
            k: 0,
            last_byte_was_ff: false,
            run: 0,
            t: 1, // MEL_E[0] = 0 -> 2^0 = 1
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
    pub fn encode(&mut self, is_significant: bool) {
        if std::env::var("HTJ2K_DEBUG").is_ok() {
            eprintln!("MelEncoder: encode({}) run={} t={} k={}", is_significant, self.run, self.t, self.k);
        }
        let mel_e = [0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 4, 5];
        
        if !is_significant {
            // Insignificant (0)
            self.run += 1;
            if self.run >= self.t {
                self.write_bit(1);
                self.run = 0;
                self.k = (self.k + 1).min(12);
                let eval = mel_e[self.k as usize];
                self.t = 1 << eval;
            }
        } else {
            // Significant (1)
            self.write_bit(0);
            let mut eval = mel_e[self.k as usize];
            
            while eval > 0 {
                eval -= 1;
                self.write_bit(((self.run >> eval) & 1) as u8);
            }
            
            self.run = 0;
            self.k = (self.k - 1).max(0);
            let eval = mel_e[self.k as usize];
            self.t = 1 << eval;
        }
    }

    /// Flush remaining bits to buffer
    pub fn flush(&mut self) {
        // Terminate MEL run if any
        if self.run > 0 {
            self.write_bit(1);
        }
    
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

/// VLC (Variable Length Coding) encoder
/// Encodes VLC codewords for quad significance and patterns
pub struct VlcEncoder {
    buffer: Vec<u8>, // Stores bytes in forward order (to be reversed later)
    current_byte: u8,
    bits_in_byte: u8,
    last_byte_was_ff: bool, // Not strictly needed for VLC in same way, but good for padding check
}

impl VlcEncoder {
    pub fn new() -> Self {
        // Matches OpenHTJ2K initialization:
        // tmp(0xF), bits(4), last(0xFF)
        // This ensures the first bits written (which are LSB of VLC codewords)
        // go into the High Nibble of the first byte (which is the last byte of the stream).
        Self {
            buffer: Vec::new(),
            current_byte: 0x0F,
            bits_in_byte: 4,
            last_byte_was_ff: true,
        }
    }

    /// Write multiple bits (LSB first packing, LSB first consumption)
    /// Matches OpenHTJ2K's state_VLC_enc::emitVLCBits
    pub fn write_bits(&mut self, mut value: u32, mut count: u8) {
        while count > 0 {
            // Available bits in current byte
            // Note: OpenHTJ2K logic: available = 8 - (last > 0x8F) - bits
            // But 'last' refers to the *previous* byte written to buffer.
            // Here we are filling 'current_byte'.
            
            // We need to know if the *previously written* byte was > 0x8F to determine if we have 7 or 8 bits.
            // In our forward-accumulation-for-reverse scheme:
            // The byte we are filling NOW will eventually be written "before" the previous one in the stream?
            // No, VLC writes backwards.
            // OpenHTJ2K: buf[pos] = tmp; pos--;
            // So the byte written AT pos is physically after byte at pos-1.
            // But decoding order is pos, pos-1...
            
            // Let's stick to OpenHTJ2K logic:
            // "last" is the byte at pos+1 (the one just written).
            // We are writing at pos.
            // So "last" is the byte that will be read *before* the current byte by the decoder?
            // Decoder reads: [Last Byte in File] ... [First Byte in File]
            // VLC stream is read Backwards.
            // So "last" in OpenHTJ2K is the byte that was written previously, which corresponds to...
            // Let's look at OpenHTJ2K again.
            // buf[pos] = tmp; pos--; last = tmp;
            // It writes from High Address to Low Address.
            // Decoder reads from High Address to Low Address?
            // No, Decoder reads bytes and consumes them.
            
            // Let's simulate:
            // Write Byte A at 100. last = A.
            // Write Byte B at 99. Check last (A). If A > 0x8F, B can only have 7 bits.
            // This is bit stuffing!
            
            // So if I store bytes in a Vec in the order they are generated (A, B, C...)
            // Then I reverse them at the end -> (... C, B, A).
            // Then Byte A is at the END of the stream.
            // Byte B is before A.
            // So B checks A.
            // So current byte checks *previous byte in this vector*?
            // Yes!
            
            let limit = if self.last_byte_was_ff { 7 } else { 8 };
            let available = limit - self.bits_in_byte;
            
            let t = available.min(count);
            
            // Pack bits: LSB of value goes to next available bit in current_byte (from LSB)
            // current_byte fills 0..7
            self.current_byte |= ((value & ((1 << t) - 1)) as u8) << self.bits_in_byte;
            
            self.bits_in_byte += t;
            value >>= t;
            count -= t;
            
            if self.bits_in_byte == limit {
                // Byte full (at limit)
                
                // OpenHTJ2K Logic:
                // if ((last > 0x8f) && tmp != 0x7F) { last = 0x00; continue; }
                // This allows using the 8th bit (bit 7) if the first 7 bits (0-6) don't form 0x7F.
                // Because if they are not 0x7F, then even if bit 7 becomes 1 (making it > 0x7F?), 
                // Wait.
                // If tmp != 0x7F (e.g. 0x3F), bit 7 is 0.
                // If we add another bit at bit 7.
                // If that bit is 0 -> 0x3F. (Safe after FF).
                // If that bit is 1 -> 0xBF. (Unsafe after FF? FF BF is Reserved).
                
                // OpenHTJ2K logic effectively extends the limit to 8 if tmp != 0x7F.
                // But wait, if we extend to 8, we might write a 1 in bit 7.
                // Then byte becomes > 0x7F.
                // If last was FF, then FF 8x is a marker.
                
                // Let's look at OpenHTJ2K again.
                // if ((last > 0x8f) && tmp != 0x7F) { last = 0x00; continue; }
                // It resets `last` to 0.
                // Then `available` recalculates: 8 - (0 > 0x8F) - 7 = 1.
                // So it allows writing ONE more bit.
                
                // If that next bit is 1:
                // tmp (was say 0x00) becomes 0x80.
                // Then next loop:
                // last was 0x00.
                // Write buf = 0x80.
                // last = 0x80.
                
                // But real last was 0xFF!
                // So we wrote FF 80.
                // FF 80 is NOT a marker (markers are FF90..FFFF).
                // So this is safe.
                
                // What if tmp was 0x7F?
                // Then bit 7 is 0.
                // If we allow 8th bit, and it is 1 -> 0xFF.
                // FF FF.
                // This requires next byte stuffing.
                
                // What if tmp was 0x7F and we DON'T continue?
                // We write 0x7F.
                // FF 7F. Safe.
                
                // The check `tmp != 0x7F` prevents extending if we are at 0x7F.
                // Because 0x7F + 1-bit could become 0xFF.
                // Any other value < 0x7F, plus 1-bit (at pos 7), results in value < 0xFF.
                // e.g. 0x00 + 1<<7 = 0x80.
                // 0x7E + 1<<7 = 0xFE.
                // 0xFE is safe after FF? FF FE is safe.
                
                // So the logic is: "If we are limited to 7 bits, BUT the 7 bits we have are NOT 0x7F, we can safely use the 8th bit."
                
                if self.last_byte_was_ff && self.current_byte != 0x7F {
                    // Pretend last byte wasn't FF, to allow filling 8th bit
                    self.last_byte_was_ff = false;
                    continue;
                }
                
                self.buffer.push(self.current_byte);
                
                // Check if THIS byte is > 0x8F, affecting the NEXT one.
                self.last_byte_was_ff = self.current_byte > 0x8F;
                
                self.current_byte = 0;
                self.bits_in_byte = 0;
            }
        }
    }
    
    pub fn get_buffer(&self) -> &[u8] {
        &self.buffer
    }
}

impl Default for VlcEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// High Throughput Block Encoder (HTJ2K Part 15)
/// Encodes code-blocks using non-iterative entropy coding
pub struct HTBlockEncoder {
    mel_encoder: MelEncoder,
    vlc_encoder: VlcEncoder,
    magsgn_encoder: MagSgnEncoder,
    width: usize,
    height: usize,
    stripe_height: usize,
    num_quads_x: usize,
    quad_exponents: Vec<u8>, // Max exponent E_q for each quad
    quad_significance: Vec<bool>, // Track significance (rho != 0) of each quad
}

impl HTBlockEncoder {
    pub fn new(width: usize, height: usize) -> Self {
        let num_quads_x = width.div_ceil(2);
        let num_quads_y = height.div_ceil(2);
        Self {
            mel_encoder: MelEncoder::new(),
            vlc_encoder: VlcEncoder::new(),
            magsgn_encoder: MagSgnEncoder::new(),
            width,
            height,
            stripe_height: 4,
            num_quads_x,
            quad_exponents: vec![0; num_quads_x * num_quads_y],
            quad_significance: vec![false; num_quads_x * num_quads_y],
        }
    }

    /// Write VLC bits (encoded backwards from end of packet)
    fn write_vlc_bits(&mut self, value: u32, count: u8) {
        // VLC bits are written to the separate VLC encoder
        self.vlc_encoder.write_bits(value, count);
    }

    /// Encode an entire code-block
    pub fn encode_block(&mut self, block: &J2kCodeBlock) -> Result<Vec<u8>, JpeglsError> {
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
        // VLC doesn't need explicit flush, the last partial byte is handled in termination
        
        // Combine streams:
        // MagSgn grows from start, MEL/VLC grows from end
        let mut output = self.magsgn_encoder.get_buffer().to_vec();
        let magsgn_len = output.len();

        // Terminate and merge MEL and VLC
        // This implements termMELandVLC from OpenHTJ2K
        let mel_buf = self.mel_encoder.buffer.clone(); // Copy for manipulation
        let vlc_buf = self.vlc_encoder.buffer.clone(); 
        
        // OpenHTJ2K Termination Logic:
        // MEL is at buffer start (forward), VLC is at buffer end (backward).
        // They might share the "fuse" byte in the middle.
        
        // In our struct:
        // mel_encoder.current_byte contains the partial MEL byte (MSB aligned).
        // vlc_encoder.current_byte contains the partial VLC byte (LSB aligned).
        
        let mut final_mel = mel_buf;
        let mut final_vlc = vlc_buf; // These are in Forward generation order
        
        let mel_rem = if self.mel_encoder.bits_in_byte == 0 { 8 } else { 8 - self.mel_encoder.bits_in_byte }; // Bits remaining in MEL byte
        let vlc_bits = self.vlc_encoder.bits_in_byte; // Valid bits in VLC byte
        
        // Check if we can fuse
        // MEL fills from MSB. Mask covers unused LSBs.
        // VLC fills from LSB. Mask covers unused MSBs.
        
        // MEL.tmp is ALREADY shifted left by flush() in MelEncoder?
        // No, MelEncoder::flush() pushes the byte. 
        // But here we want the UNFLUSHED partial state.
        // I called flush() above, which pushed the partial byte.
        // Let's UNDO that push for logic correctness or adjust.
        
        // If I called flush(), mel_encoder pushed the padded byte.
        // I should probably pop it to treat it as partial.
        
        let mut mel_partial = 0u8;
        let mut mel_has_partial = false;
        
        if self.mel_encoder.bits_in_byte > 0 {
             // It was flushed. Pop it.
             if let Some(b) = final_mel.pop() {
                 mel_partial = b; // This is already padded/shifted
                 mel_has_partial = true;
             }
        }
        
        // vlc_partial comes from current_byte.
        // If vlc_bits == 0, current_byte might be 0 or 0xF (init).
        // We only use it if vlc_bits > 0.
        let vlc_partial = self.vlc_encoder.current_byte;
        let vlc_has_partial = vlc_bits > 0;
        
        // Logic:
        // MEL_mask = (0xFF << MEL.rem) & 0xFF; // The bits MEL used
        // VLC_mask = 0xFF >> (8 - VLC.bits); // The bits VLC used
        
        // mel_partial has 1s in MEL_mask position (data) and 0s elsewhere.
        // vlc_partial has 1s in VLC_mask position (data) and 0s elsewhere.
        
        let mel_mask = if mel_has_partial { 0xFFu8 << mel_rem } else { 0 };
        let vlc_mask = if vlc_has_partial { 0xFFu8 >> (8 - vlc_bits) } else { 0 };
        
        if (mel_mask | vlc_mask) != 0 {
             let fuse = mel_partial | vlc_partial;
             
             // Check for conflict
             // (((fuse ^ MEL) & MEL_mask) | ((fuse ^ VLC) & VLC_mask)) == 0
             // means fuse bits match original bits in valid regions
             let match_mel = ((fuse ^ mel_partial) & mel_mask) == 0;
             let match_vlc = ((fuse ^ vlc_partial) & vlc_mask) == 0;
             
             if match_mel && match_vlc && fuse != 0xFF {
                 // FUSE SUCCESS
                 final_mel.push(fuse);
             } else {
                 // NO FUSE
                 if mel_has_partial { final_mel.push(mel_partial); }
                 if vlc_has_partial { final_vlc.push(vlc_partial); }
             }

        }
        
        // Append VLC (reversed) to MEL
        for &b in final_vlc.iter().rev() {
            final_mel.push(b);
        }
        
        // Append Suffix Length Indicator (Scup) if prefix is not empty
        if magsgn_len > 0 {
            // Scup is the length of the suffix (MEL + VLC)
            let scup = final_mel.len() as u32;
            let mut val = scup;
            
            // First 7 bits (Last byte in stream) - MSB 0
            let last_byte = (val & 0x7F) as u8;
            val >>= 7;
            
            let mut scup_bytes = Vec::new();
            
            // Higher bits (Earlier bytes) - MSB 1
            while val > 0 {
                scup_bytes.push(((val & 0x7F) | 0x80) as u8);
                val >>= 7;
            }
            // Push higher bits first (reverse of generation)
            scup_bytes.reverse();
            scup_bytes.push(last_byte);
            
            // Append suffix then Scup
            output.extend_from_slice(&final_mel);
            output.extend_from_slice(&scup_bytes);
        } else {
            // If prefix is empty, no Scup needed (Lcup = Scup)
            output.extend_from_slice(&final_mel);
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
        
        for (i, &coeff) in coeffs.iter().enumerate().take(4) {
            if (rho >> i) & 1 == 1 {
                let mag = coeff.unsigned_abs();
                
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

    fn encode_quad_pair(&mut self, x: usize, y_base: usize, block: &J2kCodeBlock) -> Result<(), JpeglsError> {
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
        
        // Write VLC bits (LSB-first packing handled by VlcEncoder)
        self.write_vlc_bits(vlc0.value, vlc0.bits);
        
        // Update significance state for Quad 0
        self.quad_significance[qy0 * self.num_quads_x + qx] = rho0 != 0;

        // Context for Quad 1
        // sigma_n: North neighbor of Q1 -> This is Q0
        let sigma_n = rho0 != 0;
        // sigma_w: West neighbor of Q1 -> Quad at (qx-1, qy1)
        let sigma_w = if qx > 0 {
            self.quad_significance[qy1 * self.num_quads_x + (qx - 1)]
        } else {
            false
        };
        let context1 = if sigma_n || sigma_w { 1 } else { 0 };
        
        if has_q1 {
            // 3. MEL encoding Quad 1
            if context1 == 0 {
                self.mel_encoder.encode(rho1 != 0);
            }

            // 4. VLC encoding Quad 1
            let vlc1 = vlc::encode_vlc(rho1, context1, u_off1, emb_k1, emb_1_1);
            self.write_vlc_bits(vlc1.value, vlc1.bits);
            
            // Update significance state for Quad 1
            self.quad_significance[qy1 * self.num_quads_x + qx] = rho1 != 0;

            // 5. UVLC encoding
            // u_q encoded is u_q - u_off
            let uvlc = vlc::encode_uvlc(u_q0.saturating_sub(u_off0), u_q1.saturating_sub(u_off1), 0);
            if qx == 0 && qy0 == 0 {
                eprintln!("ENC Q(0,0): UVLC (2quad) uq0={} uq1={} value={:04X} bits={}", 
                          u_q0.saturating_sub(u_off0), u_q1.saturating_sub(u_off1), uvlc.value, uvlc.bits);
            }
            self.write_vlc_bits(uvlc.value, uvlc.bits);

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
            
            self.write_vlc_bits(uvlc.value, uvlc.bits);
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
        let ne_available = qy.is_multiple_of(2) && (qx + 1 < self.num_quads_x) && (qy > 0);

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

    fn calculate_context(&self, x: usize, y_base: usize, _block: &J2kCodeBlock) -> u8 {
        // Context is 1 if at least one of the two previously encoded quads is significant.
        // Neighbors: Left (x-2, y) and Top (x, y-2)
        // In Quad coords: (qx-1, qy) and (qx, qy-1)
        
        let qx = x / 2;
        let qy = y_base / 2;
        
        let mut context = 0;
        
        // Check Left Neighbor (qx-1, qy)
        if qx > 0 {
            let idx = qy * self.num_quads_x + (qx - 1);
            if idx < self.quad_significance.len() && self.quad_significance[idx] {
                context |= 1;
            }
        }
        
        // Check Top Neighbor (qx, qy-1)
        if qy > 0 {
            let idx = (qy - 1) * self.num_quads_x + qx;
            if idx < self.quad_significance.len() && self.quad_significance[idx] {
                context |= 1;
            }
        }
        
        context
    }
}

