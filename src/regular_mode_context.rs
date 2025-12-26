use crate::error::JpeglsError;
use crate::traits::bit_wise_sign;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegularModeContext {
    a: i32,
    b: i32,
    c: i32,
    n: i32,
}

impl RegularModeContext {
    pub fn new(range: i32) -> Self {
        Self {
            a: Self::initialization_value_for_a(range),
            b: 0,
            c: 0,
            n: 1,
        }
    }

    pub fn c(&self) -> i32 {
        self.c
    }

    pub fn get_error_correction(&self, k: i32) -> i32 {
        if k != 0 {
            return 0;
        }
        bit_wise_sign(2 * self.b + self.n - 1)
    }

    pub fn update_variables_and_bias(
        &mut self,
        error_value: i32,
        near_lossless: i32,
        reset_threshold: i32,
    ) -> Result<(), JpeglsError> {
        debug_assert!(self.n != 0);

        self.a += error_value.abs();
        self.b += error_value * (2 * near_lossless + 1);

        if self.a >= 65536 * 256 || self.b.abs() >= 65536 * 256 {
            return Err(JpeglsError::InvalidData);
        }

        if self.n == reset_threshold {
            self.a >>= 1;
            self.b >>= 1;
            self.n >>= 1;
        }

        self.n += 1;
        debug_assert!(self.n != 0);

        // Code segment A.13
        const MAX_C: i32 = 127;
        const MIN_C: i32 = -128;

        if self.b + self.n <= 0 {
            self.b += self.n;
            if self.b <= -self.n {
                self.b = -self.n + 1;
            }
            if self.c > MIN_C {
                self.c -= 1;
            }
        } else if self.b > 0 {
            self.b -= self.n;
            if self.b > 0 {
                self.b = 0;
            }
            if self.c < MAX_C {
                self.c += 1;
            }
        }
        Ok(())
    }

    pub fn compute_golomb_coding_parameter(&self, max_k_value: i32) -> Result<i32, JpeglsError> {
        let mut k = 0;
        while (self.n << k) < self.a && k < max_k_value {
            k += 1;
        }

        if k == max_k_value {
            return Err(JpeglsError::InvalidData);
        }
        Ok(k)
    }

    fn initialization_value_for_a(range: i32) -> i32 {
        std::cmp::max(2, (range + 32) / 64)
    }
}
