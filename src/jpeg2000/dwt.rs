//! Discrete Wavelet Transforms for JPEG 2000

#[allow(dead_code)]
pub struct Dwt53;

impl Dwt53 {
    /// Forward 5/3 Reversible Transform (1D)
    /// Input: `signal` (spatial domain)
    /// Output: `coeffs` (interleaved Low/High pass coeffs)
    /// Note: This is an in-place implementation sketch or we can return new vec.
    /// Standard usually separates into Low (first half) and High (second half) subbands.
    pub fn forward(signal: &[i32], out_l: &mut [i32], out_h: &mut [i32]) {
        let len = signal.len();
        if len == 0 {
            return;
        }
        if len == 1 {
            out_l[0] = signal[0];
            return;
        }

        // 1. Lifting Step 1: Prediction
        // y[2n+1] = x[2n+1] - floor((x[2n] + x[2n+2])/2)
        // We need to handle extending signal.

        // Let's implement simpler buffer approach first.
        let mut x = signal.to_vec();

        // Count of low and high pass coefficients
        #[allow(clippy::manual_div_ceil)]
        let _l_count = (len + 1) / 2;
        let _h_count = len / 2;

        // Prediction (Odd samples updated based on Even samples)
        {
            let x_copy = x.clone();
            for i in 0..len {
                if i % 2 != 0 {
                    let left = x_copy[i - 1];
                    let right = if i + 1 < len {
                        x_copy[i + 1]
                    } else {
                        x_copy[i - 1]
                    };
                    x[i] -= (left + right) >> 1;
                }
            }
        }
        // Update (Even samples updated based on Odd samples)
        // y[2n] = x[2n] + floor((y[2n-1] + y[2n+1] + 2)/4)
        let x_copy = x.clone();
        for i in 0..len {
            if i % 2 == 0 {
                let left = if i > 0 { x_copy[i - 1] } else { x_copy[i + 1] };
                let right = if i + 1 < len {
                    x_copy[i + 1]
                } else {
                    x_copy[i - 1]
                };
                x[i] += (left + right + 2) >> 2;
            }
        }

        // De-interleave
        let mut l_idx = 0;
        let mut h_idx = 0;
        for i in 0..len {
            if i % 2 == 0 {
                if l_idx < out_l.len() {
                    out_l[l_idx] = x[i];
                    l_idx += 1;
                }
            } else if h_idx < out_h.len() {
                out_h[h_idx] = x[i];
                h_idx += 1;
            }
        }
    }

    /// Inverse 5/3 Reversible Transform (1D)
    pub fn inverse(in_l: &[i32], in_h: &[i32], output: &mut [i32]) {
        let len = output.len();
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

        // Reverse Update
        // x[2n] = y[2n] - floor((y[2n-1] + y[2n+1] + 2)/4)
        {
            let x_copy = x.clone();
            for i in 0..len {
                if i % 2 == 0 {
                    let left = if i > 0 { x_copy[i - 1] } else { x_copy[i + 1] };
                    let right = if i + 1 < len {
                        x_copy[i + 1]
                    } else {
                        x_copy[i - 1]
                    };
                    x[i] -= (left + right + 2) >> 2;
                }
            }
        }

        // Reverse Prediction
        // x[2n+1] = y[2n+1] + floor((x[2n] + x[2n+2])/2)
        let x_copy = x.clone();
        for i in 0..len {
            if i % 2 != 0 {
                let left = x_copy[i - 1];
                let right = if i + 1 < len {
                    x_copy[i + 1]
                } else {
                    x_copy[i - 1]
                };
                x[i] += (left + right) >> 1;
            }
        }

        output.copy_from_slice(&x);
    }

    /// Inverse 2D 5/3 Transform
    /// Reconstructs image from LL, HL, LH, HH subbands
    /// 
    /// The 2D DWT structure:
    /// ```
    /// +-------+-------+
    /// |  LL   |  HL   |  <- low-pass rows (top half)
    /// +-------+-------+
    /// |  LH   |  HH   |  <- high-pass rows (bottom half)
    /// +-------+-------+
    ///    ^       ^
    ///  low-col  high-col
    /// ```
    /// 
    /// Inverse order:
    /// 1. Vertical inverse: combine LL+LH → left cols, HL+HH → right cols
    /// 2. Horizontal inverse: combine left+right → output rows
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
        
        // Subband dimensions
        #[allow(clippy::manual_div_ceil)]
        let ll_w = (w + 1) / 2;  // LL and LH width (low-pass cols)
        let hl_w = w / 2;        // HL and HH width (high-pass cols)
        #[allow(clippy::manual_div_ceil)]
        let ll_h = (h + 1) / 2;  // LL and HL height (low-pass rows)
        let lh_h = h / 2;        // LH and HH height (high-pass rows)

        // Intermediate buffer after vertical inverse
        let mut temp = vec![0i32; w * h];

        // Step 1: Vertical inverse DWT on each column
        // For left columns (0..ll_w): combine LL and LH
        for x in 0..ll_w {
            let mut col_l = vec![0i32; ll_h];
            let mut col_h = vec![0i32; lh_h];

            // Extract LL column (low-pass vertical)
            for y in 0..ll_h {
                let idx = y * ll_w + x;
                if idx < ll.len() {
                    col_l[y] = ll[idx];
                }
            }

            // Extract LH column (high-pass vertical)
            for y in 0..lh_h {
                let idx = y * ll_w + x;
                if idx < lh.len() {
                    col_h[y] = lh[idx];
                }
            }

            let mut col_output = vec![0i32; h];
            Self::inverse(&col_l, &col_h, &mut col_output);

            // Store in left half of temp
            for y in 0..h {
                temp[y * w + x] = col_output[y];
            }
        }

        // For right columns (ll_w..w): combine HL and HH
        for x in 0..hl_w {
            let mut col_l = vec![0i32; ll_h];
            let mut col_h = vec![0i32; lh_h];

            // Extract HL column (low-pass vertical)
            for y in 0..ll_h {
                let idx = y * hl_w + x;
                if idx < hl.len() {
                    col_l[y] = hl[idx];
                }
            }

            // Extract HH column (high-pass vertical)
            for y in 0..lh_h {
                let idx = y * hl_w + x;
                if idx < hh.len() {
                    col_h[y] = hh[idx];
                }
            }

            let mut col_output = vec![0i32; h];
            Self::inverse(&col_l, &col_h, &mut col_output);

            // Store in right half of temp
            for y in 0..h {
                temp[y * w + (ll_w + x)] = col_output[y];
            }
        }

        // Step 2: Horizontal inverse DWT on each row
        for y in 0..h {
            let mut row_l = vec![0i32; ll_w];
            let mut row_h = vec![0i32; hl_w];

            // Extract left half (low-pass horizontal)
            for x in 0..ll_w {
                row_l[x] = temp[y * w + x];
            }

            // Extract right half (high-pass horizontal)
            for x in 0..hl_w {
                row_h[x] = temp[y * w + ll_w + x];
            }

            let mut row_output = vec![0i32; w];
            Self::inverse(&row_l, &row_h, &mut row_output);

            // Store in output
            for x in 0..w {
                output[y * w + x] = row_output[x];
            }
        }
    }
}

#[allow(dead_code)]
pub struct Dwt97;

impl Dwt97 {
    // 9/7 Filter Constants
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
        let mut x = signal.to_vec();

        // 1. Splitting (already done by indexing)

        // 2. Lifting Steps
        // Prediction 1
        {
            let x_copy = x.clone();
            for i in 0..len {
                if i % 2 != 0 {
                    let left = x_copy[i - 1];
                    let right = if i + 1 < len {
                        x_copy[i + 1]
                    } else {
                        x_copy[i - 1]
                    };
                    x[i] += Self::ALPHA * (left + right);
                }
            }
        }
        // Update 1
        let x_copy = x.clone();
        for i in 0..len {
            if i % 2 == 0 {
                let left = if i > 0 { x_copy[i - 1] } else { x_copy[i + 1] };
                let right = if i + 1 < len {
                    x_copy[i + 1]
                } else {
                    x_copy[i - 1]
                };
                x[i] += Self::BETA * (left + right);
            }
        }
        // Prediction 2
        let x_copy = x.clone();
        for i in 0..len {
            if i % 2 != 0 {
                let left = x_copy[i - 1];
                let right = if i + 1 < len {
                    x_copy[i + 1]
                } else {
                    x_copy[i - 1]
                };
                x[i] += Self::GAMMA * (left + right);
            }
        }
        // Update 2
        let x_copy = x.clone();
        for i in 0..len {
            if i % 2 == 0 {
                let left = if i > 0 { x_copy[i - 1] } else { x_copy[i + 1] };
                let right = if i + 1 < len {
                    x_copy[i + 1]
                } else {
                    x_copy[i - 1]
                };
                x[i] += Self::DELTA * (left + right);
            }
        }

        // Scaling
        for i in 0..len {
            if i % 2 == 0 {
                x[i] *= Self::INV_K; // Low pass
            } else {
                x[i] *= Self::K; // High pass
            }
        }

        // De-interleave
        let mut l_idx = 0;
        let mut h_idx = 0;
        for i in 0..len {
            if i % 2 == 0 {
                if l_idx < out_l.len() {
                    out_l[l_idx] = x[i];
                    l_idx += 1;
                }
            } else if h_idx < out_h.len() {
                out_h[h_idx] = x[i];
                h_idx += 1;
            }
        }
    }

    pub fn inverse(in_l: &[f32], in_h: &[f32], output: &mut [f32]) {
        let len = output.len();
        let mut x = vec![0.0f32; len];
        let mut l_idx = 0;
        let mut h_idx = 0;

        // Interleave
        for i in 0..len {
            if i % 2 == 0 {
                if l_idx < in_l.len() {
                    x[i] = in_l[l_idx];
                    l_idx += 1;
                }
            } else {
                if h_idx < in_h.len() {
                    x[i] = in_h[h_idx];
                    h_idx += 1;
                }
            }
        }

        // Inverse Scaling
        for i in 0..len {
            if i % 2 == 0 {
                x[i] *= Self::K;
            } else {
                x[i] *= Self::INV_K;
            }
        }

        // Inverse Lifting (Reverse Order, Reverse Signs)
        // Update 2
        for i in 0..len {
            if i % 2 == 0 {
                let left = if i > 0 { x[i - 1] } else { x[i + 1] };
                let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
                x[i] -= Self::DELTA * (left + right);
            }
        }
        // Prediction 2
        for i in 0..len {
            if i % 2 != 0 {
                let left = x[i - 1];
                let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
                x[i] -= Self::GAMMA * (left + right);
            }
        }
        // Update 1
        for i in 0..len {
            if i % 2 == 0 {
                let left = if i > 0 { x[i - 1] } else { x[i + 1] };
                let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
                x[i] -= Self::BETA * (left + right);
            }
        }
        // Prediction 1
        for i in 0..len {
            if i % 2 != 0 {
                let left = x[i - 1];
                let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
                x[i] -= Self::ALPHA * (left + right);
            }
        }

        output.copy_from_slice(&x);
    }

    /// Inverse 2D 9/7 Transform
    /// Reconstructs image from LL, HL, LH, HH subbands
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
        #[allow(clippy::manual_div_ceil)]
        let ll_w = (w + 1) / 2;
        let hl_w = w / 2;
        #[allow(clippy::manual_div_ceil)]
        let ll_h = (h + 1) / 2;
        let lh_h = h / 2;

        let mut temp = vec![0.0f32; w * h];

        // 1. Row Inverse Transform
        // Process Low-Vertical band (LL + HL -> L)
        for y in 0..ll_h {
            let row_ll = &ll[y * ll_w..(y + 1) * ll_w];
            let row_hl = if y * hl_w < hl.len() {
                let start = y * hl_w;
                let end = (start + hl_w).min(hl.len());
                &hl[start..end]
            } else {
                &[]
            };

            // We need full length buffers for inverse
            let mut row_l = vec![0.0f32; ll_w];
            let mut row_h = vec![0.0f32; hl_w];
            row_l[..row_ll.len()].copy_from_slice(row_ll);
            row_h[..row_hl.len()].copy_from_slice(row_hl);

            let mut row_out = vec![0.0f32; w];
            Self::inverse(&row_l, &row_h, &mut row_out);

            // Store in top half of temp
            for x in 0..w {
                temp[y * w + x] = row_out[x];
            }
        }

        // Process High-Vertical band (LH + HH -> H)
        for y in 0..lh_h {
            let row_lh = if y * ll_w < lh.len() {
                let start = y * ll_w;
                let end = (start + ll_w).min(lh.len());
                &lh[start..end]
            } else {
                &[]
            };

            let row_hh = if y * hl_w < hh.len() {
                let start = y * hl_w;
                let end = (start + hl_w).min(hh.len());
                &hh[start..end]
            } else {
                &[]
            };

            // We need full length buffers for inverse
            let mut row_l = vec![0.0f32; ll_w]; // Input L is LH (Low X)
            let mut row_h = vec![0.0f32; hl_w]; // Input H is HH (High X)
            row_l[..row_lh.len()].copy_from_slice(row_lh);
            row_h[..row_hh.len()].copy_from_slice(row_hh);

            let mut row_out = vec![0.0f32; w];
            Self::inverse(&row_l, &row_h, &mut row_out);

            // Store in bottom half of temp
            // Offset y by ll_h
            for x in 0..w {
                temp[(ll_h + y) * w + x] = row_out[x];
            }
        }

        // 2. Column Inverse Transform
        for x in 0..w {
            let mut col_l = vec![0.0f32; ll_h];
            let mut col_h = vec![0.0f32; lh_h];

            // Extract L from top half of temp
            for y in 0..ll_h {
                col_l[y] = temp[y * w + x];
            }
            // Extract H from bottom half of temp
            for y in 0..lh_h {
                col_h[y] = temp[(ll_h + y) * w + x];
            }

            let mut col_out = vec![0.0f32; h];
            Self::inverse(&col_l, &col_h, &mut col_out);

            for y in 0..h {
                output[y * w + x] = col_out[y];
            }
        }
    }
}
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_dwt_53_roundtrip() {
        let input = [10, 20, 30, 40, 50, 60, 70, 80];
        let len = input.len();
        #[allow(clippy::manual_div_ceil)]
        let l_len = (len + 1) / 2;
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
        #[allow(clippy::manual_div_ceil)]
        let l_len = (len + 1) / 2;
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
        #[allow(clippy::manual_div_ceil)]
        let l_len = (len + 1) / 2;
        let h_len = len / 2;
        let mut l = vec![0.0f32; l_len];
        let mut h = vec![0.0f32; h_len];

        Dwt97::forward(&input, &mut l, &mut h);

        let mut output = vec![0.0f32; len];
        Dwt97::inverse(&l, &h, &mut output);

        for i in 0..len {
            let diff = (input[i] - output[i]).abs();
            assert!(
                diff < 1e-4,
                "Mismatch at {}: {} vs {}",
                i,
                input[i],
                output[i]
            );
        }
    }
}
