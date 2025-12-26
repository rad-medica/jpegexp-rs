use crate::constants::DEFAULT_RESET_THRESHOLD;
use crate::{ColorTransformation, InterleaveMode};
use std::cmp::{max, min};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodingParameters {
    pub near_lossless: i32,
    pub restart_interval: u32,
    pub interleave_mode: InterleaveMode,
    pub transformation: ColorTransformation,
    pub limit: i32,
    pub quantized_bits_per_sample: i32,
    pub mapping_table_id: i32,
}

impl Default for CodingParameters {
    fn default() -> Self {
        Self {
            near_lossless: 0,
            restart_interval: 0,
            interleave_mode: InterleaveMode::None,
            transformation: ColorTransformation::None,
            limit: 256,
            quantized_bits_per_sample: 8,
            mapping_table_id: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct JpeglsPcParameters {
    pub maximum_sample_value: i32,
    pub threshold1: i32,
    pub threshold2: i32,
    pub threshold3: i32,
    pub reset_value: i32,
}

// Clamping function as defined by ISO/IEC 14495-1, Figure C.3
const fn clamp(i: i32, j: i32, maximum_sample_value: i32) -> i32 {
    if i > maximum_sample_value || i < j {
        j
    } else {
        i
    }
}

pub fn compute_maximum_near_lossless(maximum_sample_value: i32) -> i32 {
    debug_assert!(maximum_sample_value >= 1);
    min(255, maximum_sample_value / 2)
}

// Default coding threshold values as defined by ISO/IEC 14495-1, C.2.4.1.1.1
pub fn compute_default(maximum_sample_value: i32, near_lossless: i32) -> JpeglsPcParameters {
    debug_assert!(maximum_sample_value <= u16::MAX as i32);
    debug_assert!(near_lossless >= 0 && near_lossless <= compute_maximum_near_lossless(maximum_sample_value));

    // Default threshold values for JPEG-LS statistical modeling as defined in ISO/IEC 14495-1, table C.3
    // for the case MAXVAL = 255 and NEAR = 0.
    const DEFAULT_THRESHOLD1: i32 = 3;  // BASIC_T1
    const DEFAULT_THRESHOLD2: i32 = 7;  // BASIC_T2
    const DEFAULT_THRESHOLD3: i32 = 21; // BASIC_T3

    if maximum_sample_value >= 128 {
        let factor = (min(maximum_sample_value, 4095) + 128) / 256;
        let threshold1 = clamp(
            factor * (DEFAULT_THRESHOLD1 - 2) + 2 + 3 * near_lossless,
            near_lossless + 1,
            maximum_sample_value,
        );
        let threshold2 = clamp(
            factor * (DEFAULT_THRESHOLD2 - 3) + 3 + 5 * near_lossless,
            threshold1,
            maximum_sample_value,
        );

        JpeglsPcParameters {
            maximum_sample_value,
            threshold1,
            threshold2,
            threshold3: clamp(
                factor * (DEFAULT_THRESHOLD3 - 4) + 4 + 7 * near_lossless,
                threshold2,
                maximum_sample_value,
            ),
            reset_value: DEFAULT_RESET_THRESHOLD,
        }
    } else {
        let factor = 256 / (maximum_sample_value + 1);
        let threshold1 = clamp(
            max(2, DEFAULT_THRESHOLD1 / factor + 3 * near_lossless),
            near_lossless + 1,
            maximum_sample_value,
        );
        let threshold2 = clamp(
            max(3, DEFAULT_THRESHOLD2 / factor + 5 * near_lossless),
            threshold1,
            maximum_sample_value,
        );

        JpeglsPcParameters {
            maximum_sample_value,
            threshold1,
            threshold2,
            threshold3: clamp(
                max(4, DEFAULT_THRESHOLD3 / factor + 7 * near_lossless),
                threshold2,
                maximum_sample_value,
            ),
            reset_value: DEFAULT_RESET_THRESHOLD,
        }
    }
}

pub fn is_default(preset_coding_parameters: &JpeglsPcParameters, defaults: &JpeglsPcParameters) -> bool {
    if preset_coding_parameters.maximum_sample_value == 0
        && preset_coding_parameters.threshold1 == 0
        && preset_coding_parameters.threshold2 == 0
        && preset_coding_parameters.threshold3 == 0
        && preset_coding_parameters.reset_value == 0
    {
        return true;
    }

    if preset_coding_parameters.maximum_sample_value != defaults.maximum_sample_value {
        return false;
    }

    if preset_coding_parameters.threshold1 != defaults.threshold1 {
        return false;
    }

    if preset_coding_parameters.threshold2 != defaults.threshold2 {
        return false;
    }

    if preset_coding_parameters.threshold3 != defaults.threshold3 {
        return false;
    }

    if preset_coding_parameters.reset_value != defaults.reset_value {
        return false;
    }

    true
}

pub fn is_valid(
    pc_parameters: &JpeglsPcParameters,
    maximum_component_value: i32,
    near_lossless: i32,
) -> Result<JpeglsPcParameters, ()> {
    debug_assert!(maximum_component_value >= 3 && maximum_component_value <= u16::MAX as i32);

    // ISO/IEC 14495-1, C.2.4.1.1, Table C.1 defines the valid JPEG-LS preset coding parameters values.
    if pc_parameters.maximum_sample_value != 0
        && (pc_parameters.maximum_sample_value < 1 || pc_parameters.maximum_sample_value > maximum_component_value)
    {
        return Err(());
    }

    let maximum_sample_value = if pc_parameters.maximum_sample_value != 0 {
        pc_parameters.maximum_sample_value
    } else {
        maximum_component_value
    };

    if pc_parameters.threshold1 != 0
        && (pc_parameters.threshold1 < near_lossless + 1 || pc_parameters.threshold1 > maximum_sample_value)
    {
        return Err(());
    }

    let defaults = compute_default(maximum_sample_value, near_lossless);

    let threshold1 = if pc_parameters.threshold1 != 0 {
        pc_parameters.threshold1
    } else {
        defaults.threshold1
    };

    if pc_parameters.threshold2 != 0
        && (pc_parameters.threshold2 < threshold1 || pc_parameters.threshold2 > maximum_sample_value)
    {
        return Err(());
    }

    let threshold2 = if pc_parameters.threshold2 != 0 {
        pc_parameters.threshold2
    } else {
        defaults.threshold2
    };

    if pc_parameters.threshold3 != 0
        && (pc_parameters.threshold3 < threshold2 || pc_parameters.threshold3 > maximum_sample_value)
    {
        return Err(());
    }

    if pc_parameters.reset_value != 0
        && (pc_parameters.reset_value < 3 || pc_parameters.reset_value > max(255, maximum_sample_value))
    {
        return Err(());
    }

    let validated_parameters = JpeglsPcParameters {
        maximum_sample_value,
        threshold1,
        threshold2,
        threshold3: if pc_parameters.threshold3 != 0 {
            pc_parameters.threshold3
        } else {
            defaults.threshold3
        },
        reset_value: if pc_parameters.reset_value != 0 {
            pc_parameters.reset_value
        } else {
            defaults.reset_value
        },
    };

    Ok(validated_parameters)
}

pub fn compute_limit_parameter(bits_per_sample: i32, _near_lossless: i32, _component_count: i32) -> i32 {
    2 * (bits_per_sample + std::cmp::max(8, bits_per_sample))
}
