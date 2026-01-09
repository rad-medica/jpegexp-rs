use super::mag_sgn::MagSgnDecoder;
use super::mel::MelDecoder;
use super::vlc;
use crate::jpeg2000::image::J2kCodeBlock;
use crate::JpeglsError;

pub struct HTBlockCoder<'a> {
    mel_decoder: MelDecoder<'a>,
    magsgn_decoder: MagSgnDecoder<'a>,
    width: usize,
    height: usize,
    stripe_height: usize,
    num_quads_x: usize,
    quad_exponents: Vec<u8>,
    quad_significance: Vec<bool>, // Track significance (sigma) of each quad
}

impl<'a> HTBlockCoder<'a> {
    pub fn new(mel_data: &'a [u8], magsgn_data: &'a [u8], width: usize, height: usize) -> Self {
        if width == 2 && height == 2 {
            eprintln!("DEC: Creating decoder for 2x2 block, data len={}", mel_data.len());
            eprintln!("DEC: Input data (hex): {:02X?}", mel_data);
        }
        
        let num_quads_x = (width + 1) / 2;
        let num_quads_y = (height + 1) / 2;
        Self {
            mel_decoder: MelDecoder::new(mel_data),
            magsgn_decoder: MagSgnDecoder::new(magsgn_data),
            width,
            height,
            stripe_height: 4,
            num_quads_x,
            quad_exponents: vec![0; num_quads_x * num_quads_y],
            quad_significance: vec![false; num_quads_x * num_quads_y],
        }
    }

    pub fn decode_block(&mut self, block: &mut J2kCodeBlock) -> Result<(), JpeglsError> {
        eprintln!("DEC: decode_block called for block ({},{}) {}x{}", 
                  block.x, block.y, self.width, self.height);
        
        if block.width == 0 {
            block.width = self.width as u32;
        }
        if block.height == 0 {
            block.height = self.height as u32;
        }

        if block.coefficients.len() != (block.width * block.height) as usize {
            block.coefficients = vec![0; (block.width * block.height) as usize];
        }

        eprintln!("DEC: Starting decode loop, num_quads_x={} num_quads_y={}", 
                  self.num_quads_x, (self.height + 1) / 2);

        for y_stripe in (0..self.height).step_by(self.stripe_height) {
            for x in (0..self.width).step_by(2) {
                self.decode_quad_pair(x, y_stripe, block)?;
            }
        }

        eprintln!("DEC: decode_block finished successfully for block ({},{})", block.x, block.y);
        Ok(())
    }

    fn decode_quad_pair(
        &mut self,
        x: usize,
        y_base: usize,
        block: &mut J2kCodeBlock,
    ) -> Result<(), JpeglsError> {
        let qx = x / 2;
        let qy0 = y_base / 2;
        let qy1 = qy0 + 1;

        // 1. Decode Rho 0
        let context0 = self.calculate_context(x, y_base, block);
        let mut rho0 = 0u8;
        let mut emb_k0 = 0u8;
        let mut emb_1_0 = 0u8;
        let mut u_off0 = 0u8;

        let is_sig0 = if context0 == 0 {
            let mel_bit = self.mel_decoder.decode();
            if qx == 0 && qy0 == 0 && self.width == 2 {
                eprintln!("DEC Q(0,0): MEL bit = {}", mel_bit);
            }
            mel_bit
        } else {
            true
        };

        if is_sig0 {
            let peek = self.mel_decoder.peek_bits(16);
            let (r, uoff, ek, e1, bits) = vlc::decode_vlc(peek, context0);
            
            if qx == 0 && qy0 == 0 {
                eprintln!("DEC Q(0,0): context={} peek={:04X} rho={:04b} u_off={} emb_k={:04b} emb_1={:04b} bits={}",
                          context0, peek, r, uoff, ek, e1, bits);
            }
            
            rho0 = r;
            u_off0 = uoff;
            emb_k0 = ek;
            emb_1_0 = e1;
            for _ in 0..bits {
                self.mel_decoder.read_raw_bit();
            }
        }
        
        // Update significance state for Quad 0
        self.quad_significance[qy0 * self.num_quads_x + qx] = rho0 != 0;

        // 2. Decode Rho 1
        let has_q1 = y_base + 2 < self.height;
        let mut rho1 = 0u8;
        let mut emb_k1 = 0u8;
        let mut emb_1_1 = 0u8;
        let mut u_off1 = 0u8;

        if has_q1 {
            // Context 1 calculation:
            // Needs 'sigma_n' (North neighbor of Q1) -> This is Q0. sigma_n = (rho0 != 0)
            // Needs 'sigma_w' (West neighbor of Q1) -> This is Quad at (qx-1, qy1)
            
            let sigma_n = rho0 != 0;
            let sigma_w = if qx > 0 {
                self.quad_significance[qy1 * self.num_quads_x + (qx - 1)]
            } else {
                false
            };
            
            let context1 = if sigma_n || sigma_w { 1 } else { 0 };
            
            let is_sig1 = if context1 == 0 {
                self.mel_decoder.decode()
            } else {
                true
            };

            if is_sig1 {
                let peek = self.mel_decoder.peek_bits(16);
                let (r, uoff, ek, e1, bits) = vlc::decode_vlc(peek, context1);
                rho1 = r;
                u_off1 = uoff;
                emb_k1 = ek;
                emb_1_1 = e1;
                for _ in 0..bits {
                    self.mel_decoder.read_raw_bit();
                }
            }
            // Update significance state for Quad 1
            self.quad_significance[qy1 * self.num_quads_x + qx] = rho1 != 0;
        }

        // 3. Decode UVLC if needed
        let mut u_q0 = 0u8;
        let mut u_q1 = 0u8;
        if is_sig0 || (has_q1 && rho1 != 0) {
            let peek_uvlc = self.mel_decoder.peek_bits(16);
            let (uq0_val, uq1_val, bits_uvlc) = vlc::decode_uvlc(peek_uvlc, 0);
            u_q0 = uq0_val + u_off0;
            u_q1 = uq1_val + u_off1;
            
            if qx == 0 && qy0 == 0 {
                eprintln!("DEC Q(0,0): UVLC peek={:04X} decoded uq0={} uq1={} bits={}, u_off0={} u_off1={}, final u_q0={} u_q1={}",
                          peek_uvlc, uq0_val, uq1_val, bits_uvlc, u_off0, u_off1, u_q0, u_q1);
            }
            
            for _ in 0..bits_uvlc {
                self.mel_decoder.read_raw_bit();
            }
        }

        // 4. Reconstruction of E_q
        let kappa0 = self.get_kappa(qx, qy0, if rho0.count_ones() > 1 { 1 } else { 0 });
        let u0 = kappa0 + u_q0;
        self.quad_exponents[qy0 * self.num_quads_x + qx] = u0; // Rough E_q estimate
        
        if qx == 0 && qy0 == 0 {
             eprintln!("DEC Q(0,0): rho={:04b} u={} kappa={} u_q={} emb_k={:04b} emb_1={:04b}", rho0, u0, kappa0, u_q0, emb_k0, emb_1_0);
             eprintln!("DEC Q(0,0): Before reconstruct, block[0..4] = {:?}", 
                       &block.coefficients[0..4.min(block.coefficients.len())]);
        }

        self.reconstruct_quad(x, y_base, rho0, u0, emb_k0, emb_1_0, block)?;
        
        if qx == 0 && qy0 == 0 {
             eprintln!("DEC Q(0,0): After reconstruct, block[0..4] = {:?}", 
                       &block.coefficients[0..4.min(block.coefficients.len())]);
        }

        if has_q1 {
            let kappa1 = self.get_kappa(qx, qy1, if rho1.count_ones() > 1 { 1 } else { 0 });
            let u1 = kappa1 + u_q1;
            self.quad_exponents[qy1 * self.num_quads_x + qx] = u1;
            self.reconstruct_quad(x, y_base + 2, rho1, u1, emb_k1, emb_1_1, block)?;
        }

        Ok(())
    }

    fn get_kappa(&self, qx: usize, qy: usize, gamma: u8) -> u8 {
        if gamma == 0 { return 1; }
        let mut max_e = 0u8;
        
        // NE neighbor (qx+1, qy-1) availability:
        // It is available only if it is in a previous stripe.
        // If qy is even (Top of stripe), qy-1 is Bottom of prev stripe (Available).
        // If qy is odd (Bottom of stripe), qy-1 is Top of current stripe (Future for qx+1).
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

    fn reconstruct_quad(
        &mut self,
        x: usize,
        y: usize,
        rho: u8,
        u_val: u8,
        emb_k: u8,
        emb_1: u8,
        block: &mut J2kCodeBlock,
    ) -> Result<(), JpeglsError> {
        if x == 0 && y == 0 {
            eprintln!("DEC reconstruct_quad: x={} y={} rho={:04b} u_val={} emb_k={:04b} emb_1={:04b}",
                      x, y, rho, u_val, emb_k, emb_1);
        }
        
        if rho == 0 { 
            if x == 0 && y == 0 {
                eprintln!("DEC reconstruct_quad: rho==0, returning early");
            }
            return Ok(()); 
        }

        let w = block.width as usize;
        let h = block.height as usize;
        
        // HTJ2K magnitude reconstruction (following OpenHTJ2K ht_cleanup_decode)
        // For each sample in the quad
        for i in 0..4 {
            let sigma = (rho >> i) & 1; // Is this sample significant?
            
            if x == 0 && y == 0 && i == 0 {
                eprintln!("DEC sample loop i={}: sigma={} rho={:04b}", i, sigma, rho);
            }
            
            if sigma == 0 {
                continue; // Sample is zero
            }
            
            let px = x + (i % 2);
            let py = y + (i / 2);
            
            if x == 0 && y == 0 && i == 0 {
                eprintln!("DEC sample[{},{}]: px={} py={} w={} h={}", x, y, px, py, w, h);
            }
            
            if px >= w || py >= h {
                if x == 0 && y == 0 && i == 0 {
                    eprintln!("DEC sample[{},{}]: OUT OF BOUNDS", px, py);
                }
                continue; // Out of bounds
            }
            
            // Calculate m: number of magnitude bits to read from MagSgn stream
            // m = U - bit_k (where U is u_val calculated earlier)
            let bit_k = (emb_k >> i) & 1;
            let m = u_val.saturating_sub(bit_k);
            
            // Read m+1 bits (m magnitude bits + 1 sign bit)
            // MagSgn stream structure: [Magnitude Bits (MSB..LSB)] [Sign Bit]
            // So we read Magnitude bits first, then Sign bit last.
            // Since we shift into ms_val (msb first), the last bit read (Sign) is at LSB.
            
            let mut ms_val = 0u32;
            for _ in 0..=m {  // Read m+1 bits
                let bit = self.magsgn_decoder.read_bit().ok_or(JpeglsError::InvalidData)?;
                ms_val = (ms_val << 1) | (bit as u32);
            }
            
            // Extract Sign (LSB)
            let sign_bit = ms_val & 1;
            let sign = if sign_bit == 1 { -1i32 } else { 1i32 };
            
            // Extract Magnitude (Upper m bits)
            let v_n = ms_val >> 1; // Discard sign bit
            
            // Add emb_1 bit at position m
            // When bit_k = 1, m = u_val - 1, so known_1 is the MSB
            // When bit_k = 0, m = u_val, so known_1 is one bit beyond the MSB (not used for lossless)
            let known_1 = (emb_1 >> i) & 1;
            let v = v_n | ((known_1 as u32) << m);
            
            if px == 0 && py == 0 {
                eprintln!("DEC sample[{},{}]: bit_k={} m={} ms_val={:06b} v_n={} known_1={} v={} sign={}",
                          px, py, bit_k, m, ms_val, v_n, known_1, v, sign);
            }
            
            // Reconstruct magnitude
            // For lossless (reversible), the magnitude is exactly v.
            // The center-bin logic (v |= 1, v += 2) is for lossy quantization.
            // Since we are targeting lossless first, we use v directly.
            // TODO: Pass transformation type to HTBlockCoder to handle lossy reconstruction.
            let mu = v as i32;
            
            // Apply sign
            block.coefficients[py * w + px] = mu * sign;
        }
        
        Ok(())
    }

    fn calculate_context(&self, x: usize, y_base: usize, _block: &J2kCodeBlock) -> u8 {
        // Context is 1 if at least one of the two previously decoded quads is significant.
        // Neighbors: Left (x-2, y) and Top (x, y-2)
        // In Quad coords: (qx-1, qy) and (qx, qy-1)
        
        let qx = x / 2;
        let qy = y_base / 2;
        
        let mut context = 0;
        
        // Check Left Neighbor (qx-1, qy)
        if qx > 0 {
            // Need to verify index bounds to avoid panic
            let idx = qy * self.num_quads_x + (qx - 1);
            if idx < self.quad_significance.len() && self.quad_significance[idx] {
                context |= 1;
            }
        }
        
        // Check Top Neighbor (qx, qy-1)
        // Since we process in stripes of 4, qy increments by 2 per stripe loop.
        // But inside stripe we have qy (Top) and qy+1 (Bottom).
        // If we are at qy (Top Quad), neighbor is qy-1 (Previous Stripe Bottom).
        // If we are at qy+1 (Bottom Quad), neighbor is qy (Current Stripe Top).
        
        // For Quad 0 (qy)
        // y_base passed is the top row of the quad pair.
        // If qy > 0, we check (qy-1).
        
        if qy > 0 {
             let idx = (qy - 1) * self.num_quads_x + qx;
             if idx < self.quad_significance.len() && self.quad_significance[idx] {
                context |= 1;
            }
        }
        
        context
    }
}
