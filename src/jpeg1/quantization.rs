//! Quantization implementation for JPEG 1.
//! Handles quantization tables and the quantization of DCT coefficients.

use crate::jpeg1::dct::BLOCK_DIM;

/// Standard JPEG luminance quantization table (Quality 50).
pub const STD_LUMINANCE_QUANT_TABLE: [u8; BLOCK_DIM] = [
    16, 11, 10, 16, 24, 40, 51, 61,
    12, 12, 14, 19, 26, 58, 60, 55,
    14, 13, 16, 24, 40, 57, 69, 56,
    14, 17, 22, 29, 51, 87, 80, 62,
    18, 22, 37, 56, 68, 109, 103, 77,
    24, 35, 55, 64, 81, 104, 113, 92,
    49, 64, 78, 87, 103, 121, 120, 101,
    72, 92, 95, 98, 112, 100, 103, 99,
];

/// Standard JPEG chrominance quantization table (Quality 50).
pub const STD_CHROMINANCE_QUANT_TABLE: [u8; BLOCK_DIM] = [
    17, 18, 24, 47, 99, 99, 99, 99,
    18, 21, 26, 66, 99, 99, 99, 99,
    24, 26, 56, 99, 99, 99, 99, 99,
    47, 66, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
];

/// Quantizes DCT coefficients using a quantization table.
pub fn quantize_block(dct_block: &[f32; BLOCK_DIM], quant_table: &[u8; BLOCK_DIM], output: &mut [i16; BLOCK_DIM]) {
    for i in 0..BLOCK_DIM {
        let q_val = quant_table[i] as f32;
        output[i] = (dct_block[i] / q_val).round() as i16;
    }
}

/// De-quantizes DCT coefficients.
pub fn dequantize_block(quant_block: &[i16; BLOCK_DIM], quant_table: &[u8; BLOCK_DIM], output: &mut [f32; BLOCK_DIM]) {
    for i in 0..BLOCK_DIM {
        let q_val = quant_table[i] as f32;
        output[i] = quant_block[i] as f32 * q_val;
    }
}

/// Scales a quantization table by a quality factor (1-100).
pub fn get_scaled_quant_table(base_table: &[u8; BLOCK_DIM], quality: u32) -> [u8; BLOCK_DIM] {
    let mut scaled_table = [0u8; BLOCK_DIM];
    let s = if quality < 50 { 5000 / quality } else { 200 - 2 * quality };
    
    for i in 0..BLOCK_DIM {
        let mut val = (base_table[i] as u32 * s + 50) / 100;
        if val == 0 { val = 1; }
        if val > 255 { val = 255; }
        scaled_table[i] = val as u8;
    }
    scaled_table
}
