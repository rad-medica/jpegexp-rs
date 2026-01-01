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
}

impl<'a, 'b> J2kBitReader<'a, 'b> {
    pub fn new(reader: &'a mut crate::jpeg_stream_reader::JpegStreamReader<'b>) -> Self {
        Self {
            reader,
        }
    }

    pub fn read_bit(&mut self) -> Result<u8, BitIoError> {
        self.reader.read_bit().map_err(|_| BitIoError)
    }

    pub fn align_to_byte(&mut self) {
        self.reader.align_to_byte();
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
}

impl Default for J2kBitWriter {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            bit_buffer: 0,
            bits_count: 0,
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
        if self.bits_count == 8 {
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
