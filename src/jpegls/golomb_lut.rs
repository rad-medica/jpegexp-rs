#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GolombCodeMatch {
    pub error_value: i16,
    pub bit_count: i8,
}

const fn countl_zero_u8(mut x: u8) -> i8 {
    if x == 0 {
        return 8;
    }
    let mut count = 0;
    while (x & 0x80) == 0 {
        x <<= 1;
        count += 1;
    }
    count
}

pub const GOLOMB_LUT: [[GolombCodeMatch; 256]; 32] = {
    let mut lut = [[GolombCodeMatch {
        error_value: 0,
        bit_count: 0,
    }; 256]; 32];
    let mut k: usize = 0;
    while k < 32 {
        let mut value: usize = 0;
        while value < 256 {
            let byte_value = value as u8;
            let unary_length = countl_zero_u8(byte_value);
            let length = unary_length + k as i8 + 1;

            if length <= 8 {
                let shift = 8 - unary_length - 1 - k as i8;
                let remainder = (value >> shift) & ((1 << k) - 1);
                let error_val = ((unary_length as i16) << k) + remainder as i16;
                lut[k][value] = GolombCodeMatch {
                    error_value: error_val,
                    bit_count: length,
                };
            }
            value += 1;
        }
        k += 1;
    }
    lut
};
