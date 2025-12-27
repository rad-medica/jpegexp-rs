use std::convert::TryInto;
use std::fmt::Debug;

pub trait JpeglsSample:
    Copy
    + Clone
    + Debug
    + Default
    + PartialEq
    + PartialOrd
    + std::ops::Add<Output = Self>
    + std::ops::Sub<Output = Self>
    + From<u8>
    + TryInto<u8>
    + TryInto<i32>
{
    const BITS: u32;
    const MAX_VALUE: i32;

    fn to_i32(self) -> i32;
    fn from_i32(val: i32) -> Self;

    fn is_near(lhs: i32, rhs: i32, near: i32) -> bool {
        (lhs - rhs).abs() <= near
    }

    fn correct_prediction(predicted: i32) -> i32 {
        if predicted < 0 {
            0
        } else if predicted > Self::MAX_VALUE {
            Self::MAX_VALUE
        } else {
            predicted
        }
    }

    fn compute_reconstructed_sample(predicted: i32, error_value: i32) -> i32 {
        (predicted + error_value) & Self::MAX_VALUE
    }
}

impl JpeglsSample for u8 {
    const BITS: u32 = 8;
    const MAX_VALUE: i32 = 255;
    fn to_i32(self) -> i32 {
        self as i32
    }
    fn from_i32(val: i32) -> Self {
        val as u8
    }
}

impl JpeglsSample for u16 {
    const BITS: u32 = 16;
    const MAX_VALUE: i32 = 65535;
    fn to_i32(self) -> i32 {
        self as i32
    }
    fn from_i32(val: i32) -> Self {
        val as u16
    }
}

pub fn bit_wise_sign(i: i32) -> i32 {
    if i < 0 {
        -1
    } else if i > 0 {
        1
    } else {
        0
    }
}

pub fn apply_sign(val: i32, sign: i32) -> i32 {
    if sign < 0 { -val } else { val }
}

pub fn apply_sign_for_index(val: i32, sign: i32) -> usize {
    if sign < 0 {
        (-val) as usize
    } else {
        val as usize
    }
}
