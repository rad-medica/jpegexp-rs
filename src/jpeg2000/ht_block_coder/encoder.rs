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
}

impl HTBlockEncoder {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            mel_encoder: MelEncoder::new(),
            magsgn_encoder: MagSgnEncoder::new(),
            vlc_buffer: Vec::new(),
            vlc_bits: 0,
            vlc_current: 0,
            width,
            height,
            stripe_height: 4,
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
        // 2. For each quad:
        //    a. Encode significance pattern via MEL
        //    b. Encode VLC codeword for significant quads
        //    c. Encode sign/magnitude bits via MagSgn

        for y_stripe in (0..self.height).step_by(self.stripe_height) {
            for x in (0..self.width).step_by(2) {
                self.encode_quad(x, y_stripe, block)?;
            }
        }

        // Finalize encoders
        self.mel_encoder.flush();
        self.magsgn_encoder.flush();

        // Flush VLC buffer
        if self.vlc_bits > 0 {
            self.vlc_current >>= 8 - self.vlc_bits;
            self.vlc_buffer.push(self.vlc_current);
        }

        // Combine streams:
        // MagSgn grows from start, MEL/VLC grows from end
        let mut output = self.magsgn_encoder.get_buffer().to_vec();

        // Append VLC (reversed) and MEL
        let mel_data = self.mel_encoder.get_buffer();
        let vlc_data = &self.vlc_buffer;

        // Interleave MEL and VLC at the end
        for &b in vlc_data.iter().rev() {
            output.push(b);
        }
        for &b in mel_data.iter().rev() {
            output.push(b);
        }

        Ok(output)
    }

    fn encode_quad(&mut self, x: usize, y_base: usize, block: &J2kCodeBlock) -> Result<(), ()> {
        // Determine significance pattern (rho) for the 2x2 quad
        let coords = [
            (x, y_base),
            (x + 1, y_base),
            (x, y_base + 1),
            (x + 1, y_base + 1),
        ];

        let mut rho: u8 = 0;
        let mut quad_coeffs = [0i32; 4];

        for (i, &(px, py)) in coords.iter().enumerate() {
            if px < self.width && py < self.height {
                let idx = py * self.width + px;
                if idx < block.coefficients.len() {
                    let coeff = block.coefficients[idx];
                    quad_coeffs[i] = coeff;
                    if coeff != 0 {
                        rho |= 1 << i;
                    }
                }
            }
        }

        // Encode MEL symbol
        let is_significant = rho != 0;
        self.mel_encoder.encode(is_significant);

        if is_significant {
            // Encode VLC codeword for rho pattern
            let context = self.calculate_context(x, y_base, block);
            let vlc_codeword = vlc::encode_vlc(rho, context);

            // Write VLC bits
            for i in (0..vlc_codeword.bits).rev() {
                self.write_vlc_bit(((vlc_codeword.value >> i) & 1) as u8);
            }

            // Encode MagSgn for each significant coefficient
            for (i, &coeff) in quad_coeffs.iter().enumerate() {
                if (rho >> i) & 1 == 1 {
                    // Write sign bit (0 = positive, 1 = negative)
                    let sign_bit = if coeff < 0 { 1 } else { 0 };
                    self.magsgn_encoder.write_bit(sign_bit);

                    // Write magnitude bits (simplified: just the magnitude value)
                    let mag = coeff.unsigned_abs();
                    if mag > 1 {
                        // Count significant bits
                        let msb = 32 - mag.leading_zeros();
                        for b in (1..msb).rev() {
                            self.magsgn_encoder.write_bit(((mag >> b) & 1) as u8);
                        }
                    }
                }
            }
        }

        Ok(())
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
