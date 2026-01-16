use super::mag_sgn::MagSgnDecoder;
use super::mel::MelDecoder;
use super::vlc::decode_uvlc;
use super::vlc_ohtj2k::{decode_vlc_ohtj2k, calc_next_context, VlcDecoder};
use crate::jpeg2000::image::J2kCodeBlock;
use crate::JpeglsError;

/// HTJ2K Block Decoder (ISO/IEC 15444-15)
/// 
/// The cleanup pass data is structured as:
/// - MagSgn data: forward stream from byte 0
/// - MEL+VLC data: backward (MEL) and forward (VLC) from the same buffer
///   - VLC reads FORWARD from byte 0 of MEL+VLC buffer
///   - MEL reads BACKWARD from the end of MEL+VLC buffer
pub struct HTBlockCoder<'a> {
    mel_decoder: MelDecoder<'a>,
    vlc_decoder: VlcDecoder<'a>,
    magsgn_decoder: MagSgnDecoder<'a>,
    width: usize,
    height: usize,
    stripe_height: usize,
    num_quads_x: usize,
    quad_exponents: Vec<u8>,
    quad_significance: Vec<bool>,
    vlc_context: u16,  // OpenHTJ2K-style context for VLC table
    mel_run: i32,      // Current MEL run counter
}

impl<'a> HTBlockCoder<'a> {
    pub fn new(mel_vlc_data: &'a [u8], magsgn_data: &'a [u8], width: usize, height: usize) -> Self {
        if std::env::var("HTJ2K_DEBUG").is_ok() {
            eprintln!("DEC: Creating decoder for {}x{} block", width, height);
            eprintln!("DEC: MEL/VLC data len={} MagSgn data len={}", mel_vlc_data.len(), magsgn_data.len());
            if mel_vlc_data.len() <= 32 {
                eprintln!("DEC: MEL/VLC data (hex): {:02X?}", mel_vlc_data);
            }
        }

        let num_quads_x = width.div_ceil(2);
        let num_quads_y = height.div_ceil(2);

        let mut coder = Self {
            mel_decoder: MelDecoder::new(mel_vlc_data, 4), // MEL reads forward from start
            vlc_decoder: VlcDecoder::new(mel_vlc_data),     // VLC reads backward from end
            magsgn_decoder: MagSgnDecoder::new(magsgn_data),
            width,
            height,
            stripe_height: 4,
            num_quads_x,
            quad_exponents: vec![0; num_quads_x * num_quads_y],
            quad_significance: vec![false; num_quads_x * num_quads_y],
            vlc_context: 0,  // Start with context 0
            mel_run: 0,
        };

        // Initialize MEL run
        coder.mel_run = coder.mel_decoder.get_run();
        coder
    }

    pub fn decode_block(&mut self, block: &mut J2kCodeBlock) -> Result<(), JpeglsError> {
        let debug = std::env::var("HTJ2K_DEBUG").is_ok();
        
        if debug {
            eprintln!("DEC: decode_block called for block ({},{}) {}x{}", 
                      block.x, block.y, self.width, self.height);
        }
        
        if block.width == 0 {
            block.width = self.width as u32;
        }
        if block.height == 0 {
            block.height = self.height as u32;
        }

        if block.coefficients.len() != (block.width * block.height) as usize {
            block.coefficients = vec![0; (block.width * block.height) as usize];
        }

        if debug {
            eprintln!("DEC: Starting decode loop, num_quads_x={} num_quads_y={}", 
                      self.num_quads_x, self.height.div_ceil(2));
        }

        // Reset VLC context for each block
        self.vlc_context = 0;

        for y_stripe in (0..self.height).step_by(self.stripe_height) {
            for x in (0..self.width).step_by(2) {
                self.decode_quad_pair(x, y_stripe, block)?;
            }
        }

        if debug {
            eprintln!("DEC: decode_block finished successfully for block ({},{})", block.x, block.y);
        }
        Ok(())
    }

    #[allow(unused_assignments)]
    fn decode_quad_pair(
        &mut self,
        x: usize,
        y_base: usize,
        block: &mut J2kCodeBlock,
    ) -> Result<(), JpeglsError> {
        let debug = std::env::var("HTJ2K_DEBUG").is_ok();
        let qx = x / 2;
        let qy0 = y_base / 2;
        let qy1 = qy0 + 1;

        // ===== Decode Quad 0 =====
        let context0 = self.calculate_context(x, y_base, block);
        let mut rho0 = 0u8;
        let mut emb_k0 = 0u8;
        let mut emb_1_0 = 0u8;
        let mut u_off0 = 0u8;

        // Fetch VLC value and decode (OpenHTJ2K: always decode VLC first)
        let mut vlcval = self.vlc_decoder.fetch();
        if debug && qx == 0 && qy0 == 0 {
            eprintln!("DEC Q(0,0): Before VLC decode: vlcval=0x{:08X} vlc_context={} context0={}", vlcval, self.vlc_context, context0);
        }
        let (mut tv0, _, _, _, _, _) = decode_vlc_ohtj2k(vlcval, self.vlc_context);

        // MEL overrides result when context == 0
        let is_sig0 = if context0 == 0 {
            if debug { eprintln!("DEC Q(0,0): MEL check mel_run={}", self.mel_run); }
            self.mel_run -= 2;
            let sig = self.mel_run == -1;
            if self.mel_run < 0 {
                self.mel_run = self.mel_decoder.get_run();
            }
            if debug && qx == 0 && qy0 == 0 {
                eprintln!("DEC Q(0,0): MEL override: mel_run={} sig={} tv0_before=0x{:04X}", self.mel_run, sig, tv0);
            }
            if !sig {
                tv0 = 0; // Override decoded value to 0 if MEL says insignificant
            }
            sig
        } else {
            true
        };

        // Extract values from (possibly overridden) tv0
        // OpenHTJ2K formula: (tv & 0x000F) >> 1 for bits_consumed
        let u_off0_tmp = (tv0 & 1) as u8;
        let bits_consumed = ((tv0 & 0x000F) >> 1) as u8;  // Match OpenHTJ2K exactly
        let rho0_tmp = ((tv0 >> 4) & 0xF) as u8;
        let emb_1_tmp = ((tv0 >> 8) & 0xF) as u8;
        let emb_k_tmp = ((tv0 >> 12) & 0xF) as u8;

        if debug && qx == 0 && qy0 == 0 && is_sig0 {
            eprintln!("DEC Q(0,0): vlc_context={} tv=0x{:04X} rho={:04b} u_off={} emb_k={:04b} emb_1={:04b} bits={}",
                      self.vlc_context, tv0, rho0_tmp, u_off0_tmp, emb_k_tmp, emb_1_tmp, bits_consumed);
        }

        // Advance VLC by bits from (possibly overridden) tv0
        vlcval = self.vlc_decoder.advance(bits_consumed);

        // Store decoded values from (possibly overridden) tv0
        rho0 = rho0_tmp;
        u_off0 = u_off0_tmp;
        emb_k0 = emb_k_tmp;
        emb_1_0 = emb_1_tmp;

        // Update context
        if is_sig0 {
            self.vlc_context = calc_next_context(tv0);
        } else {
            self.vlc_context = 0;
        }

        self.quad_significance[qy0 * self.num_quads_x + qx] = rho0 != 0;

        // ===== Decode Quad 1 =====
        let has_q1 = y_base + 2 < self.height;
        let mut rho1 = 0u8;
        let mut emb_k1 = 0u8;
        let mut emb_1_1 = 0u8;
        let mut u_off1 = 0u8;

        if has_q1 {
            let sigma_n = rho0 != 0;
            let sigma_w = if qx > 0 {
                self.quad_significance[qy1 * self.num_quads_x + (qx - 1)]
            } else {
                false
            };

            let context1 = if sigma_n || sigma_w { 1 } else { 0 };

            // Decode VLC (OpenHTJ2K: always decode VLC first)
            let (mut tv1, _, _, _, _, _) = decode_vlc_ohtj2k(vlcval, self.vlc_context);

            // MEL overrides result when context == 0
            let is_sig1 = if context1 == 0 {
                self.mel_run -= 2;
                let sig = self.mel_run == -1;
                if self.mel_run < 0 {
                    self.mel_run = self.mel_decoder.get_run();
                }
                if !sig {
                    tv1 = 0; // Override decoded value to 0 if MEL says insignificant
                }
                sig
            } else {
                true
            };

            // Extract values from (possibly overridden) tv1
            // OpenHTJ2K formula: (tv & 0x000F) >> 1 for bits_consumed
            let u_off1_tmp = (tv1 & 1) as u8;
            let bits_consumed = ((tv1 & 0x000F) >> 1) as u8;  // Match OpenHTJ2K exactly
            let rho1_tmp = ((tv1 >> 4) & 0xF) as u8;
            let emb_1_tmp = ((tv1 >> 8) & 0xF) as u8;
            let emb_k_tmp = ((tv1 >> 12) & 0xF) as u8;

            // Advance VLC by bits from (possibly overridden) tv1
            vlcval = self.vlc_decoder.advance(bits_consumed);

            // Store decoded values from (possibly overridden) tv1
            rho1 = rho1_tmp;
            u_off1 = u_off1_tmp;
            emb_k1 = emb_k_tmp;
            emb_1_1 = emb_1_tmp;

            // Update context
            if is_sig1 {
                self.vlc_context = calc_next_context(tv1);
            } else {
                self.vlc_context = 0;
            }

            self.quad_significance[qy1 * self.num_quads_x + qx] = rho1 != 0;
        }

        // ===== Decode UVLC for magnitude exponents =====
        let mut u_q0 = 0u8;
        let mut u_q1 = 0u8;

        if is_sig0 || (has_q1 && rho1 != 0) {
            // UVLC decoding: use stored vlcval (not fetch() again - OpenHTJ2K pattern)
            let (uq0_val, uq1_val, bits_uvlc) = decode_uvlc(vlcval as u16, 0);

            u_q0 = uq0_val + u_off0;
            u_q1 = uq1_val + u_off1;

            if debug && qx == 0 && qy0 == 0 {
                eprintln!("DEC Q(0,0): UVLC vlcval=0x{:04X} decoded uq0={} uq1={} bits={}, u_off0={} u_off1={}, final u_q0={} u_q1={}",
                          vlcval as u16, uq0_val, uq1_val, bits_uvlc, u_off0, u_off1, u_q0, u_q1);
            }

            // Advance VLC (don't need the returned value here since this is the last VLC operation)
            let _ = self.vlc_decoder.advance(bits_uvlc);
        }

        // ===== Reconstruct Quad 0 =====
        let kappa0 = self.get_kappa(qx, qy0, if rho0.count_ones() > 1 { 1 } else { 0 });
        let u0 = kappa0 + u_q0;
        self.quad_exponents[qy0 * self.num_quads_x + qx] = u0;
        
        if debug && qx == 0 && qy0 == 0 {
             eprintln!("DEC Q(0,0): rho={:04b} u={} kappa={} u_q={} emb_k={:04b} emb_1={:04b}", 
                       rho0, u0, kappa0, u_q0, emb_k0, emb_1_0);
        }

        self.reconstruct_quad(x, y_base, rho0, u0, emb_k0, emb_1_0, block)?;

        // ===== Reconstruct Quad 1 =====
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
        
        let ne_available = qy.is_multiple_of(2) && (qx + 1 < self.num_quads_x) && (qy > 0);

        let neighbors = [
            if qx > 0 && qy > 0 { Some((qx - 1, qy - 1)) } else { None }, // NW
            if qy > 0 { Some((qx, qy - 1)) } else { None },               // N
            if ne_available { Some((qx + 1, qy - 1)) } else { None },     // NE
            if qx > 0 { Some((qx - 1, qy)) } else { None },               // W
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
        let debug = std::env::var("HTJ2K_DEBUG").is_ok();
        
        if debug && x == 0 && y == 0 {
            eprintln!("DEC reconstruct_quad: x={} y={} rho={:04b} u_val={} emb_k={:04b} emb_1={:04b}",
                      x, y, rho, u_val, emb_k, emb_1);
        }
        
        if rho == 0 { 
            return Ok(()); 
        }

        let w = block.width as usize;
        let h = block.height as usize;
        
        // For each sample in the quad (sample order: 0=TL, 1=TR, 2=BL, 3=BR)
        // ISO 15444-15 Table 2: 0=(0,0), 1=(1,0), 2=(0,1), 3=(1,1)
        for i in 0..4 {
            let sigma = (rho >> i) & 1;
            
            if sigma == 0 {
                continue;
            }
            
            // NOTE: We observed OpenHTJ2K producing rho=2 (bit 1) for pixel (0,0).
            // Standard says bit 0 is (0,0).
            // This suggests scan order swap between 0 and 1.
            // Let's implement standard scan order first, but acknowledge potential issue.
            // Standard:
            // i=0 -> (0,0)
            // i=1 -> (1,0)
            // i=2 -> (0,1)
            // i=3 -> (1,1)
            
            let px = x + (i % 2);
            let py = y + (i / 2);
            
            if px >= w || py >= h {
                continue;
            }
            
            // Calculate m: number of magnitude bits to read
            let bit_k = (emb_k >> i) & 1;
            let m = u_val.saturating_sub(bit_k);

            // Safety check: m should be reasonable for image coefficients (< 31 bits)
            if m >= 31 {
                if debug {
                    eprintln!("WARNING: Unreasonable m={} at ({},{}), u_val={}, bit_k={}",
                              m, px, py, u_val, bit_k);
                }
                continue; // Skip this sample
            }

            // Read m+1 bits (m magnitude bits + 1 sign bit)
            let mut ms_val = 0u32;
            for _ in 0..=m {
                let bit = self.magsgn_decoder.read_bit().ok_or(JpeglsError::InvalidData)?;
                ms_val = (ms_val << 1) | (bit as u32);
            }

            // Extract Sign (LSB) and Magnitude
            let sign_bit = ms_val & 1;
            let sign = if sign_bit == 1 { -1i32 } else { 1i32 };
            let v_n = ms_val >> 1;

            // Add emb_1 bit at position m
            let known_1 = (emb_1 >> i) & 1;
            let v = v_n | ((known_1 as u32) << m);
            
            if debug && px == 0 && py == 0 {
                eprintln!("DEC sample[{},{}]: bit_k={} m={} ms_val={:06b} v_n={} known_1={} v={} sign={}",
                          px, py, bit_k, m, ms_val, v_n, known_1, v, sign);
            }
            
            let mu = v as i32;
            block.coefficients[py * w + px] = mu * sign;
        }
        
        Ok(())
    }

    fn calculate_context(&self, x: usize, y_base: usize, _block: &J2kCodeBlock) -> u8 {
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

