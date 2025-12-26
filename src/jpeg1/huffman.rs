//! Huffman coding implementation for JPEG 1 Baseline.

use crate::error::JpeglsError;

#[derive(Debug, Clone, Copy, Default)]
pub struct HuffmanCode {
    pub value: u16,
    pub length: u8,
}

pub const STD_LUMINANCE_DC_LENGTHS: [u8; 16] = [
    0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0
];

pub const STD_LUMINANCE_DC_VALUES: [u8; 12] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11
];

pub const STD_LUMINANCE_AC_LENGTHS: [u8; 16] = [
    0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 125
];

pub const STD_LUMINANCE_AC_VALUES: [u8; 162] = [
    0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 
    0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07, 
    0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xa1, 0x08, 
    0x23, 0x42, 0xb1, 0xc1, 0x15, 0x52, 0xd1, 0xf0, 
    0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0a, 0x16, 
    0x17, 0x18, 0x19, 0x1a, 0x25, 0x26, 0x27, 0x28, 
    0x29, 0x2a, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 
    0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 
    0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 
    0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 
    0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 
    0x7a, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 
    0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 
    0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 
    0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 
    0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5, 
    0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4, 
    0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe1, 0xe2, 
    0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 
    0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 
    0xf9, 0xfa,
];

#[derive(Clone)]
pub struct HuffmanTable {
    pub codes: [HuffmanCode; 256],
    pub min_code: [i32; 16],
    pub max_code: [i32; 16],
    pub val_ptr: [i32; 16],
    pub values: Vec<u8>,
}

impl HuffmanTable {
    pub fn build_from_dht(lengths: &[u8; 16], values: &[u8]) -> Self {
        let mut table = Self {
            codes: [HuffmanCode::default(); 256],
            min_code: [0; 16],
            max_code: [-1; 16],
            val_ptr: [0; 16],
            values: values.to_vec(),
        };

        let mut code = 0u16;
        let mut val_idx = 0;
        for i in 0..16 {
            let n = lengths[i] as usize;
            if n > 0 {
                table.min_code[i] = code as i32;
                table.val_ptr[i] = val_idx as i32;
                for _ in 0..n {
                    let v = values[val_idx] as usize;
                    table.codes[v] = HuffmanCode { value: code, length: (i + 1) as u8 };
                    code += 1;
                    val_idx += 1;
                }
                table.max_code[i] = (code - 1) as i32;
            }
            code <<= 1;
        }
        table
    }

    pub fn decode(&self, reader: &mut JpegBitReader) -> Result<u8, JpeglsError> {
        let mut code = 0i32;
        for i in 0..16 {
            let bit = reader.read_bits(1)? as i32;
            code = (code << 1) | bit;
            if code <= self.max_code[i] {
                let idx = self.val_ptr[i] + (code - self.min_code[i]);
                return Ok(self.values[idx as usize]);
            }
        }
        Err(JpeglsError::InvalidData)
    }

    pub fn standard_luminance_dc() -> Self {
        Self::build_from_dht(&STD_LUMINANCE_DC_LENGTHS, &STD_LUMINANCE_DC_VALUES)
    }

    pub fn standard_luminance_ac() -> Self {
        Self::build_from_dht(&STD_LUMINANCE_AC_LENGTHS, &STD_LUMINANCE_AC_VALUES)
    }
}

pub struct JpegBitReader<'a> {
    source: &'a [u8],
    position: usize,
    bit_buffer: u32,
    bits_in_buffer: i32,
}

impl<'a> JpegBitReader<'a> {
    pub fn new(source: &'a [u8]) -> Self {
        Self { source, position: 0, bit_buffer: 0, bits_in_buffer: 0 }
    }

    pub fn read_bits(&mut self, count: u8) -> Result<u16, JpeglsError> {
        if count == 0 { return Ok(0); }
        let count = count as i32;
        while self.bits_in_buffer < count {
            let byte = self.read_byte_unstuffed()?;
            self.bit_buffer = (self.bit_buffer << 8) | byte as u32;
            self.bits_in_buffer += 8;
        }
        let shift = self.bits_in_buffer - count;
        let val = (self.bit_buffer >> shift) & ((1 << count) - 1);
        self.bits_in_buffer = shift;
        if shift > 0 {
            self.bit_buffer &= (1 << shift) - 1;
        } else {
            self.bit_buffer = 0;
        }
        Ok(val as u16)
    }

    fn read_byte_unstuffed(&mut self) -> Result<u8, JpeglsError> {
        if self.position >= self.source.len() { return Err(JpeglsError::InvalidData); }
        let byte = self.source[self.position];
        self.position += 1;
        if byte == 0xFF {
            if self.position < self.source.len() && self.source[self.position] == 0x00 {
                self.position += 1;
            }
        }
        Ok(byte)
    }
}

pub struct JpegBitWriter<'a> {
    destination: &'a mut [u8],
    position: usize,
    bit_buffer: u32,
    bits_in_buffer: i32,
}

impl<'a> JpegBitWriter<'a> {
    pub fn new(destination: &'a mut [u8]) -> Self {
        Self { destination, position: 0, bit_buffer: 0, bits_in_buffer: 0 }
    }

    pub fn write_bits(&mut self, value: u16, length: u8) -> Result<(), JpeglsError> {
        if length == 0 { return Ok(()); }
        let length = length as i32;
        self.bit_buffer = (self.bit_buffer << length) | (value as u32 & ((1 << length) - 1));
        self.bits_in_buffer += length;
        while self.bits_in_buffer >= 8 {
            let shift = self.bits_in_buffer - 8;
            let byte = (self.bit_buffer >> shift) as u8;
            self.emit_byte(byte)?;
            self.bits_in_buffer = shift;
            if shift > 0 {
                self.bit_buffer &= (1 << shift) - 1;
            } else {
                self.bit_buffer = 0;
            }
        }
        Ok(())
    }

    fn emit_byte(&mut self, byte: u8) -> Result<(), JpeglsError> {
        if self.position >= self.destination.len() { return Err(JpeglsError::ParameterValueNotSupported); }
        self.destination[self.position] = byte;
        self.position += 1;
        if byte == 0xFF {
            if self.position >= self.destination.len() { return Err(JpeglsError::ParameterValueNotSupported); }
            self.destination[self.position] = 0x00;
            self.position += 1;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), JpeglsError> {
        if self.bits_in_buffer > 0 {
            let pad_bits = 8 - self.bits_in_buffer;
            self.write_bits((1 << pad_bits) - 1, pad_bits as u8)?;
        }
        Ok(())
    }

    pub fn len(&self) -> usize { self.position }
}

pub struct HuffmanEncoder {
    pub dc_previous_value: [i16; 4],
}

impl HuffmanEncoder {
    pub fn new() -> Self { Self { dc_previous_value: [0; 4] } }
    pub fn get_category(v: i16) -> u8 {
        if v == 0 { return 0; }
        (16 - v.abs().leading_zeros()) as u8
    }
    pub fn get_diff_bits(v: i16, cat: u8) -> (u16, u8) {
        if cat == 0 { return (0, 0); }
        if v >= 0 { (v as u16, cat) } else { ((v + (1 << cat) - 1) as u16, cat) }
    }
    pub fn decode_value_bits(bits: u16, cat: u8) -> i16 {
        if cat == 0 { return 0; }
        let threshold = 1 << (cat - 1);
        if bits >= threshold { bits as i16 } else { (bits as i32 - (1 << cat) + 1) as i16 }
    }
}
