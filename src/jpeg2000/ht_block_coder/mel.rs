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
        Self {
            data,
            pos: 0,
            bits_buffer: 0,
            bits_left: 0,
            k: 0,
            run: 0,
        }
    }

    /// Read a single raw bit from the bitstream (bypasses MEL state machine).
    /// This is used for VLC decoding which shares the same bitstream.
    pub fn read_raw_bit(&mut self) -> Option<u8> {
        if self.bits_left == 0 {
            if self.pos >= self.data.len() {
                return None; // EOF
            }

            self.bits_buffer = self.data[self.pos];
            self.pos += 1;

            // Handle 0xFF stuffing
            if self.bits_buffer == 0xFF && self.pos < self.data.len() {
                let next = self.data[self.pos];
                if next == 0x00 {
                    self.pos += 1;
                }
            }

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
                if temp_pos >= self.data.len() {
                    break;
                }
                temp_buffer = self.data[temp_pos];
                temp_pos += 1;
                if temp_buffer == 0xFF && temp_pos < self.data.len() && self.data[temp_pos] == 0x00 {
                    temp_pos += 1;
                }
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
        // 0 bit -> Run of length 2^k
        // 1 bit -> End of run / Significant logic?

        // HTJ2K MEL Logic:
        // Read bit.
        // If 0: It's a run of 'E' (exponent) zeros?
        // Wait, standard state machine:
        // If bit == 0:
        //   Run of 2^k zeros.
        //   k = min(12, k + 1)
        //   Return 0 (and set run counter for subsequent calls)
        // If bit == 1:
        //   Run length was < 2^k.
        //   Need to read more bits to determine actual length?
        //   Or simply "One 1" and adapt k?

        // Correct logic from standard:
        // When decoding a symbol:
        // 1. If run > 0, return 0, decrement run. (Handled at start)
        // 2. Read 'u' (next bit).
        // 3. If u == 0:
        //    We have a run of 2^k '0's.
        //    self.run = (1 << k) - 1; // Current symbol is 0, plus (2^k - 1) more.
        //    k = min(12, k+1)
        //    return 0
        // 4. If u == 1:
        //    Run broken.
        //    run = 0;
        //    k = max(0, k-1)
        //    return 1 (Significant)

        if bit == 0 {
            let run_len = 1 << self.k;
            self.run = run_len - 1; // Current one is 0, so remaining is len-1
            self.k = (self.k + 1).min(12);
            false
        } else {
            self.run = 0;
            self.k = (self.k - 1).max(0);
            true
        }
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
