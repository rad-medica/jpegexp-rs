
pub fn quantize_scalar(coeff: f32, step_size: f32) -> i32 {
    // Dead-zone scalar quantization
    // q = sign(x) * floor(|x| / delta)
    if step_size <= 0.0 { return coeff as i32; } // Should not happen
    
    let sign = if coeff >= 0.0 { 1 } else { -1 };
    let mag = coeff.abs();
    
    (sign as f32 * (mag / step_size).floor()) as i32
}

pub fn dequantize_scalar(q: i32, step_size: f32) -> f32 {
    // Reconstruction
    // x = (q + r) * delta , r typically 0.0 (center of bin) or biased?
    // In JPEG 2000 irreversible:
    // x = (q + 0.5 * sign(q)) * delta  if q != 0
    // x = 0 if q = 0
    
    if q == 0 { return 0.0; }
    
    let sign = if q > 0 { 1.0 } else { -1.0 };
    let mag = q.abs() as f32;
    
    (mag + 0.5) * step_size * sign
}

// For 5/3 Integer, quantization is implicit (step_size = 1.0, effectively lossless if no shift)
// Usually just bit-shifts.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantization_roundtrip() {
        let val = 10.5;
        let step = 2.0;
        let q = quantize_scalar(val, step);
        // 10.5 / 2.0 = 5.25 -> 5
        assert_eq!(q, 5);
        
        let recon = dequantize_scalar(q, step);
        // (5 + 0.5) * 2.0 = 11.0
        // Deadzone quantization is lossy.
        assert!((val - recon).abs() <= step);
    }
}
