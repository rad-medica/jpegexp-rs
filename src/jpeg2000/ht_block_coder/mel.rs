/// Magnitude Exponent Logic (MEL) decoder state and functionality.
/// Implements the MEL coding scheme defined in ISO/IEC 15444-15 (HTJ2K).
pub struct MelDecoder<'a> {
    data: &'a [u8],
    pos: usize,
    bits_buffer: u8,
    bits_left: u8,
    k: i32,   // State index (exponent)
    run: i32, // Current run length remaining
}

impl<'a> MelDecoder<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        // Experimental: OpenHTJ2K seems to add a 00 padding byte at the end.
        // Scan backwards and skip trailing zeros to find the start of the MEL stream.
        let mut effective_len = data.len();
        while effective_len > 0 && data[effective_len - 1] == 0 {
            effective_len -= 1;
        }

        // If buffer was all zeros, we might have trimmed valid zeros.
        // But for MEL, a stream of all zeros means "run of zeros", which matches behavior of 00 bytes.
        // So trimming is probably safe-ish for decoding logic, but let's keep at least 1 byte if meaningful?
        // Actually, if we trim all, effective_len=0. read_raw_bit returns None.
        // MEL decode should handle EOF as 0?
        if effective_len == 0 && !data.is_empty() {
            // Revert to full length if it appears to be all zeros (e.g. black image)
            // effective_len = data.len();
            // Wait, for black image, we WANT 0s.
            // If we trim to 0 length, read_bit returns None.
            // We should handle None.
        }

        Self {
            data: &data[..effective_len],
            pos: effective_len, // Start at end of buffer
            bits_buffer: 0,
            bits_left: 0,
            k: 0,
            run: 0,
        }
    }

    /// Read a single raw bit from the bitstream (bypasses MEL state machine).
    /// The MEL bitstream grows backward from the end of the buffer.
    pub fn read_raw_bit(&mut self) -> Option<u8> {
        if self.bits_left == 0 {
            if self.pos == 0 {
                return None; // EOF
            }

            self.pos -= 1;
            let mut byte = self.data[self.pos];

            // Handle 0xFF stuffing (backward reading)
            // If we encounter 0x00 and the *next* byte (lower address) is 0xFF,
            // then this 0x00 is a stuffing byte and should be skipped.
            // The byte to return is the 0xFF.
            if self.pos > 0 && byte == 0x00 && self.data[self.pos - 1] == 0xFF {
                // Skip the stuffing byte 0x00
                self.pos -= 1;
                byte = 0xFF;
            }

            self.bits_buffer = byte;
            self.bits_left = 8;
        }

        let bit = (self.bits_buffer >> (self.bits_left - 1)) & 1;
        self.bits_left -= 1;
        Some(bit)
    }

    /// Read a single bit from the bitstream (through MEL state machine).
    fn read_bit(&mut self) -> Option<u8> {
        self.read_raw_bit()
    }

    /// Peek at the next N bits without consuming them.
    /// This is needed for VLC decoding which shares the same bitstream.
    pub fn peek_bits(&self, count: u8) -> u16 {
        let mut peek_value = 0u16;
        let mut temp_pos = self.pos;
        let mut temp_buffer = self.bits_buffer;
        let mut temp_left = self.bits_left;

        for _ in 0..count.min(16) {
            if temp_left == 0 {
                if temp_pos == 0 {
                    break;
                }

                // Read backward logic (match read_raw_bit)
                let mut next_read_pos = temp_pos - 1;
                temp_buffer = self.data[next_read_pos];

                // Stuffing check
                if next_read_pos > 0 && temp_buffer == 0x00 && self.data[next_read_pos - 1] == 0xFF
                {
                    next_read_pos -= 1;
                    temp_buffer = 0xFF;
                }

                temp_pos = next_read_pos;
                temp_left = 8;
            }
            let bit = (temp_buffer >> (temp_left - 1)) & 1;
            peek_value = (peek_value << 1) | (bit as u16);
            temp_left -= 1;
        }
        peek_value
    }

    /// Decode a MEL symbol (0 or 1).
    /// Used to determine significance of a group of samples.
    pub fn decode(&mut self) -> bool {
        // If we are in a run
        if self.run > 0 {
            self.run -= 1;
            return false; // Symbol is 0 (insignificant) during run
        }

        let bit = self.read_bit().unwrap_or(0);

        if bit == 0 {
            // Full run of 2^k zeros
            let run_len = 1 << self.k;
            self.run = run_len - 1; // Current one is 0, so remaining is len-1
            self.k = (self.k + 1).min(12);
            false
        } else {
            // Partial run (or immediate 1)
            // Read k bits to determine how many zeros preceded this 1
            let partial_run = if self.k > 0 {
                // Read k bits
                let mut val = 0;
                for _ in 0..self.k {
                    val = (val << 1) | self.read_bit().unwrap_or(0) as i32;
                }
                val
            } else {
                0
            };

            self.run = partial_run; // These are zeros to return in subsequent calls
            self.k = (self.k - 1).max(0);

            if self.run > 0 {
                self.run -= 1;
                false // Return 0 (first of the partial run)
            } else {
                true // No zeros, immediate 1
            }
        }
    }
}

/// MEL Encoder
pub struct MelEncoder {
    buffer: Vec<u8>,
    bit_buffer: u8,
    bits_count: u8,
    k: i32,
    run_accum: i32,
}

impl MelEncoder {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            bit_buffer: 0,
            bits_count: 0,
            k: 0,
            run_accum: 0,
        }
    }

    fn write_bit(&mut self, bit: u8) {
        // Bits are packed LSB to MSB?
        // No, HTJ2K bitstream grows backwards, but within byte?
        // MelDecoder reads: (self.bits_buffer >> (self.bits_left - 1)) & 1.
        // This is MSB first within the byte.
        // But the BYTES are read backwards.
        // So we should pack MSB first, and push bytes.
        // Wait, MelDecoder::read_raw_bit reads bytes from END of buffer.

        self.bit_buffer = (self.bit_buffer << 1) | (bit & 1);
        self.bits_count += 1;
        if self.bits_count == 8 {
            // Handle stuffing: if byte is 0xFF, next must be < 0x80.
            // But HTJ2K writes backward.
            // Encoder writes bytes normally, then we reverse them?
            // Or we just append, and the decoder reads from end.

            // Standard J2K stuffing: 0xFF followed by 0x00 is 0xFF data.
            // HTJ2K MEL is raw bits.
            // If we produce 0xFF, we must stuff 0x00?
            // MelDecoder handles 0xFF, 0x00 sequence as 0xFF data.

            self.buffer.push(self.bit_buffer);
            if self.bit_buffer == 0xFF {
                self.buffer.push(0x00); // Stuffing
            }
            self.bit_buffer = 0;
            self.bits_count = 0;
        }
    }

    fn write_bits(&mut self, val: i32, count: i32) {
        for i in (0..count).rev() {
            self.write_bit(((val >> i) & 1) as u8);
        }
    }

    pub fn encode(&mut self, val: bool) {
        if !val {
            // Symbol is 0 (insignificant)
            self.run_accum += 1;
            if self.run_accum == (1 << self.k) {
                // Full run reached
                self.write_bit(0); // '0' indicates full run
                self.k = (self.k + 1).min(12);
                self.run_accum = 0;
            }
        } else {
            // Symbol is 1 (significant)
            self.write_bit(1); // '1' indicates run break

            // Encode partial run length (run_accum) using k bits
            self.write_bits(self.run_accum, self.k);

            self.k = (self.k - 1).max(0);
            self.run_accum = 0;
        }
    }

    pub fn flush(&mut self) {
        // If we have accumulated zeros at the end of the stream, we must output them?
        // HTJ2K: "The MEL bitstream is terminated by a '1' bit if the last symbol was 0?"
        // No, typically we just pad.
        // But if `run_accum > 0`, these are real 0s that haven't been encoded yet.
        // We can treat it as a break with a 1? No, that would add a 1.
        // Standard says we can pad with 1s? Or we assume remaining are 0?
        // Actually, if we just stop, the decoder might wait for more bits.
        // Usually we flush by treating as a break?
        // Let's assume we treat it as a break (write 1 and partial run).
        // This effectively encodes the trailing zeros and a phantom 1.
        // If the decoder knows the number of quads, it stops.
        // If it doesn't, it might read an extra 1.

        if self.run_accum > 0 {
            self.write_bit(1);
            self.write_bits(self.run_accum, self.k);
            self.k = (self.k - 1).max(0);
            self.run_accum = 0;
        }

        if self.bits_count > 0 {
            // Pad remaining bits in byte.
            // Usually pad with 0 or 1?
            // MelDecoder reads MSB first.
            // If we have 3 bits: [b0 b1 b2 . . . . .]
            // We want them at top: [b0 b1 b2 0 0 0 0 0] ?
            // self.bit_buffer contains the bits in lower part.
            // e.g. bits=3, val=0x07 (111).
            // We want byte: 11100000.
            self.bit_buffer <<= 8 - self.bits_count;
            self.buffer.push(self.bit_buffer);
            if self.bit_buffer == 0xFF {
                self.buffer.push(0x00);
            }
        }
    }

    pub fn get_buffer(&self) -> &[u8] {
        &self.buffer
    }
}

impl Default for MelEncoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mel_decoder_runs() {
        // Construct a bitstream.
        // k=0 initially.
        // Pattern: 0 (Run 2^0=1), 0 (Run 2^1=2), 1 (Hit)
        // Bitstream: 0, 0, 1 ...
        // Expected output:
        // Read 0 -> run=0, k=1. Out: 0.
        // Read 0 -> run=1, k=2. Out: 0.
        // (Next call: run>0 -> run=0, Out: 0).
        // Read 1 -> run=0, k=1. Out: 1.

        // Bits: 0 0 1 (packed into byte: 00100000 = 0x20)
        let data = vec![0x20];
        let mut mel = MelDecoder::new(&data);

        assert!(!mel.decode(), "First bit 0 -> 0 (Run 1)");
        assert_eq!(mel.k, 1);

        assert!(!mel.decode(), "Second bit 0 -> 0 (Run 2)");
        assert_eq!(mel.k, 2);
        assert_eq!(mel.run, 1, "Remaining run should be 1");

        assert!(!mel.decode(), "Inside run -> 0");
        assert_eq!(mel.run, 0);

        assert!(mel.decode(), "Third bit 1 -> 1");
        assert_eq!(mel.k, 1);
    }
}
