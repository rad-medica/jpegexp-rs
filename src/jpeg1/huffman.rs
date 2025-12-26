//! Huffman coding implementation for JPEG 1 Baseline.
//! Handles standard Huffman tables and bit-stream packing.

use crate::error::JpeglsError;

/// Represents a Huffman code with its bit value and length.
#[derive(Debug, Clone, Copy, Default)]
pub struct HuffmanCode {
    pub value: u16,
    pub length: u8,
}

/// Standard JPEG DC luminance Huffman table lengths (Table K.1 in spec).
pub const STD_LUMINANCE_DC_LENGTHS: [u8; 16] = [
    0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0
];

/// Standard JPEG DC luminance Huffman table values (Table K.1 in spec).
pub const STD_LUMINANCE_DC_VALUES: [u8; 12] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11
];

/// Encapsulates MSB-aligned Huffman table for encoding and decoding.
#[derive(Clone)]
pub struct HuffmanTable {
    pub codes: [HuffmanCode; 256],
    pub lengths: [u8; 16],
    pub values: Vec<u8>,
    
    // Decoding fields
    pub min_code: [i32; 16],
    pub max_code: [i32; 16],
    pub val_ptr: [i32; 16],
}

impl HuffmanTable {
    pub fn new() -> Self {
        Self {
            codes: [HuffmanCode::default(); 256],
            lengths: [0; 16],
            values: Vec::new(),
            min_code: [0; 16],
            max_code: [-1; 16],
            val_ptr: [0; 16],
        }
    }

    /// Builds a table from JPEG DHT lengths and values.
    pub fn build_from_dht(lengths: &[u8; 16], values: &[u8]) -> Self {
        let mut table = Self::new();
        table.lengths.copy_from_slice(lengths);
        table.values = values.to_vec();
        
        let mut code = 0u16;
        let mut val_idx = 0;

        for i in 0..16 {
            let n_codes = lengths[i] as usize;
            if n_codes == 0 {
                table.max_code[i] = -1;
            } else {
                table.val_ptr[i] = val_idx as i32;
                table.min_code[i] = code as i32;
                for _ in 0..n_codes {
                    let val = values[val_idx] as usize;
                    table.codes[val] = HuffmanCode {
                        value: code,
                        length: (i + 1) as u8,
                    };
                    code += 1;
                    val_idx += 1;
                }
                table.max_code[i] = (code - 1) as i32;
            }
            code <<= 1;
        }
        table
    }

    pub fn standard_luminance_dc() -> Self {
        Self::build_from_dht(&STD_LUMINANCE_DC_LENGTHS, &STD_LUMINANCE_DC_VALUES)
    }

    /// Returns the standard Luminance AC Huffman table.
    pub fn standard_luminance_ac() -> Self {
        let lengths = [0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 125];
        let values = [
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
        Self::build_from_dht(&lengths, &values)
    }

    /// Decodes the next symbol from the given JpegBitReader.
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
}

/// Helper for reading bits from bytes with JPEG anti-stuffing (skipping FF00).
pub struct JpegBitReader<'a> {
    source: &'a [u8],
    position: usize,
    bit_buffer: u32,
    bits_in_buffer: i32,
}

impl<'a> JpegBitReader<'a> {
    pub fn new(source: &'a [u8]) -> Self {
        Self {
            source,
            position: 0,
            bit_buffer: 0,
            bits_in_buffer: 0,
        }
    }

    pub fn read_bits(&mut self, count: u8) -> Result<u16, JpeglsError> {
        if count == 0 { return Ok(0); }
        let count = count as i32;
        while self.bits_in_buffer < count {
            let byte = self.read_byte_antituffed()?;
            self.bit_buffer = (self.bit_buffer << 8) | (byte as u32);
            self.bits_in_buffer += 8;
        }

        let shift = self.bits_in_buffer - count;
        let value = (self.bit_buffer >> shift) & ((1 << count) - 1);
        self.bits_in_buffer -= count;
        Ok(value as u16)
    }

    fn read_byte_antituffed(&mut self) -> Result<u8, JpeglsError> {
        if self.position >= self.source.len() {
            return Err(JpeglsError::InvalidData);
        }
        let byte = self.source[self.position];
        self.position += 1;

        if byte == 0xFF {
            if self.position >= self.source.len() {
                return Err(JpeglsError::InvalidData);
            }
            let next_byte = self.source[self.position];
            if next_byte == 0x00 {
                self.position += 1;
            }
        }
        Ok(byte)
    }
}

/// Helper for packing bits into bytes with JPEG bit-stuffing (FF00).
pub struct JpegBitWriter<'a> {
    destination: &'a mut [u8],
    position: usize,
    bit_buffer: u32,
    bits_in_buffer: i32,
}

impl<'a> JpegBitWriter<'a> {
    pub fn new(destination: &'a mut [u8]) -> Self {
        Self {
            destination,
            position: 0,
            bit_buffer: 0,
            bits_in_buffer: 0,
        }
    }

    pub fn write_bits(&mut self, value: u16, length: u8) -> Result<(), JpeglsError> {
        if length == 0 { return Ok(()); }
        let length = length as i32;
        let mask = (1u32 << length) - 1;
        self.bit_buffer = (self.bit_buffer << length) | (value as u32 & mask);
        self.bits_in_buffer += length;

        while self.bits_in_buffer >= 8 {
            let shift = self.bits_in_buffer - 8;
            let byte = ((self.bit_buffer >> shift) & 0xFF) as u8;
            self.emit_byte(byte)?;
            self.bits_in_buffer = shift;
            // No need to mask bit_buffer here if we use a mask when adding bits 
            // but for safety, mask it now to keep only 'shift' bits.
            if shift > 0 {
                self.bit_buffer &= (1u32 << shift) - 1;
            } else {
                self.bit_buffer = 0;
            }
        }
        Ok(())
    }

    fn emit_byte(&mut self, byte: u8) -> Result<(), JpeglsError> {
        if self.position >= self.destination.len() {
            return Err(JpeglsError::ParameterValueNotSupported);
        }
        self.destination[self.position] = byte;
        self.position += 1;

        if byte == 0xFF {
            if self.position >= self.destination.len() {
                return Err(JpeglsError::ParameterValueNotSupported);
            }
            self.destination[self.position] = 0x00;
            self.position += 1;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), JpeglsError> {
        if self.bits_in_buffer > 0 {
            // Pad with ones (standard JPEG practice for scan data)
            let pad_bits = 8 - self.bits_in_buffer;
            let value = (1u32 << pad_bits) - 1;
            self.write_bits(value as u16, pad_bits as u8)?;
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.position
    }
}

/// Encapsulates Huffman coding state.
pub struct HuffmanEncoder {
    pub dc_previous_value: [i16; 4], // Support for up to 4 components
}

impl HuffmanEncoder {
    pub fn new() -> Self {
        Self {
            dc_previous_value: [0; 4],
        }
    }

    /// Computes the magnitude category of an integer (ISO/IEC 10918-1 F.1.2.1).
    pub fn get_category(value: i16) -> u8 {
        if value == 0 { return 0; }
        let abs_val = value.abs() as u16;
        (16 - abs_val.leading_zeros()) as u8
    }

    /// Encodes the bits for a given category and value (ISO/IEC 10918-1 F.1.2.1.1).
    pub fn get_diff_bits(value: i16, category: u8) -> (u16, u8) {
        if category == 0 { return (0, 0); }
        if value >= 0 {
            (value as u16, category)
        } else {
            ((value + (1 << category) - 1) as u16, category)
        }
    }

    /// Decodes the value from bits given its category (ISO/IEC 10918-1 F.1.2.1.1).
    pub fn decode_value_bits(bits: u16, category: u8) -> i16 {
        if category == 0 { return 0; }
        let threshold = 1 << (category - 1);
        if bits >= threshold {
            bits as i16
        } else {
            (bits as i32 - (1 << category as i32) + 1) as i16
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_writer_reader_roundtrip() {
        let mut buffer = [0u8; 100];
        {
            let mut writer = JpegBitWriter::new(&mut buffer);
            writer.write_bits(0x01, 2).unwrap();
            writer.write_bits(0xFF, 8).unwrap(); // Should trigger stuffing
            writer.write_bits(0x0A, 4).unwrap();
            writer.flush().unwrap();
        }

        let mut reader = JpegBitReader::new(&buffer);
        assert_eq!(reader.read_bits(2).unwrap(), 0x01);
        assert_eq!(reader.read_bits(8).unwrap(), 0xFF);
        assert_eq!(reader.read_bits(4).unwrap(), 0x0A);
    }
}
