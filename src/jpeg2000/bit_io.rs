pub struct J2kBitReader<'a> {
    data: &'a [u8],
    pos: usize,
    bit_buffer: u8,
    bits_left: u8,
}

impl<'a> J2kBitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            bit_buffer: 0,
            bits_left: 0,
        }
    }

    pub fn read_bit(&mut self) -> Result<u8, ()> {
        if self.bits_left == 0 {
            if self.pos >= self.data.len() {
                return Err(()); // EOF
            }
            let b = self.data[self.pos];
            self.pos += 1;

            // Byte stuffing handling for J2K Packet Headers
            if b == 0xFF {
                if self.pos < self.data.len() {
                    let next = self.data[self.pos];
                    if next == 0x00 {
                        self.pos += 1; // Skip stuffing
                    }
                }
            }

            self.bit_buffer = b;
            self.bits_left = 8;
        }

        let bit = (self.bit_buffer >> (self.bits_left - 1)) & 1;
        self.bits_left -= 1;
        Ok(bit)
    }

    pub fn read_bits(&mut self, mut count: u8) -> Result<u32, ()> {
        let mut bits = 0u32;
        while count > 0 {
            let bit = self.read_bit()?;
            bits = (bits << 1) | (bit as u32);
            count -= 1;
        }
        Ok(bits)
    }

    pub fn has_data(&self) -> bool {
        self.pos < self.data.len() || self.bits_left > 0
    }

    pub fn position(&self) -> usize {
        self.pos
    }
}

pub struct J2kBitWriter {
    data: Vec<u8>,
    bit_buffer: u8,
    bits_count: u8,
}

impl J2kBitWriter {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            bit_buffer: 0,
            bits_count: 0,
        }
    }

    pub fn write_bit(&mut self, bit: u8) {
        self.bit_buffer = (self.bit_buffer << 1) | (bit & 1);
        self.bits_count += 1;
        if self.bits_count == 8 {
            self.flush_byte();
        }
    }

    fn flush_byte(&mut self) {
        let b = self.bit_buffer;
        self.data.push(b);
        if b == 0xFF {
            // Stuffing
            self.data.push(0x00);
        }
        self.bit_buffer = 0;
        self.bits_count = 0;
    }

    pub fn finish(mut self) -> Vec<u8> {
        if self.bits_count > 0 {
            self.bit_buffer <<= 8 - self.bits_count;
            self.flush_byte();
        }
        self.data
    }

    pub fn get_output(&self) -> &[u8] {
        &self.data
    }
}
