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
            x[i] += (left + right + 2) >> 2;
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
        if len == 0 { return; }
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
        for i in 0..len {
            if i % 2 == 0 {
                if l_idx < in_l.len() {
                    x[i] = in_l[l_idx];
                    l_idx += 1;
                }
            } else if h_idx < in_h.len() {
                x[i] = in_h[h_idx];
                h_idx += 1;
            }
        }

        // Reverse Update (Even samples)
        for i in (0..len).step_by(2) {
            let left = if i > 0 { x[i - 1] } else { x[i + 1] };
            let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
            x[i] -= (left + right + 2) >> 2;
        }

        // Reverse Prediction (Odd samples)
        for i in (1..len).step_by(2) {
            let left = x[i - 1];
            let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
            x[i] += (left + right) >> 1;
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
        if w == 0 || h == 0 { return; }

        let ll_w = (w + 1) / 2;
        let hl_w = w / 2;
        let ll_h = (h + 1) / 2;
        let lh_h = h / 2;

        let mut temp = vec![0i32; w * h];

        // Step 1: Vertical inverse DWT on each column
        for x in 0..ll_w {
            let mut col_l = vec![0i32; ll_h];
            let mut col_h = vec![0i32; lh_h];
            for y in 0..ll_h {
                let idx = y * ll_w + x;
                if idx < ll.len() { col_l[y] = ll[idx]; }
            }
            for y in 0..lh_h {
                let idx = y * ll_w + x; // Fixed indexing: LH has same width as LL
                if idx < lh.len() { col_h[y] = lh[idx]; }
            }
            let mut col_output = vec![0i32; h];
            Self::inverse(&col_l, &col_h, &mut col_output);
            for y in 0..h { temp[y * w + x] = col_output[y]; }
        }

        for x in 0..hl_w {
            let mut col_l = vec![0i32; ll_h];
            let mut col_h = vec![0i32; lh_h];
            for y in 0..ll_h {
                let idx = y * hl_w + x;
                if idx < hl.len() { col_l[y] = hl[idx]; }
            }
            for y in 0..lh_h {
                let idx = y * hl_w + x;
                if idx < hh.len() { col_h[y] = hh[idx]; }
            }
            let mut col_output = vec![0i32; h];
            Self::inverse(&col_l, &col_h, &mut col_output);
            for y in 0..h { temp[y * w + (ll_w + x)] = col_output[y]; }
        }

        // Step 2: Horizontal inverse DWT on each row
        for y in 0..h {
            let mut row_l = vec![0i32; ll_w];
            let mut row_h = vec![0i32; hl_w];
            for x in 0..ll_w { row_l[x] = temp[y * w + x]; }
            for x in 0..hl_w { row_h[x] = temp[y * w + ll_w + x]; }
            let mut row_output = vec![0i32; w];
            Self::inverse(&row_l, &row_h, &mut row_output);
            for x in 0..w { output[y * w + x] = row_output[x]; }
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
        if len == 0 { return; }
        if len == 1 {
            if !out_l.is_empty() { out_l[0] = signal[0]; }
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

        for i in 0..len {
            if i % 2 == 0 { x[i] *= Self::INV_K; }
            else { x[i] *= Self::K; }
        }

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
        if len == 0 { return; }
        if len == 1 {
            if !in_l.is_empty() { output[0] = in_l[0]; }
            return;
        }
        let mut x = vec![0.0f32; len];
        let mut l_idx = 0;
        let mut h_idx = 0;
        for i in 0..len {
            if i % 2 == 0 {
                if l_idx < in_l.len() { x[i] = in_l[l_idx]; l_idx += 1; }
            } else {
                if h_idx < in_h.len() { x[i] = in_h[h_idx]; h_idx += 1; }
            }
        }

        for i in 0..len {
            if i % 2 == 0 { x[i] *= Self::K; }
            else { x[i] *= Self::INV_K; }
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
        if w == 0 || h == 0 { return; }

        let ll_w = (w + 1) / 2;
        let hl_w = w / 2;
        let ll_h = (h + 1) / 2;
        let lh_h = h / 2;

        let mut temp = vec![0.0f32; w * h];

        for x in 0..ll_w {
            let mut col_l = vec![0.0f32; ll_h];
            let mut col_h = vec![0.0f32; lh_h];
            for y in 0..ll_h {
                let idx = y * ll_w + x;
                if idx < ll.len() { col_l[y] = ll[idx]; }
            }
            for y in 0..lh_h {
                let idx = y * ll_w + x;
                if idx < lh.len() { col_h[y] = lh[idx]; }
            }
            let mut col_output = vec![0.0f32; h];
            Self::inverse(&col_l, &col_h, &mut col_output);
            for y in 0..h { temp[y * w + x] = col_output[y]; }
        }

        for x in 0..hl_w {
            let mut col_l = vec![0.0f32; ll_h];
            let mut col_h = vec![0.0f32; lh_h];
            for y in 0..ll_h {
                let idx = y * hl_w + x;
                if idx < hl.len() { col_l[y] = hl[idx]; }
            }
            for y in 0..lh_h {
                let idx = y * hl_w + x;
                if idx < hh.len() { col_h[y] = hh[idx]; }
            }
            let mut col_output = vec![0.0f32; h];
            Self::inverse(&col_l, &col_h, &mut col_output);
            for y in 0..h { temp[y * w + (ll_w + x)] = col_output[y]; }
        }

        for y in 0..h {
            let mut row_l = vec![0.0f32; ll_w];
            let mut row_h = vec![0.0f32; hl_w];
            for x in 0..ll_w { row_l[x] = temp[y * w + x]; }
            for x in 0..hl_w { row_h[x] = temp[y * w + ll_w + x]; }
            let mut row_output = vec![0.0f32; w];
            Self::inverse(&row_l, &row_h, &mut row_output);
            for x in 0..w { output[y * w + x] = row_output[x]; }
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
        let l_len = (len + 1) / 2;
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
}
