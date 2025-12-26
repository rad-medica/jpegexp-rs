/// Magnitude Refinement and Sign Coding (MagSgn).
/// Handles reading and processing of raw bits for refinement and sign logic.

pub struct MagSgnDecoder<'a> {
    data: &'a [u8],
    pos: usize,
    bits_buffer: u8,
    bits_left: u8,
}

impl<'a> MagSgnDecoder<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            bits_buffer: 0,
            bits_left: 0,
        }
    }

    /// Read bits from the MagSgn stream.
    /// The MagSgn stream is typically read from the *beginning* of the buffer,
    /// while MEL/VLC might be read from the end or reverse?
    /// Standard says: "The MagSgn bitstream ... grows forward from the start of the buffer."
    /// "The MEL bitstream ... grows backward from the end of the buffer."
    /// This splitting logic belongs in the parent coder. This struct just reads forward.
    pub fn read_bit(&mut self) -> Option<u8> {
        if self.bits_left == 0 {
            if self.pos >= self.data.len() {
                return None; // EOF
            }
            self.bits_buffer = self.data[self.pos];
            self.pos += 1;

            // Byte stuffing logic if needed (usually handled by packet parser splitting)
            // If raw buffer provided here, assume clean.

            self.bits_left = 8;
        }

        let bit = (self.bits_buffer >> (self.bits_left - 1)) & 1;
        self.bits_left -= 1;
        Some(bit)
    }

    pub fn read_bits(&mut self, count: u8) -> Option<u32> {
        let mut res = 0;
        for _ in 0..count {
            res = (res << 1) | (self.read_bit()? as u32);
        }
        Some(res)
    }
}
