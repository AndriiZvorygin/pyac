//! `exp` on f32 and f64, in the subsets the runtime can evaluate.
//!
//! These are the only transcendental functions the vector verbs need, kept apart
//! from the verbs themselves so the numeric paths read on their own.

use super::super::ERROR_FLOAT_DOMAIN;
use super::super::float::is_finite_f32;
#[cfg(feature = "shader-f64")]
use super::super::float::is_finite_f64;

pub(super) fn exp_f32(value: f32) -> Result<f32, u32> {
    if value.to_bits() == 0xff80_0000 || value < -104.0 {
        return Ok(0.0);
    }
    if !is_finite_f32(value) || value > 88.0 {
        return Err(ERROR_FLOAT_DOMAIN);
    }
    let scaled = value * 1.442_695_1_f32;
    let mut exponent = scaled as i32;
    if exponent as f32 > scaled {
        exponent -= 1;
    }
    let reduced = value - exponent as f32 * 0.693_147_2_f32;
    let coefficients = [
        1.0_f32,
        1.0,
        0.5,
        0.166_666_67,
        0.041_666_668,
        0.008_333_334,
        0.001_388_888_9,
        0.000_198_412_7,
        0.000_024_801_588,
        0.000_002_755_731_9,
    ];
    let mut polynomial = coefficients[9];
    let mut index = 9;
    while index > 0 {
        index -= 1;
        polynomial = array_at!(coefficients, index) + reduced * polynomial;
    }
    let scale = if exponent >= -126 {
        f32::from_bits(((exponent + 127) as u32) << 23)
    } else if exponent >= -149 {
        f32::from_bits(1u32 << (exponent + 149))
    } else {
        0.0
    };
    let output = polynomial * scale;
    if !is_finite_f32(output) {
        return Err(ERROR_FLOAT_DOMAIN);
    }
    Ok(output)
}

#[cfg(feature = "shader-f64")]
pub(super) fn exp_f64(value: f64) -> Result<f64, u32> {
    if value.to_bits() == 0xfff0_0000_0000_0000 || value < -746.0 {
        return Ok(0.0);
    }
    if !is_finite_f64(value) || value > 709.0 {
        return Err(ERROR_FLOAT_DOMAIN);
    }
    let scaled = value * 1.442_695_040_888_963_4_f64;
    let mut exponent = scaled as i32;
    if exponent as f64 > scaled {
        exponent -= 1;
    }
    let reduced = value - exponent as f64 * 0.693_147_180_559_945_3_f64;
    let coefficients = [
        1.0_f64,
        1.0,
        0.5,
        0.166_666_666_666_666_66,
        0.041_666_666_666_666_664,
        0.008_333_333_333_333_333,
        0.001_388_888_888_888_889,
        0.000_198_412_698_412_698_4,
        0.000_024_801_587_301_587_3,
        0.000_002_755_731_922_398_589_3,
        0.000_000_275_573_192_239_858_9,
        0.000_000_025_052_108_385_441_72,
    ];
    let mut polynomial = coefficients[11];
    let mut index = 11;
    while index > 0 {
        index -= 1;
        polynomial = array_at!(coefficients, index) + reduced * polynomial;
    }
    let scale = if exponent >= -1022 {
        f64::from_bits(((exponent + 1023) as u64) << 52)
    } else if exponent >= -1074 {
        f64::from_bits(1u64 << (exponent + 1074))
    } else {
        0.0
    };
    let output = polynomial * scale;
    if !is_finite_f64(output) {
        return Err(ERROR_FLOAT_DOMAIN);
    }
    Ok(output)
}
