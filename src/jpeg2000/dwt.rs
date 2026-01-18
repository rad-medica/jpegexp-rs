//! Discrete Wavelet Transforms for JPEG 2000

#[allow(dead_code)]
pub struct Dwt53;

impl Dwt53 {
    /// Forward 5/3 Reversible Transform (1D)
    pub fn forward(signal: &[i32], out_l: &mut [i32], out_h: &mut [i32]) {
        let len = signal.len();
        if len == 0 {
            return;
        }
        if len == 1 {
            if !out_l.is_empty() {
                out_l[0] = signal[0];
            }
            return;
        }

        let mut x = signal.to_vec();

        // 1. Prediction (Odd samples)
        for i in (1..len).step_by(2) {
            let left = x[i - 1];
            let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
            x[i] -= (left + right) >> 1;
        }

        // 2. Update (Even samples)
        for i in (0..len).step_by(2) {
            let left = if i > 0 { x[i - 1] } else { x[i + 1] };
            let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
            // Standard (ISO 15444-1 F.4.5) uses +2 in the floor calculation: floor((a+b+2)/4)
            x[i] += (left + right + 2) >> 2;
        }

        // De-interleave
        let mut l_idx = 0;
        let mut h_idx = 0;
        for (i, val) in x.iter().enumerate().take(len) {
            if i % 2 == 0 {
                if l_idx < out_l.len() {
                    out_l[l_idx] = *val;
                    l_idx += 1;
                }
            } else if h_idx < out_h.len() {
                out_h[h_idx] = *val;
                h_idx += 1;
            }
        }
    }

    /// Inverse 5/3 Reversible Transform (1D)
    pub fn inverse(in_l: &[i32], in_h: &[i32], output: &mut [i32]) {
        let len = output.len();
        if len == 0 {
            return;
        }
        if len == 1 {
            if !in_l.is_empty() {
                output[0] = in_l[0];
            }
            return;
        }

        // Re-interleave
        let mut x = vec![0i32; len];
        let mut l_idx = 0;
        let mut h_idx = 0;
        for (i, val) in x.iter_mut().enumerate().take(len) {
            if i % 2 == 0 {
                if l_idx < in_l.len() {
                    *val = in_l[l_idx];
                    l_idx += 1;
                }
            } else if h_idx < in_h.len() {
                *val = in_h[h_idx];
                h_idx += 1;
            }
        }

        // Reverse Update (Even samples)
        // For the boundary case (last even index), the forward transform's update
        // formula differs slightly from the inverse. We need to use the correct
        // formula to match.
        for i in (0..len).step_by(2) {
            let left = if i > 0 { x[i - 1] } else { x[i + 1] };
            let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
            // The forward uses: x_even += floor((left + right + 2) / 4)
            // The inverse should undo this. Due to boundary truncation effects,
            // we use the same formula for consistency.
            x[i] -= (left + right + 2) >> 2;
        }

        // Reverse Prediction (Odd samples)
        // For the last odd index (boundary case where i+1 >= len), the forward
        // transform used x[i-1] as both neighbors due to symmetric extension.
        // Forward at boundary: h = x - floor((left + left) / 2) = x - left
        // So x = h + left = h + x[i-1]
        // This simplifies to: x[i] = in_h + in_l
        for i in (1..len).step_by(2) {
            let left = x[i - 1];
            if i + 1 < len {
                // Normal case: use both neighbors
                let right = x[i + 1];
                x[i] += (left + right) >> 1;
            } else {
                // Boundary case: x[i] = in_h + in_l = x[i] + x[i-1]
                // After reverse update at i-1 (even), x[i-1] = in_l
                // So we need: x[i] = in_h + x[i-1]
                x[i] += left;
            }
        }

        output.copy_from_slice(&x);
    }

    /// Inverse 2D 5/3 Transform
    pub fn inverse_2d(
        ll: &[i32],
        hl: &[i32],
        lh: &[i32],
        hh: &[i32],
        width: u32,
        height: u32,
        output: &mut [i32],
    ) {
        let w = width as usize;
        let h = height as usize;
        if w == 0 || h == 0 {
            return;
        }

        let ll_w = w.div_ceil(2);
        let hl_w = w / 2;
        let ll_h = h.div_ceil(2);
        let lh_h = h / 2;

        let mut temp = vec![0i32; w * h];

        // Step 1: Vertical inverse DWT on each column
        for x in 0..ll_w {
            let mut col_l = vec![0i32; ll_h];
            let mut col_h = vec![0i32; lh_h];
            for (y, val) in col_l.iter_mut().enumerate().take(ll_h) {
                let idx = y * ll_w + x;
                if idx < ll.len() {
                    *val = ll[idx];
                }
            }
            for (y, val) in col_h.iter_mut().enumerate().take(lh_h) {
                let idx = y * ll_w + x; // Fixed indexing: LH has same width as LL
                if idx < lh.len() {
                    *val = lh[idx];
                }
            }
            let mut col_output = vec![0i32; h];
            Self::inverse(&col_l, &col_h, &mut col_output);
            for y in 0..h {
                temp[y * w + x] = col_output[y];
            }
        }

        for x in 0..hl_w {
            let mut col_l = vec![0i32; ll_h];
            let mut col_h = vec![0i32; lh_h];
            for (y, val) in col_l.iter_mut().enumerate().take(ll_h) {
                let idx = y * hl_w + x;
                if idx < hl.len() {
                    *val = hl[idx];
                }
            }
            for (y, val) in col_h.iter_mut().enumerate().take(lh_h) {
                let idx = y * hl_w + x;
                if idx < hh.len() {
                    *val = hh[idx];
                }
            }
            let mut col_output = vec![0i32; h];
            Self::inverse(&col_l, &col_h, &mut col_output);
            for y in 0..h {
                temp[y * w + (ll_w + x)] = col_output[y];
            }
        }

        // Step 2: Horizontal inverse DWT on each row
        for y in 0..h {
            let mut row_l = vec![0i32; ll_w];
            let mut row_h = vec![0i32; hl_w];
            for x in 0..ll_w {
                row_l[x] = temp[y * w + x];
            }
            for x in 0..hl_w {
                row_h[x] = temp[y * w + ll_w + x];
            }
            let mut row_output = vec![0i32; w];
            Self::inverse(&row_l, &row_h, &mut row_output);
            for x in 0..w {
                output[y * w + x] = row_output[x];
            }
        }
    }
}

#[allow(dead_code)]
pub struct Dwt97;

impl Dwt97 {
    const ALPHA: f32 = -1.5861343;
    const BETA: f32 = -0.05298012;
    const GAMMA: f32 = 0.8829111;
    const DELTA: f32 = 0.44350687;
    const K: f32 = 1.2301741;
    const INV_K: f32 = 1.0 / 1.2301741;

    pub fn forward(signal: &[f32], out_l: &mut [f32], out_h: &mut [f32]) {
        let len = signal.len();
        if len == 0 {
            return;
        }
        if len == 1 {
            if !out_l.is_empty() {
                out_l[0] = signal[0];
            }
            return;
        }
        let mut x = signal.to_vec();

        for i in (1..len).step_by(2) {
            let left = x[i - 1];
            let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
            x[i] += Self::ALPHA * (left + right);
        }
        for i in (0..len).step_by(2) {
            let left = if i > 0 { x[i - 1] } else { x[i + 1] };
            let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
            x[i] += Self::BETA * (left + right);
        }
        for i in (1..len).step_by(2) {
            let left = x[i - 1];
            let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
            x[i] += Self::GAMMA * (left + right);
        }
        for i in (0..len).step_by(2) {
            let left = if i > 0 { x[i - 1] } else { x[i + 1] };
            let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
            x[i] += Self::DELTA * (left + right);
        }

        for (i, val) in x.iter_mut().enumerate().take(len) {
            if i % 2 == 0 {
                *val *= Self::INV_K;
            } else {
                *val *= Self::K;
            }
        }

        let mut l_idx = 0;
        let mut h_idx = 0;
        for (i, val) in x.iter().enumerate().take(len) {
            if i % 2 == 0 {
                if l_idx < out_l.len() {
                    out_l[l_idx] = *val;
                    l_idx += 1;
                }
            } else if h_idx < out_h.len() {
                out_h[h_idx] = *val;
                h_idx += 1;
            }
        }
    }

    pub fn inverse(in_l: &[f32], in_h: &[f32], output: &mut [f32]) {
        let len = output.len();
        if len == 0 {
            return;
        }
        if len == 1 {
            if !in_l.is_empty() {
                output[0] = in_l[0];
            }
            return;
        }
        let mut x = vec![0.0f32; len];
        let mut l_idx = 0;
        let mut h_idx = 0;
        for (i, val) in x.iter_mut().enumerate().take(len) {
            if i % 2 == 0 {
                if l_idx < in_l.len() {
                    *val = in_l[l_idx];
                    l_idx += 1;
                }
            } else if h_idx < in_h.len() {
                *val = in_h[h_idx];
                h_idx += 1;
            }
        }

        for (i, val) in x.iter_mut().enumerate().take(len) {
            if i % 2 == 0 {
                *val *= Self::K;
            } else {
                *val *= Self::INV_K;
            }
        }

        for i in (0..len).step_by(2) {
            let left = if i > 0 { x[i - 1] } else { x[i + 1] };
            let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
            x[i] -= Self::DELTA * (left + right);
        }
        for i in (1..len).step_by(2) {
            let left = x[i - 1];
            let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
            x[i] -= Self::GAMMA * (left + right);
        }
        for i in (0..len).step_by(2) {
            let left = if i > 0 { x[i - 1] } else { x[i + 1] };
            let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
            x[i] -= Self::BETA * (left + right);
        }
        for i in (1..len).step_by(2) {
            let left = x[i - 1];
            let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
            x[i] -= Self::ALPHA * (left + right);
        }

        output.copy_from_slice(&x);
    }

    pub fn inverse_2d(
        ll: &[f32],
        hl: &[f32],
        lh: &[f32],
        hh: &[f32],
        width: u32,
        height: u32,
        output: &mut [f32],
    ) {
        let w = width as usize;
        let h = height as usize;
        if w == 0 || h == 0 {
            return;
        }

        let ll_w = w.div_ceil(2);
        let hl_w = w / 2;
        let ll_h = h.div_ceil(2);
        let lh_h = h / 2;

        let mut temp = vec![0.0f32; w * h];

        for x in 0..ll_w {
            let mut col_l = vec![0.0f32; ll_h];
            let mut col_h = vec![0.0f32; lh_h];
            for (y, val) in col_l.iter_mut().enumerate().take(ll_h) {
                let idx = y * ll_w + x;
                if idx < ll.len() {
                    *val = ll[idx];
                }
            }
            for (y, val) in col_h.iter_mut().enumerate().take(lh_h) {
                let idx = y * ll_w + x;
                if idx < lh.len() {
                    *val = lh[idx];
                }
            }
            let mut col_output = vec![0.0f32; h];
            Self::inverse(&col_l, &col_h, &mut col_output);
            for y in 0..h {
                temp[y * w + x] = col_output[y];
            }
        }

        for x in 0..hl_w {
            let mut col_l = vec![0.0f32; ll_h];
            let mut col_h = vec![0.0f32; lh_h];
            for (y, val) in col_l.iter_mut().enumerate().take(ll_h) {
                let idx = y * hl_w + x;
                if idx < hl.len() {
                    *val = hl[idx];
                }
            }
            for (y, val) in col_h.iter_mut().enumerate().take(lh_h) {
                let idx = y * hl_w + x;
                if idx < hh.len() {
                    *val = hh[idx];
                }
            }
            let mut col_output = vec![0.0f32; h];
            Self::inverse(&col_l, &col_h, &mut col_output);
            for y in 0..h {
                temp[y * w + (ll_w + x)] = col_output[y];
            }
        }

        for y in 0..h {
            let mut row_l = vec![0.0f32; ll_w];
            let mut row_h = vec![0.0f32; hl_w];
            for x in 0..ll_w {
                row_l[x] = temp[y * w + x];
            }
            for x in 0..hl_w {
                row_h[x] = temp[y * w + ll_w + x];
            }
            let mut row_output = vec![0.0f32; w];
            Self::inverse(&row_l, &row_h, &mut row_output);
            for x in 0..w {
                output[y * w + x] = row_output[x];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dwt_53_roundtrip() {
        let input = [10, 20, 30, 40, 50, 60, 70, 80];
        let len = input.len();
        let l_len = len.div_ceil(2);
        let h_len = len / 2;
        let mut l = vec![0i32; l_len];
        let mut h = vec![0i32; h_len];
        Dwt53::forward(&input, &mut l, &mut h);
        let mut output = vec![0i32; len];
        Dwt53::inverse(&l, &h, &mut output);
        assert_eq!(input.to_vec(), output);
    }

    #[test]
    fn test_dwt_53_odd_length() {
        let input = [10, 20, 30, 40, 50];
        let len = input.len();
        let l_len = len.div_ceil(2);
        let h_len = len / 2;
        let mut l = vec![0i32; l_len];
        let mut h = vec![0i32; h_len];
        Dwt53::forward(&input, &mut l, &mut h);
        let mut output = vec![0i32; len];
        Dwt53::inverse(&l, &h, &mut output);
        assert_eq!(input.to_vec(), output);
    }

    #[test]
    fn test_dwt_97_roundtrip() {
        let input = [10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0];
        let len = input.len();
        let l_len = len.div_ceil(2);
        let h_len = len / 2;
        let mut l = vec![0.0f32; l_len];
        let mut h = vec![0.0f32; h_len];
        Dwt97::forward(&input, &mut l, &mut h);
        let mut output = vec![0.0f32; len];
        Dwt97::inverse(&l, &h, &mut output);
        for i in 0..len {
            let diff = (input[i] - output[i]).abs();
            assert!(diff < 1e-4);
        }
    }

    #[test]
    fn test_dwt_53_2d_with_constant_hh() {
        // This tests the exact scenario from 12-bit checkerboard:
        // LL=0, HL=0, LH=0, HH=constant (8190)
        // This is what happens after DWT of a perfect checkerboard

        // Start with 8x8 for simplicity
        let width = 8u32;
        let height = 8u32;

        // After one level of DWT, we have 4x4 subbands
        let ll_size = 4 * 4;
        let hl_size = 4 * 4;
        let lh_size = 4 * 4;
        let hh_size = 4 * 4;

        let ll = vec![0i32; ll_size]; // All zero
        let hl = vec![0i32; hl_size]; // All zero
        let lh = vec![0i32; lh_size]; // All zero
        let hh = vec![8190i32; hh_size]; // Constant 8190

        let mut output = vec![0i32; (width * height) as usize];

        // Run IDWT
        Dwt53::inverse_2d(&ll, &hl, &lh, &hh, width, height, &mut output);

        // Check what we get
        let mut unique_vals: Vec<i32> = output.clone();
        unique_vals.sort_unstable();
        unique_vals.dedup();

        println!("IDWT with LL=0, HL=0, LH=0, HH=8190:");
        println!("Output size: {}", output.len());
        println!("Unique values: {:?}", unique_vals);
        println!("First 16 values: {:?}", &output[..16]);

        // Check if output is constant (which would be wrong)
        let is_constant = unique_vals.len() == 1;
        let min_val = output.iter().min().unwrap();
        let max_val = output.iter().max().unwrap();

        println!(
            "Min: {}, Max: {}, Is constant: {}",
            min_val,
            max_val,
            is_constant
        );

        // IDWT should produce varying output, not constant!
        // With HH=8190 (high frequency), we expect significant variation
        assert!(
            !is_constant,
            "IDWT output should not be constant when HH has data"
        );

        // The range should be significant (at least 1000)
        let range = max_val - min_val;
        assert!(
            range > 1000,
            "IDWT should produce significant variation, got range={}",
            range
        );
    }

    #[test]
    fn test_dwt_53_checkerboard_roundtrip() {
        // Test a full checkerboard roundtrip to understand what's happening
        // 8x8 checkerboard after level shift: -2048, 2047, -2048, 2047...
        let width = 8;
        let height = 8;
        let mut input = vec![0i32; width * height];

        for y in 0..height {
            for x in 0..width {
                let val = if (x + y) % 2 == 0 { -2048 } else { 2047 };
                input[y * width + x] = val;
            }
        }

        println!("Input checkerboard (8x8, after level shift):");
        for y in 0..4 {
            println!("  {:?}", &input[y * width..(y * width + 8)]);
        }

        // Apply forward DWT 2D
        let mut coeffs = input.clone();

        // Apply 1D DWT to rows
        for y in 0..height {
            let row: Vec<i32> = coeffs[y * width..(y + 1) * width].to_vec();
            let l_len = width.div_ceil(2);
            let h_len = width / 2;
            let mut l = vec![0i32; l_len];
            let mut h = vec![0i32; h_len];
            Dwt53::forward(&row, &mut l, &mut h);
            for (i, &v) in l.iter().enumerate() {
                coeffs[y * width + i] = v;
            }
            for (i, &v) in h.iter().enumerate() {
                coeffs[y * width + l_len + i] = v;
            }
        }

        // Apply 1D DWT to columns
        for x in 0..width {
            let col: Vec<i32> = (0..height).map(|y| coeffs[y * width + x]).collect();
            let l_len = height.div_ceil(2);
            let h_len = height / 2;
            let mut l = vec![0i32; l_len];
            let mut h = vec![0i32; h_len];
            Dwt53::forward(&col, &mut l, &mut h);
            for (i, &v) in l.iter().enumerate() {
                coeffs[i * width + x] = v;
            }
            for (i, &v) in h.iter().enumerate() {
                coeffs[(l_len + i) * width + x] = v;
            }
        }

        println!("\nAfter forward DWT (coefficients):");
        for y in 0..4 {
            println!("  {:?}", &coeffs[y * width..(y * width + 8)]);
        }

        // Extract subbands
        let ll_w = 4;
        let ll_h = 4;
        let mut ll = vec![0i32; ll_w * ll_h];
        let mut hl = vec![0i32; ll_w * ll_h];
        let mut lh = vec![0i32; ll_w * ll_h];
        let mut hh = vec![0i32; ll_w * ll_h];

        for y in 0..ll_h {
            for x in 0..ll_w {
                ll[y * ll_w + x] = coeffs[y * width + x];
                hl[y * ll_w + x] = coeffs[y * width + ll_w + x];
                lh[y * ll_w + x] = coeffs[(ll_h + y) * width + x];
                hh[y * ll_w + x] = coeffs[(ll_h + y) * width + ll_w + x];
            }
        }

        println!("\nLL subband (4x4): {:?}", ll);
        println!("HL subband (4x4): {:?}", hl);
        println!("LH subband (4x4): {:?}", lh);
        println!("HH subband (4x4): {:?}", hh);

        // Now apply inverse DWT
        let mut reconstructed = vec![0i32; width * height];
        Dwt53::inverse_2d(
            &ll,
            &hl,
            &lh,
            &hh,
            width as u32,
            height as u32,
            &mut reconstructed,
        );

        println!("\nAfter inverse DWT (reconstructed):");
        for y in 0..4 {
            println!("  {:?}", &reconstructed[y * width..(y * width + 8)]);
        }

        // Check reconstruction error
        let mut max_error = 0i32;
        for i in 0..input.len() {
            let error = (input[i] - reconstructed[i]).abs();
            max_error = max_error.max(error);
        }

        println!("\nMax reconstruction error: {}", max_error);
        assert_eq!(max_error, 0, "DWT should be perfectly reversible");
    }
}
