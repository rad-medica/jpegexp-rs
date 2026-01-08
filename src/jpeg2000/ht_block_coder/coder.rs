use crate::JpeglsError;
use super::mel::MelDecoder;
use super::vlc;
use super::mag_sgn::MagSgnDecoder;
use crate::jpeg2000::image::J2kCodeBlock;

pub struct HTBlockCoder<'a> {
    mel_decoder: MelDecoder<'a>,
    magsgn_decoder: MagSgnDecoder<'a>,
    width: usize,
    height: usize,
    stripe_height: usize,
}

impl<'a> HTBlockCoder<'a> {
    pub fn new(mel_data: &'a [u8], magsgn_data: &'a [u8], width: usize, height: usize) -> Self {
        Self {
            mel_decoder: MelDecoder::new(mel_data),
            magsgn_decoder: MagSgnDecoder::new(magsgn_data),
            width,
            height,
            stripe_height: 4,
        }
    }

    pub fn decode_block(&mut self, block: &mut J2kCodeBlock) -> Result<(), JpeglsError> {
        // HTJ2K block decoding process (ISO 15444-15)
        // 1. Initialize decoders (MEL, VLC, MagSgn) - done in new()
        // 2. Decode passes: Cleanup (HT Clean)
        // Note: HTJ2K replaces the standard 3 passes with a single "HT Clean" pass
        // that handles everything via MEL/VLC/MagSgn.
        
        if block.width == 0 { block.width = self.width as u32; }
        if block.height == 0 { block.height = self.height as u32; }
        
        // Initialize coefficients
        if block.coefficients.len() != (block.width * block.height) as usize {
            block.coefficients = vec![0; (block.width * block.height) as usize];
        }

        // We iterate through "stripes" (4 rows high).
        // Each stripe contains 2 rows of 2x2 quads.
        for y_stripe in (0..self.height).step_by(self.stripe_height) {
            // Iterate quad rows within the stripe (0 and 2 offset)
            for qy_offset in (0..self.stripe_height).step_by(2) {
                let y = y_stripe + qy_offset;
                if y >= self.height {
                    break;
                }
                
                // Iterate quads horizontally
                for x in (0..self.width).step_by(2) {
                    // Decode quad at (x, y)
                    self.decode_quad(x, y, block)?;
                }
            }
        }

        Ok(())
    }

    fn decode_quad(&mut self, x: usize, y_base: usize, block: &mut J2kCodeBlock) -> Result<(), JpeglsError> {
        // Calculate context for MEL
        // Context depends on significance of neighbors (sigma)
        let context = self.calculate_context(x, y_base, block);

        // 2. MEL Decoding
        // Decode significance of the quad (is_significant)
        // If symbol is 0 -> Run of length 2^k -> insignificant.
        // If symbol is 1 -> Significant quad.

        let is_significant = self.mel_decoder.decode();
        
        if std::env::var("HTJ2K_DEBUG").is_ok() {
            eprintln!("[HTJ2K] Quad ({:2},{:2}): ctx={}, is_sig={}", 
                     x, y_base, context, is_significant);
        }

        if is_significant {
            // 3. VLC Decoding
            let peek = self.mel_decoder.peek_bits(16);
            let (rho, _u_off, _e_k, bits_consumed) = vlc::decode_vlc(peek, context);
            
            if std::env::var("HTJ2K_DEBUG").is_ok() {
                eprintln!("       VLC: peek={:016b}, rho={:04b}, bits={}", 
                         peek, rho, bits_consumed);
            }

            // Consume the VLC bits
            for _ in 0..bits_consumed {
                let _ = self.mel_decoder.read_raw_bit();
            }

            self.apply_rho(x, y_base, rho, block);

            self.process_magsgn(x, y_base, rho, block)?;
        } else {
            // insignificant quad
        }

        Ok(())
    }

    fn calculate_context(&self, _x: usize, _y: usize, _block: &J2kCodeBlock) -> u8 {
        // TODO: Implement actual context calculation based on neighbors.
        // For now return 0 (initial context).
        0
    }

    fn apply_rho(&self, x: usize, y: usize, rho: u8, block: &mut J2kCodeBlock) {
        // rho is 4 bits: (0,0), (1,0), (0,1), (1,1)
        // LSB corresponds to first in scan
        
        let w = block.width as usize;
        let h = block.height as usize;
        let coeffs = &mut block.coefficients;

        // (0,0)
        if (rho & 1) != 0 && x < w && y < h {
            coeffs[y * w + x] = 1; // Mark as significant (placeholder value)
        }
        // (1,0)
        if (rho & 2) != 0 && (x + 1) < w && y < h {
            coeffs[y * w + (x + 1)] = 1;
        }
        // (0,1)
        if (rho & 4) != 0 && x < w && (y + 1) < h {
            coeffs[(y + 1) * w + x] = 1;
        }
        // (1,1)
        if (rho & 8) != 0 && (x + 1) < w && (y + 1) < h {
            coeffs[(y + 1) * w + (x + 1)] = 1;
        }
    }

    fn process_magsgn(&mut self, x: usize, y: usize, rho: u8, block: &mut J2kCodeBlock) -> Result<(), JpeglsError> {
        let w = block.width as usize;
        let h = block.height as usize;
        let coeffs = &mut block.coefficients;

        // Iterate pixels in quad
        for qy in 0..2 {
            for qx in 0..2 {
                let bit_idx = qy * 2 + qx;
                if (rho & (1 << bit_idx)) != 0 {
                    let px = x + qx;
                    let py = y + qy;
                    
                    if px < w && py < h {
                        // Read Sign
                        let sign = self.magsgn_decoder.read_bit().ok_or(JpeglsError::InvalidData)?;
                        
                        // Read Magnitude Refinement
                        // Placeholder: value is 1 (from apply_rho)
                        let mut val = 1;
                        
                        // If we need to read more bits (refinement), we do it here.
                        // For now, apply sign.
                        
                        if sign == 1 {
                            val = -val;
                        }
                        
                        coeffs[py * w + px] = val;
                    }
                }
            }
        }
        Ok(())
    }
}
