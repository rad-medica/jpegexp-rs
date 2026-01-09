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
}

impl<'a> HTBlockCoder<'a> {
    pub fn new(mel_data: &'a [u8], magsgn_data: &'a [u8], width: usize, height: usize) -> Self {
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
        }
    }

    pub fn decode_block(&mut self, block: &mut J2kCodeBlock) -> Result<(), JpeglsError> {
        if block.width == 0 {
            block.width = self.width as u32;
        }
        if block.height == 0 {
            block.height = self.height as u32;
        }

        if block.coefficients.len() != (block.width * block.height) as usize {
            block.coefficients = vec![0; (block.width * block.height) as usize];
        }

        for y_stripe in (0..self.height).step_by(self.stripe_height) {
            for x in (0..self.width).step_by(2) {
                self.decode_quad_pair(x, y_stripe, block)?;
            }
        }

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

        let is_sig0 = if context0 == 0 {
            self.mel_decoder.decode()
        } else {
            true // VLC will tell us if it's actually 0
        };

        if is_sig0 {
            let peek = self.mel_decoder.peek_bits(16);
            let (r, _uoff, ek, e1, bits) = vlc::decode_vlc(peek, context0);
            rho0 = r;
            emb_k0 = ek;
            emb_1_0 = e1;
            for _ in 0..bits {
                self.mel_decoder.read_raw_bit();
            }
        }

        // 2. Decode Rho 1
        let has_q1 = y_base + 2 < self.height;
        let mut rho1 = 0u8;
        let mut emb_k1 = 0u8;
        let mut emb_1_1 = 0u8;

        if has_q1 {
            let context1 = (rho0 >> 1) | (rho0 & 1);
            let is_sig1 = if context1 == 0 {
                self.mel_decoder.decode()
            } else {
                true
            };

            if is_sig1 {
                let peek = self.mel_decoder.peek_bits(16);
                let (r, _uoff, ek, e1, bits) = vlc::decode_vlc(peek, context1);
                rho1 = r;
                emb_k1 = ek;
                emb_1_1 = e1;
                for _ in 0..bits {
                    self.mel_decoder.read_raw_bit();
                }
            }
        }

        // 3. Decode UVLC if needed
        let mut u_q0 = 0u8;
        let mut u_q1 = 0u8;
        if is_sig0 || (has_q1 && rho1 != 0) {
            let peek_uvlc = self.mel_decoder.peek_bits(16);
            let (uq0, uq1, bits_uvlc) = vlc::decode_uvlc(peek_uvlc, 0);
            u_q0 = uq0;
            u_q1 = uq1;
            for _ in 0..bits_uvlc {
                self.mel_decoder.read_raw_bit();
            }
        }

        // 4. Reconstruction of E_q
        let kappa0 = self.get_kappa(qx, qy0, if rho0.count_ones() > 1 { 1 } else { 0 });
        let u0 = kappa0 + u_q0;
        self.quad_exponents[qy0 * self.num_quads_x + qx] = u0; // Rough E_q estimate
        
        self.reconstruct_quad(x, y_base, rho0, u0, emb_k0, emb_1_0, block)?;

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
        if rho == 0 { return Ok(()); }

        let w = block.width as usize;
        let h = block.height as usize;
        
        // HTJ2K magnitude reconstruction (following OpenHTJ2K ht_cleanup_decode)
        // For each sample in the quad
        for i in 0..4 {
            let sigma = (rho >> i) & 1; // Is this sample significant?
            if sigma == 0 {
                continue; // Sample is zero
            }
            
            let px = x + (i % 2);
            let py = y + (i / 2);
            if px >= w || py >= h {
                continue; // Out of bounds
            }
            
            // Calculate m: number of magnitude bits to read from MagSgn stream
            // m = U - bit_k (where U is u_val calculated earlier)
            let bit_k = (emb_k >> i) & 1;
            let m = u_val.saturating_sub(bit_k);
            
            // Read m bits from MagSgn stream
            // Format: [sign_bit, mag_bit_(m-1), ..., mag_bit_0]
            // Total bits = m + 1 (m magnitude bits + 1 sign bit)
            let mut ms_val = 0u32;
            for _ in 0..=m {  // Read m+1 bits (1 sign + m magnitude)
                let bit = self.magsgn_decoder.read_bit().ok_or(JpeglsError::InvalidData)?;
                ms_val = (ms_val << 1) | (bit as u32);
            }
            
            // Extract sign (MSB)
            let sign_bit = (ms_val >> m) & 1;
            let sign = if sign_bit == 1 { -1i32 } else { 1i32 };
            
            // Extract magnitude (lower m bits)
            let v_n = if m > 0 {
                ms_val & ((1 << m) - 1)
            } else {
                0
            };
            
            // Add emb_1 bit as MSB of v
            let known_1 = (emb_1 >> i) & 1;
            let mut v = v_n | ((known_1 as u32) << m);
            
            // Reconstruct magnitude (OpenHTJ2K formula for lossless)
            // v_n |= 1 means add "center of bin"
            // Then (v_n + 2) and shift by (pLSB - 1)
            // For lossless (pLSB = 0), shift = -1, which we skip
            let mu = if m != 0 {
                v |= 1;  // Add center of bin
                (v + 2) as i32
            } else {
                v as i32
            };
            
            // Apply sign
            block.coefficients[py * w + px] = mu * sign;
        }
        
        Ok(())
    }

    fn calculate_context(&self, x: usize, y_base: usize, block: &J2kCodeBlock) -> u8 {
        let width = self.width;
        let height = self.height;
        let neighbors = [
            if x >= 2 { Some((x - 2, y_base)) } else { None },
            if y_base >= 2 { Some((x, y_base - 2)) } else { None },
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
