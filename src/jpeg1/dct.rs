//! Discrete Cosine Transform (DCT) implementation for JPEG 1.

use std::f32::consts::PI;

pub const BLOCK_SIZE: usize = 8;
pub const BLOCK_DIM: usize = BLOCK_SIZE * BLOCK_SIZE;

pub fn fdct_8x8(input: &[f32; 64], output: &mut [f32; 64]) {
    for u in 0..8 {
        for v in 0..8 {
            let mut sum = 0.0f32;
            for x in 0..8 {
                for y in 0..8 {
                    let cos_x = (((2 * x + 1) * u) as f32 * PI) / 16.0;
                    let cos_y = (((2 * y + 1) * v) as f32 * PI) / 16.0;
                    sum += input[x * 8 + y] * cos_x.cos() * cos_y.cos();
                }
            }
            let cu = if u == 0 { 1.0 / 2.0f32.sqrt() } else { 1.0 };
            let cv = if v == 0 { 1.0 / 2.0f32.sqrt() } else { 1.0 };
            output[u * 8 + v] = 0.25 * cu * cv * sum;
        }
    }
}

pub fn idct_8x8_baseline(input: &[f32; 64], output: &mut [f32; 64]) {
    for x in 0..8 {
        for y in 0..8 {
            let mut sum = 0.0f32;
            for u in 0..8 {
                for v in 0..8 {
                    let cu = if u == 0 { 1.0 / 2.0f32.sqrt() } else { 1.0 };
                    let cv = if v == 0 { 1.0 / 2.0f32.sqrt() } else { 1.0 };
                    let cos_x = (((2 * x + 1) * u) as f32 * PI) / 16.0;
                    let cos_y = (((2 * y + 1) * v) as f32 * PI) / 16.0;
                    sum += cu * cv * input[u * 8 + v] * cos_x.cos() * cos_y.cos();
                }
            }
            output[x * 8 + y] = 0.25 * sum;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fdct_idct_dc_only() {
        let mut input = [-128.0f32; 64];
        let mut dct_coeffs = [0.0f32; 64];
        fdct_8x8(&input, &mut dct_coeffs);
        
        let mut output = [0.0f32; 64];
        idct_8x8_baseline(&dct_coeffs, &mut output);
        
        for i in 0..64 {
            assert!((input[i] - output[i]).abs() < 0.1, "Mismatch at {}: {} vs {}", i, input[i], output[i]);
        }
    }
}
