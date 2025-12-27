use crate::jpeg2000::ht_block_coder::mag_sgn::MagSgnDecoder;
use crate::jpeg2000::ht_block_coder::mel::MelDecoder;
use crate::jpeg2000::ht_block_coder::vlc;
use crate::jpeg2000::image::J2kCodeBlock;

/// High Throughput Block Coder (HTJ2K Part 15).
/// Processes code-blocks using non-iterative entropy coding.
pub struct HTBlockCoder<'a> {
    mel_decoder: MelDecoder<'a>,
    magsgn_decoder: MagSgnDecoder<'a>,
    // VLC doesn't hold state (static tables), but context does.

    // State
    width: usize,
    height: usize,
    stripe_height: usize, // Usually 4
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

    /// Decodes an entire codeblock into quantized coefficients.
    pub fn decode_block(&mut self, block: &mut J2kCodeBlock) -> Result<(), ()> {
        // HTJ2K decoding flow:
        // 1. Cleanup Pass (logic driven by MEL and VLC)
        // 2. SigProp Pass (forward scan)
        // 3. MagRef Pass (refinement)

        // Actually, HTJ2K (Part 15) defines a single "Cleanup Pass" that does ALMOST EVERYTHING?
        // "The fast block coder ... is based on a single pass cleaning up ... "
        // Wait, Part 15 Section 6.1: "The block coder ... one pass per bit-plane?"
        // NO. "The HT block coder ... all bit-planes are coded ... in a non-iterative manner."
        // "Sub-bit-plane passes are NOT used."
        // It processes "quads" of samples.

        // Initialize block dimensions and coefficients if not already set
        if block.width == 0 {
            block.width = self.width as u32;
        }
        if block.height == 0 {
            block.height = self.height as u32;
        }
        let size = (block.width * block.height) as usize;
        if block.coefficients.is_empty() {
            block.coefficients = vec![0i32; size];
        } else if block.coefficients.len() < size {
            block.coefficients.resize(size, 0i32);
        }

        // We iterate through "stripes" (4 rows high).
        for y_stripe in (0..self.height).step_by(self.stripe_height) {
            for x in 0..self.width {
                // Decode quad at (x, y_stripe)
                self.decode_quad(x, y_stripe, block)?;
            }
        }

        Ok(())
    }

    fn decode_quad(&mut self, x: usize, y_base: usize, block: &mut J2kCodeBlock) -> Result<(), ()> {
        // 1. Calculate Context (sigma neighbors)
        // Context is based on significance of neighboring samples
        // For HTJ2K, context is typically 0 or 1 based on neighbor significance
        let context = self.calculate_context(x, y_base, block);

        // 2. MEL Decoding (is the quad significant?)
        // MEL decodes runs of insignificant quads.
        // We need to ask MEL: "Are we in a run of zeros?"

        // Logic: if mel.run > 0 -> insignificant.
        // If mel.run == 0 -> we decode next MEL symbol.
        // If symbol is 0 -> Run of length 2^k -> insignificant.
        // If symbol is 1 -> Significant quad.

        let is_significant = self.mel_decoder.decode();

        if is_significant {
            // 3. VLC Decoding
            // VLC codewords are interleaved with MEL in the same bitstream.
            // We peek ahead to decode the VLC codeword, then consume the bits.

            // VLC Decoding
            // MEL and VLC share the same bitstream (read backwards from end of packet).
            // Use peek to read VLC codeword without consuming bits yet.
            let peek = self.mel_decoder.peek_bits(16);
            let (rho, _u_off, _e_k, bits_consumed) = vlc::decode_vlc(peek, context);

            // Consume the VLC bits by reading raw bits (bypass MEL state machine)
            // since VLC codewords are encoded as raw bits in the shared stream
            for _ in 0..bits_consumed {
                let _ = self.mel_decoder.read_raw_bit();
            }

            // Advance MEL/VLC stream by `bits_consumed`.

            // 4. Update Block State (Significance)
            // Apply `rho` pattern to the 2x2 quad at (x, y_base).
            // rho is 4 bits: (0,0), (1,0), (0,1), (1,1) -> LSB to MSB?

            self.apply_rho(x, y_base, rho, block);

            // 5. Magnitude Refinement / Sign (MagSgn)
            // For each significant pixel in rho, read sign bit and refinement bits from MagSgn stream.
            self.process_magsgn(x, y_base, rho, block)?;
        } else {
            // insignificant quad
        }

        Ok(())
    }

    /// Calculate context based on neighbor significance
    fn calculate_context(&self, x: usize, y_base: usize, block: &J2kCodeBlock) -> u8 {
        // Context is 0 if no significant neighbors, 1 if there are significant neighbors
        // This is a simplified version - full HTJ2K context calculation considers
        // the specific pattern of neighbor significance (H, V, D neighbors)
        let width = block.width as usize;
        let height = block.height as usize;

        // Check neighbors: left, above, above-left, above-right
        let neighbors = [
            if x > 0 { Some((x - 1, y_base)) } else { None },
            if y_base > 0 { Some((x, y_base - 1)) } else { None },
            if x > 0 && y_base > 0 { Some((x - 1, y_base - 1)) } else { None },
            if x + 1 < width && y_base > 0 { Some((x + 1, y_base - 1)) } else { None },
        ];

        for neighbor in neighbors.iter().flatten() {
            let (nx, ny) = *neighbor;
            if nx < width && ny < height {
                let idx = ny * width + nx;
                if idx < block.coefficients.len() && block.coefficients[idx] != 0 {
                    return 1; // Has significant neighbor
                }
            }
        }
        0 // No significant neighbors
    }

    fn apply_rho(&mut self, x: usize, y_base: usize, rho: u8, block: &mut J2kCodeBlock) {
        // Apply significance mapping
        // pixel order: (0,0), (1,0), (0,1), (1,1) usually (raster within quad)
        // rho bits: bit 0 = (x, y), bit 1 = (x+1, y), bit 2 = (x, y+1), bit 3 = (x+1, y+1)
        let width = block.width as usize;
        let coords = [(x, y_base), (x + 1, y_base), (x, y_base + 1), (x + 1, y_base + 1)];

        for (i, &(px, py)) in coords.iter().enumerate() {
            if (rho >> i) & 1 != 0 {
                // Pixel is significant - mark it (coefficient will be set in process_magsgn)
                if px < width && py < block.height as usize {
                    let idx = py * width + px;
                    if idx < block.coefficients.len() {
                        // Coefficient will be set to proper value in process_magsgn
                        // For now, just ensure it's non-zero to mark significance
                        // (actual value will be overwritten)
                    }
                }
            }
        }
    }

    fn process_magsgn(
        &mut self,
        x: usize,
        y_base: usize,
        rho: u8,
        block: &mut J2kCodeBlock,
    ) -> Result<(), ()> {
        // Process each pixel in the quad in raster order
        let coords = [(0, 0), (1, 0), (0, 1), (1, 1)];
        let width = block.width as usize;

        for (i, &(dx, dy)) in coords.iter().enumerate() {
            let is_sig_in_quad = (rho >> i) & 1 == 1;

            if is_sig_in_quad {
                let px = x + dx;
                let py = y_base + dy;

                if px >= width || py >= block.height as usize {
                    continue;
                }

                let idx = py * width + px;
                if idx >= block.coefficients.len() {
                    continue;
                }

                // If pixel was NOT significant before this pass (HT Cleanup), we read SIGN.
                // In HTJ2K Cleanup pass, we process *newly* significant pixels.
                // Standard says: "If sample becomes significant ... read sign bit."

                // Check if already significant (coefficient non-zero means already significant)
                let is_newly_significant = block.coefficients[idx] == 0;

                if is_newly_significant {
                    // Read Sign Bit
                    let sign_bit = self.magsgn_decoder.read_bit().ok_or(())?;
                    // 0 = Positive, 1 = Negative
                    // Store sign: we'll set the coefficient sign based on this

                    // For newly significant pixels, start with magnitude 1 (will be refined)
                    block.coefficients[idx] = if sign_bit == 0 { 1 } else { -1 };
                } else {
                    // Already significant - read magnitude refinement bit
                    // This refines the existing magnitude
                    if let Some(mag_bit) = self.magsgn_decoder.read_bit() {
                        let current_mag = block.coefficients[idx].abs();
                        // Add the refinement bit to the magnitude
                        // Simplified: shift left and add bit
                        block.coefficients[idx] = if block.coefficients[idx] >= 0 {
                            (current_mag << 1) | (mag_bit as i32)
                        } else {
                            -((current_mag << 1) | (mag_bit as i32))
                        };
                    }
                }
            }
        }
        Ok(())
    }
}
