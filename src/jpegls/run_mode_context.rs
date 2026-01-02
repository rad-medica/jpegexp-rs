use crate::error::JpeglsError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunModeContext {
    run_interruption_type: i32,
    a: i32,
    n: i32,
    nn: i32,
}

impl RunModeContext {
    pub fn new(run_interruption_type: i32, range: i32) -> Self {
        Self {
            run_interruption_type,
            a: Self::initialization_value_for_a(range),
            n: 1,
            nn: 0,
        }
    }

    pub fn run_interruption_type(&self) -> i32 {
        self.run_interruption_type
    }

    pub fn a(&self) -> i32 {
        self.a
    }

    pub fn n(&self) -> i32 {
        self.n
    }

    pub fn nn(&self) -> i32 {
        self.nn
    }

    pub fn compute_golomb_coding_parameter_checked(&self) -> Result<i32, JpeglsError> {
        let temp = self.a + (self.n >> 1) * self.run_interruption_type;
        let mut n_test = self.n;
        let mut k = 0;

        while n_test < temp {
            n_test <<= 1;
            k += 1;
            if k > 32 {
                return Err(JpeglsError::InvalidData);
            }
        }
        Ok(k)
    }

    pub fn compute_golomb_coding_parameter(&self) -> i32 {
        let temp = self.a + (self.n >> 1) * self.run_interruption_type;
        let mut n_test = self.n;
        let mut k = 0;

        while n_test < temp {
            n_test <<= 1;
            k += 1;
            debug_assert!(k <= 32);
        }
        k
    }

    // Code segment A.23
    pub fn update_variables(
        &mut self,
        error_value: i32,
        e_mapped_error_value: i32,
        reset_threshold: i32,
    ) {
        if error_value < 0 {
            self.nn += 1;
        }

        self.a += (e_mapped_error_value + 1 - self.run_interruption_type) >> 1;

        if self.n == reset_threshold {
            self.a >>= 1;
            self.n >>= 1;
            self.nn >>= 1;
        }

        self.n += 1;
    }

    pub fn decode_error_value(&self, temp: i32, k: i32) -> i32 {
        let map = (temp & 1) != 0;
        let error_value_abs = (temp + (map as i32)) / 2;

        if (k != 0 || (2 * self.nn >= self.n)) == map {
            debug_assert!(map == self.compute_map(-error_value_abs, k));
            -error_value_abs
        } else {
            debug_assert!(map == self.compute_map(error_value_abs, k));
            error_value_abs
        }
    }

    // Code segment A.21
    pub fn compute_map(&self, error_value: i32, k: i32) -> bool {
        if k == 0 && error_value > 0 && 2 * self.nn < self.n {
            return true;
        }

        if error_value < 0 && 2 * self.nn >= self.n {
            return true;
        }

        if error_value < 0 && k != 0 {
            return true;
        }

        false
    }

    fn initialization_value_for_a(range: i32) -> i32 {
        std::cmp::max(2, (range + 32) / 64)
    }
}
