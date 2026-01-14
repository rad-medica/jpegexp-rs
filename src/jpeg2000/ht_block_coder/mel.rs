/// Magnitude Exponent Logic (MEL) decoder state and functionality.
/// Implements the MEL coding scheme defined in ISO/IEC 15444-15 (HTJ2K).
/// MEL reads FORWARD from the start of the MEL/VLC buffer.
pub struct MelDecoder<'a> {
    data: &'a [u8],
    pos: usize,
    tmp: u64, // Bit buffer (64 bits)
    bits: u32, // Number of valid bits in tmp
    length: i32, // Remaining bytes to read
    unstuff: bool, // Whether the last byte read was 0xFF
    k: i32, // State index (exponent)
    num_runs: i32, // Number of decoded runs in buffer
    runs: u64, // Buffer of decoded runs (7 bits per run)
}

impl<'a> MelDecoder<'a> {
    pub fn new(data: &'a [u8], _first_byte_bits: u8) -> Self {
        // MEL reads forward from position 0
        // data.len() is already Scup-1 (the MEL/VLC segment length)
        let mut decoder = Self {
            data,
            pos: 0,
            tmp: 0,
            bits: 0,
            length: data.len() as i32, // This is Scup - 1
            unstuff: false,
            k: 0,
            num_runs: 0,
            runs: 0,
        };

        // Initial fill of tmp buffer (align to 4-byte boundary like OpenHTJ2K)
        // Calculate alignment: 4 - (buf address & 0x3)
        let num = 4 - (decoder.pos & 0x3);
        let num_initial = num.min(decoder.length as usize);

        for _ in 0..num_initial {
            let d = if decoder.length > 0 {
                decoder.data[decoder.pos]
            } else {
                0xFF
            };

            let d = if decoder.length == 1 {
                d | 0x0F // Last byte of MEL+VLC, set LSBs
            } else {
                d
            };

            decoder.pos += if decoder.length > 0 { 1 } else { 0 };
            decoder.length -= if decoder.length > 0 { 1 } else { 0 };

            let d_bits = 8 - if decoder.unstuff { 1 } else { 0 };
            decoder.tmp = (decoder.tmp << d_bits) | (d as u64);
            decoder.bits += d_bits;
            decoder.unstuff = d == 0xFF;
        }

        // Shift tmp to MSB (critical for peek_bits to work correctly)
        decoder.tmp <<= 64 - decoder.bits;
        decoder
    }

    /// Read more bits from the MEL stream into tmp buffer
    fn read(&mut self) {
        if self.bits > 32 {
            return; // Enough bits available
        }

        let mut val = 0xFFFFFFFFu32;

        if self.length > 4 {
            // Read 4 bytes at once
            if self.pos + 4 <= self.data.len() {
                val = u32::from_le_bytes(
                    [
                        self.data[self.pos],
                        self.data[self.pos + 1],
                        self.data[self.pos + 2],
                        self.data[self.pos + 3],
                    ],
                );
                self.pos += 4;
                self.length -= 4;
            }
        } else if self.length > 0 {
            // Read remaining bytes one at a time
            let mut i = 0;
            while self.length > 1 && self.pos < self.data.len() {
                let v = self.data[self.pos];
                self.pos += 1;
                let mask = !(0xFFu32 << i);
                val = (val & mask) | ((v as u32) << i);
                self.length -= 1;
                i += 8;
            }

            // Last byte (length == 1) - MEL and VLC may overlap
            if self.length == 1 && self.pos < self.data.len() {
                let mut v = self.data[self.pos];
                v |= 0x0F; // Set lower nibble
                self.pos += 1;
                let mask = !(0xFFu32 << i);
                val = (val & mask) | ((v as u32) << i);
                self.length -= 1;
            }
        }

        // Unstuff the 32-bit value
        let mut bits_local = 32 - if self.unstuff { 1 } else { 0 };
        let mut t = 0u32;

        // Process 4 bytes with unstuffing
        for byte_idx in 0..4 {
            let byte = ((val >> (byte_idx * 8)) & 0xFF) as u8;
            let unstuff_flag = byte == 0xFF;
            let shift = 8 - if unstuff_flag { 1 } else { 0 };
            t = (t << shift) | (byte as u32);
            if byte_idx < 3 {
                bits_local -= if unstuff_flag { 1 } else { 0 };
            } else {
                self.unstuff = unstuff_flag;
            }
        }

        // Add unstuffed bits to tmp
        self.tmp |= (t as u64) << (64 - bits_local - self.bits);
        self.bits += bits_local;
    }

    /// Peek at the next N bits without consuming them (from MSB of tmp)
    pub fn peek_bits(&mut self, count: u8) -> u32 {
        if self.bits < count as u32 {
            self.read();
        }
        ((self.tmp >> (64 - count)) & ((1u64 << count) - 1)) as u32
    }

    /// Consume N bits from the stream
    pub fn advance(&mut self, count: u8) {
        self.tmp <<= count;
        self.bits = self.bits.saturating_sub(count as u32);
        if self.bits < 32 {
            self.read();
        }
    }

    /// Decode MEL runs and store them in the runs buffer
    fn decode_runs(&mut self) {
        const MEL_E: [i32; 13] = [0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 4, 5];

        if self.bits < 6 {
            self.read();
        }

        if std::env::var("HTJ2K_DEBUG").is_ok() {
            eprintln!(
                "MelDecoder: decode_runs start k={} bits={} tmp={:016X}",
                self.k,
                self.bits,
                self.tmp
            );
        }

        // Decode runs while we have enough bits and space in runs buffer
        while self.bits >= 6 && self.num_runs < 8 {
            let eval = MEL_E[self.k as usize];
            let run: i32;

            // Check the MSB of tmp
            if (self.tmp & (1u64 << 63)) != 0 {
                // "1" bit found
                run = ((1 << eval) - 1) << 1; // Stretch of zeros not terminating in one
                self.k = (self.k + 1).min(12);
                self.tmp <<= 1;
                self.bits -= 1;
            } else {
                // "0" bit found
                let val = ((self.tmp >> (63 - eval)) & ((1u64 << eval) - 1)) as i32;
                run = (val << 1) + 1; // Stretch of zeros terminating with one
                self.k = (self.k - 1).max(0);
                self.tmp <<= eval as u32 + 1;
                self.bits -= eval as u32 + 1;
            }

            // Store run in runs buffer (7 bits per run)
            let shift = self.num_runs * 7;
            self.runs &= !(0x3Fu64 << shift); // Clear 6 bits (enough for run)
            self.runs |= (run as u64) << shift;
            self.num_runs += 1;
        }
    }

    /// Get one MEL run value (public method used by coder)
    pub fn get_run(&mut self) -> i32 {
        if self.num_runs == 0 {
            self.decode_runs();
        }
        let run = (self.runs & 0x7F) as i32;
        self.runs >>= 7;
        self.num_runs -= 1;
        if std::env::var("HTJ2K_DEBUG").is_ok() {
            eprintln!("MelDecoder: get_run returning {}", run);
        }
        run
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mel_decoder_basic() {
        // Basic test to ensure MEL decoder can be created and get runs
        let data = vec![0xFF, 0xFF, 0x0F];
        let mut mel = MelDecoder::new(&data, 4);

        // Should be able to get a run without panicking
        let _run = mel.get_run();
        assert!(true);
    }
}
