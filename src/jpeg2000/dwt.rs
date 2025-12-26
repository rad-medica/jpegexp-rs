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
        if len == 0 { return; }
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
        let l_count = (len + 1) / 2;
        let _h_count = len / 2;

        // Prediction (Odd samples updated based on Even samples)
        for i in 0..len {
            if i % 2 != 0 {
                let left = x[i - 1];
                let right = if i + 1 < len { x[i + 1] } else { x[i - 1] }; // Symmetric extension
                x[i] -= (left + right) >> 1;
            }
        }

        // Update (Even samples updated based on Odd samples)
        // y[2n] = x[2n] + floor((y[2n-1] + y[2n+1] + 2)/4)
        for i in 0..len {
             if i % 2 == 0 {
                let left = if i > 0 { x[i - 1] } else { x[i + 1] }; // Symmetric extension
                let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
                x[i] += (left + right + 2) >> 2;
             }
        }

        // De-interleave
        let mut l_idx = 0;
        let mut h_idx = 0;
        for i in 0..len {
            if i % 2 == 0 {
                if l_idx < out_l.len() { out_l[l_idx] = x[i]; l_idx += 1; }
            } else {
                if h_idx < out_h.len() { out_h[h_idx] = x[i]; h_idx += 1; }
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
        for i in 0..len {
            if i % 2 == 0 {
                 if l_idx < in_l.len() { x[i] = in_l[l_idx]; l_idx += 1; }
            } else {
                 if h_idx < in_h.len() { x[i] = in_h[h_idx]; h_idx += 1; }
            }
        }

        // Reverse Update
        // x[2n] = y[2n] - floor((y[2n-1] + y[2n+1] + 2)/4)
        for i in 0..len {
            if i % 2 == 0 {
                let left = if i > 0 { x[i - 1] } else { x[i + 1] };
                let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
                x[i] -= (left + right + 2) >> 2;
            }
        }

        // Reverse Prediction
        // x[2n+1] = y[2n+1] + floor((x[2n] + x[2n+2])/2)
        for i in 0..len {
            if i % 2 != 0 {
                let left = x[i - 1];
                let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
                x[i] += (left + right) >> 1;
            }
        }
        
        output.copy_from_slice(&x);
    }
}

#[allow(dead_code)]
pub struct Dwt97;

impl Dwt97 {
    // 9/7 Filter Constants
    const ALPHA: f32 = -1.586134342;
    const BETA: f32 = -0.052980118;
    const GAMMA: f32 = 0.882911075;
    const DELTA: f32 = 0.443506852;
    const K: f32 = 1.230174105;
    const INV_K: f32 = 1.0 / 1.230174105;

    pub fn forward(signal: &[f32], out_l: &mut [f32], out_h: &mut [f32]) {
        let len = signal.len();
        if len == 0 { return; }
        let mut x = signal.to_vec();

        // 1. Splitting (already done by indexing)
        
        // 2. Lifting Steps
        // Prediction 1
        for i in 0..len {
            if i % 2 != 0 {
                let left = x[i - 1];
                let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
                x[i] += Self::ALPHA * (left + right);
            }
        }
        // Update 1
        for i in 0..len {
            if i % 2 == 0 {
                let left = if i > 0 { x[i - 1] } else { x[i + 1] }; 
                let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
                x[i] += Self::BETA * (left + right);
            }
        }
        // Prediction 2
        for i in 0..len {
            if i % 2 != 0 {
                let left = x[i - 1];
                let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
                x[i] += Self::GAMMA * (left + right);
            }
        }
        // Update 2
        for i in 0..len {
            if i % 2 == 0 {
                let left = if i > 0 { x[i - 1] } else { x[i + 1] }; 
                let right = if i + 1 < len { x[i + 1] } else { x[i - 1] };
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
                if l_idx < out_l.len() { out_l[l_idx] = x[i]; l_idx += 1; }
            } else {
                if h_idx < out_h.len() { out_h[h_idx] = x[i]; h_idx += 1; }
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
                 if l_idx < in_l.len() { x[i] = in_l[l_idx]; l_idx += 1; }
            } else {
                 if h_idx < in_h.len() { x[i] = in_h[h_idx]; h_idx += 1; }
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
}
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
            assert!(diff < 1e-4, "Mismatch at {}: {} vs {}", i, input[i], output[i]);
        }
    }
}
