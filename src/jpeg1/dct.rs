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

#[allow(dead_code)]
pub fn idct_8x8_fixed_point(input: &[f32; 64], output: &mut [f32; 64]) {
    // A simple, separable, fixed-point IDCT
    // Scale factor: 12 bits (4096)

    let mut intermediate = [0i32; 64];

    // Row pass
    for y in 0..8 {
        for x in 0..8 {
            let mut val = 0i32;
            for u in 0..8 {
                let cu = if u == 0 { 2896 } else { 4096 }; // 1/sqrt(2) * 4096
                let angle = ((2 * x + 1) * u) as f32 * std::f32::consts::PI / 16.0;
                let cos_val = (angle.cos() * 4096.0) as i32;
                let i_val = (input[y * 8 + u] * 256.0) as i32; // Scale input by 256 (8 bits)

                // val += i_val * cu * cos_val
                // shifts: cu(12) + cos(12) = 24. We want to keep some precision.
                // i_val is 8+bits.
                val += (i_val * cu >> 12) * cos_val >> 12;
            }
            intermediate[y * 8 + x] = val;
        }
    }

    // Column pass
    for x in 0..8 {
        for y in 0..8 {
            let mut val = 0i32;
            for v in 0..8 {
                let cv = if v == 0 { 2896 } else { 4096 };
                let angle = ((2 * y + 1) * v) as f32 * std::f32::consts::PI / 16.0;
                let cos_val = (angle.cos() * 4096.0) as i32;
                let i_val = intermediate[v * 8 + x];

                val += (i_val * cv >> 12) * cos_val >> 12;
            }
            // Final scaling
            // We reduced shifts during accumulation to avoid overflow? No, we used >>12.
            // Input scaled by 256.
            // Formula requires * 0.25.
            // Output is f32.
            output[y * 8 + x] = (val as f32) / 256.0 * 0.25;
        }
    }
}
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_fdct_idct_dc_only() {
        let input = [-128.0f32; 64];
        let mut dct_coeffs = [0.0f32; 64];
        fdct_8x8(&input, &mut dct_coeffs);

        let mut output = [0.0f32; 64];
        idct_8x8_baseline(&dct_coeffs, &mut output);

        for i in 0..64 {
            assert!(
                (input[i] - output[i]).abs() < 0.1,
                "Mismatch at {}: {} vs {}",
                i,
                input[i],
                output[i]
            );
        }
    }
}
