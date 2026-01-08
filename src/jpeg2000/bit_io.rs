// ...existing code...
// --- BitIoError definition and impls ---

#[derive(Debug, Clone)]
pub struct BitIoError;

impl std::fmt::Display for BitIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bit IO error")
    }
}

impl std::error::Error for BitIoError {}

pub struct J2kBitReader<'a, 'b> {
    reader: &'a mut crate::jpeg_stream_reader::JpegStreamReader<'b>,
    bit_buffer: u8,
    bits_count: u8,
    last_byte: u8,
}

impl<'a, 'b> J2kBitReader<'a, 'b> {
    pub fn new(reader: &'a mut crate::jpeg_stream_reader::JpegStreamReader<'b>) -> Self {
        Self {
            reader,
            bit_buffer: 0,
            bits_count: 0,
            last_byte: 0,
        }
    }

    pub fn read_bit(&mut self) -> Result<u8, BitIoError> {
        if self.bits_count == 0 {
            let b = self.reader.read_u8().map_err(|_| BitIoError)?;

            if self.last_byte == 0xFF {
                // Stuffed byte: MSB is stuffed 0, bits 6..0 are data
                // We verify MSB is 0? Standard says "next byte shall be < 0x90".
                // We consume 7 bits.
                self.bit_buffer = b;
                self.bits_count = 7;
                // Note: bit_buffer still has b7 at top.
                // If bits_count=7, we read from bit index 6 (0-based) down to 0?
                // Or we mask out MSB?
                // self.bit_buffer &= 0x7F; // Ensure MSB is ignored
                // If we want MSB-first of the DATA.
                // Data bits are b6..b0.
                // First bit is b6.
                // Our logic below: bit = (buffer >> (count - 1)) & 1.
                // If count=7. shift=6. We read bit 6. Correct.
            } else {
                self.bit_buffer = b;
                self.bits_count = 8;
            }
            self.last_byte = b;
        }

        let bit = (self.bit_buffer >> (self.bits_count - 1)) & 1;
        self.bits_count -= 1;

        Ok(bit)
    }

    pub fn align_to_byte(&mut self) {
        self.bits_count = 0;
        self.bit_buffer = 0;
        self.last_byte = 0;
    }

    pub fn read_bits(&mut self, mut count: u8) -> Result<u32, BitIoError> {
        let mut bits = 0u32;
        while count > 0 {
            let bit = self.read_bit()?;
            bits = (bits << 1) | (bit as u32);
            count -= 1;
        }
        Ok(bits)
    }
}

pub struct J2kBitWriter {
    data: Vec<u8>,
    bit_buffer: u8,
    bits_count: u8,
    last_byte_ff: bool,
}

impl Default for J2kBitWriter {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            bit_buffer: 0,
            bits_count: 0,
            last_byte_ff: false,
        }
    }
}

impl J2kBitWriter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn write_bit(&mut self, bit: u8) {
        self.bit_buffer = (self.bit_buffer << 1) | (bit & 1);
        self.bits_count += 1;

        let limit = if self.last_byte_ff { 7 } else { 8 };

        if self.bits_count == limit {
            self.flush_byte();
        }
    }

    pub fn write_bits(&mut self, value: u32, mut count: u8) {
        while count > 0 {
            let bit = ((value >> (count - 1)) & 1) as u8;
            self.write_bit(bit);
            count -= 1;
        }
    }

    fn flush_byte(&mut self) {
        let b = self.bit_buffer;
        self.data.push(b);
        self.last_byte_ff = b == 0xFF;
        self.bit_buffer = 0;
        self.bits_count = 0;
    }

    pub fn align_to_byte(&mut self) {
        if self.bits_count > 0 {
            let limit = if self.last_byte_ff { 7 } else { 8 };
            self.bit_buffer <<= limit - self.bits_count;
            self.flush_byte();
        }
    }

    pub fn finish(mut self) -> Vec<u8> {
        self.align_to_byte();
        self.data
    }

    pub fn get_output(&self) -> &[u8] {
        &self.data
    }
}
