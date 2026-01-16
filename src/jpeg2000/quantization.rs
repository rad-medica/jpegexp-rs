pub fn quantize_scalar(coeff: f32, step_size: f32) -> i32 {
    // Dead-zone scalar quantization
    // q = sign(x) * floor(|x| / delta)
    if step_size <= 0.0 {
        return coeff as i32;
    } // Should not happen

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

    if q == 0 {
        return 0.0;
    }

    let sign = if q > 0 { 1.0 } else { -1.0 };
    let mag = q.abs() as f32;

    (mag + 0.5) * step_size * sign
}

/// Calculate step size from encoded QCD value
pub fn decode_step_size(encoded: u16, _guard_bits: u8) -> f32 {
    let exponent = (encoded >> 11) & 0x1F;
    let mantissa = encoded & 0x7FF;

    // Δ_b = 2^(R_b - ε_b) * (1 + m_b/2^11)
    // where R_b = depth + guard_bits + gain
    // For LL: gain = 0, for others: gain = log2(2) = 1 for HL/LH, log2(2*sqrt(2)) ≈ 1.5 for HH

    
    (1.0 + (mantissa as f32) / 2048.0) * 2.0f32.powi(-(exponent as i32))
}

/// Calculate step sizes for all subbands
pub fn calculate_step_sizes(qcd: &super::image::J2kQcd, decomposition_levels: u8) -> Vec<f32> {
    let num_subbands = 1 + 3 * decomposition_levels as usize;
    let mut step_sizes = Vec::with_capacity(num_subbands);

    let quant_style = qcd.quant_style & 0x1F; // Lower 5 bits
    let guard_bits = (qcd.quant_style >> 5) & 0x07; // Bits 5-7

    match quant_style {
        0 => {
            // No quantization (reversible)
            vec![1.0; num_subbands]
        }
        1 => {
            // Derived quantization
            if qcd.step_sizes.is_empty() {
                return vec![1.0; num_subbands];
            }

            // Base step size from first entry
            let base_encoded = qcd.step_sizes[0];
            let base_step = decode_step_size(base_encoded, guard_bits);

            // For derived quantization, step sizes are derived for each resolution level
            for level in 0..=decomposition_levels {
                if level == 0 {
                    // LL subband at lowest resolution
                    step_sizes.push(base_step);
                } else {
                    // HL, LH, HH subbands
                    // Each higher resolution level has step sizes derived by multiplying by 2^(level-1)
                    let factor = 2.0f32.powi(level as i32 - 1);
                    step_sizes.push(base_step * factor); // HL
                    step_sizes.push(base_step * factor); // LH
                    step_sizes.push(base_step * factor * 2.0f32.sqrt()); // HH (additional gain)
                }
            }
            step_sizes
        }
        2 => {
            // Expounded quantization - explicit step sizes for each subband
            for &encoded in &qcd.step_sizes {
                step_sizes.push(decode_step_size(encoded, guard_bits));
            }

            // If we don't have enough step sizes, pad with 1.0
            while step_sizes.len() < num_subbands {
                step_sizes.push(1.0);
            }

            step_sizes.truncate(num_subbands);
            step_sizes
        }
        _ => {
            // Unknown style, default to no quantization
            vec![1.0; num_subbands]
        }
    }
}

/// Quantize DWT coefficients using proper JPEG2000 quantization
pub fn quantize_coefficients(
    coeffs: &mut [f32],
    width: usize,
    height: usize,
    qcd: &super::image::J2kQcd,
    decomposition_levels: u8,
) {
    if qcd.quant_style == 0 {
        // No quantization (reversible) - convert back to i32
        for coeff in coeffs.iter_mut() {
            *coeff = coeff.round();
        }
        return;
    }

    let step_sizes = calculate_step_sizes(qcd, decomposition_levels);

    // Apply quantization to each subband
    // The coefficients are arranged as: LL | HL | LH | HH | HL | LH | HH | ...
    // where each group of 3 subbands (HL,LH,HH) corresponds to one resolution level

    let mut subband_idx = 0;
    let mut offset = 0;

    // Start with LL at lowest resolution
    let mut current_width = width >> decomposition_levels;
    let mut current_height = height >> decomposition_levels;
    current_width = current_width.max(1);
    current_height = current_height.max(1);

    let ll_size = current_width * current_height;
    for i in 0..ll_size {
        coeffs[offset + i] = quantize_scalar(coeffs[offset + i], step_sizes[subband_idx]) as f32;
    }
    offset += ll_size;
    subband_idx += 1;

    // Process each resolution level from coarse to fine
    for level in (1..=decomposition_levels).rev() {
        let level_width = width >> (decomposition_levels - level);
        let level_height = height >> (decomposition_levels - level);
        let level_width = level_width.max(1);
        let level_height = level_height.max(1);

        let subband_width = level_width / 2;
        let subband_height = level_height / 2;
        let subband_size = subband_width * subband_height;

        // HL subband
        for i in 0..subband_size {
            coeffs[offset + i] =
                quantize_scalar(coeffs[offset + i], step_sizes[subband_idx]) as f32;
        }
        offset += subband_size;
        subband_idx += 1;

        // LH subband
        for i in 0..subband_size {
            coeffs[offset + i] =
                quantize_scalar(coeffs[offset + i], step_sizes[subband_idx]) as f32;
        }
        offset += subband_size;
        subband_idx += 1;

        // HH subband
        for i in 0..subband_size {
            coeffs[offset + i] =
                quantize_scalar(coeffs[offset + i], step_sizes[subband_idx]) as f32;
        }
        offset += subband_size;
        subband_idx += 1;
    }
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
