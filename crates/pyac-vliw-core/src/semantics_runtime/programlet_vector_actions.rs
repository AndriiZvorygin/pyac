use super::float::is_finite_f32;
#[cfg(feature = "shader-f64")]
use super::float::is_finite_f64;
use super::*;

#[path = "programlet_vector_actions/float_math.rs"]
mod float_math;
use float_math::{exp_f32, exp_f64};

const SUBTRACT_VERB: u32 = 0xabd2;
const EXPONENTIAL_VERB: u32 = 0x8c29;
const DIVIDE_VERB: u32 = 0x8893;
const SUM_VERB: u32 = 0xbc8a;
/// `gjat` = magnitude and `bwit` = boundary: the phrase `gjat bwit` is
/// "maximum". The atom names say what the words mean; the local aliases say
/// what this reader does with them.
const MAGNITUDE: u32 = 0xa9b2;
const BOUNDARY: u32 = 0xa831;
const MAXIMUM_FIRST: u32 = MAGNITUDE;
const MAXIMUM_SECOND: u32 = BOUNDARY;

fn vector_precision_matches(vector_kind: u32, scalar_kind: u32) -> bool {
    (vector_kind == VALUE_VECTOR_FLOAT32 && scalar_kind == VALUE_FLOAT32)
        || (vector_kind == VALUE_VECTOR_FLOAT64 && scalar_kind == VALUE_FLOAT64)
}

fn apply_f32(
    vector: &ProgramletVectorValue,
    scalar: Option<ProgramletNumber>,
    other: Option<&ProgramletVectorValue>,
    verb: u32,
) -> Result<ProgramletVectorValue, u32> {
    let mut result = *vector;
    let mut result_values = result.values;
    let values = vector.values;
    let scalar_value = match scalar {
        Some(value) => Some(f32::from_bits(value.bits)),
        None => None,
    };
    let mut lane = 0;
    while lane < vector.length {
        let input = f32::from_bits(array_at!(values, lane));
        let other_value = other.map(|value| f32::from_bits(array_at!(value.values, lane)));
        let output = match verb {
            PLUS => input + other_value.ok_or(ERROR_MALFORMED)?,
            MAXIMUM_FIRST => other_value
                .map(|other| if other > input { other } else { input })
                .ok_or(ERROR_MALFORMED)?,
            MULTIPLY => input * scalar_value.ok_or(ERROR_MALFORMED)?,
            SUBTRACT_VERB => input - scalar_value.ok_or(ERROR_MALFORMED)?,
            DIVIDE_VERB => {
                let divisor = scalar_value.ok_or(ERROR_MALFORMED)?;
                if divisor == 0.0 {
                    return Err(ERROR_DIVISION_BY_ZERO);
                }
                input / divisor
            }
            EXPONENTIAL_VERB => exp_f32(input)?,
            _ => return Err(ERROR_UNSUPPORTED),
        };
        if !is_finite_f32(output) && !(verb == SUBTRACT_VERB && output.to_bits() == 0xff80_0000) {
            return Err(ERROR_FLOAT_DOMAIN);
        }
        array_put!(&mut result_values, lane, output.to_bits());
        lane += 1;
    }
    result.values = result_values;
    Ok(result)
}

#[cfg(feature = "shader-f64")]
fn apply_f64(
    vector: &ProgramletVectorValue,
    scalar: Option<ProgramletNumber>,
    other: Option<&ProgramletVectorValue>,
    verb: u32,
) -> Result<ProgramletVectorValue, u32> {
    let mut result = *vector;
    let mut result_values = result.values;
    let values = vector.values;
    let scalar_value = match scalar {
        Some(value) => Some(f64::from_bits(
            u64::from(value.bits) | (u64::from(value.high) << 32),
        )),
        None => None,
    };
    let mut lane = 0;
    while lane < vector.length {
        let offset = lane * 2;
        let input = f64::from_bits(
            u64::from(array_at!(values, offset)) | (u64::from(array_at!(values, offset + 1)) << 32),
        );
        let other_value = other.map(|value| {
            f64::from_bits(
                u64::from(array_at!(value.values, offset))
                    | (u64::from(array_at!(value.values, offset + 1)) << 32),
            )
        });
        let output = match verb {
            PLUS => input + other_value.ok_or(ERROR_MALFORMED)?,
            MAXIMUM_FIRST => other_value
                .map(|other| if other > input { other } else { input })
                .ok_or(ERROR_MALFORMED)?,
            MULTIPLY => input * scalar_value.ok_or(ERROR_MALFORMED)?,
            SUBTRACT_VERB => input - scalar_value.ok_or(ERROR_MALFORMED)?,
            DIVIDE_VERB => {
                let divisor = scalar_value.ok_or(ERROR_MALFORMED)?;
                if divisor == 0.0 {
                    return Err(ERROR_DIVISION_BY_ZERO);
                }
                input / divisor
            }
            EXPONENTIAL_VERB => exp_f64(input)?,
            _ => return Err(ERROR_UNSUPPORTED),
        };
        if !is_finite_f64(output)
            && !(verb == SUBTRACT_VERB && output.to_bits() == 0xfff0_0000_0000_0000)
        {
            return Err(ERROR_FLOAT_DOMAIN);
        }
        let bits = output.to_bits();
        array_put!(&mut result_values, offset, bits as u32);
        array_put!(&mut result_values, offset + 1, (bits >> 32) as u32);
        lane += 1;
    }
    result.values = result_values;
    Ok(result)
}

fn reduce_vector(vector: &ProgramletVectorValue, verb: u32) -> Result<ProgramletNumber, u32> {
    if vector.length == 0 {
        return Err(ERROR_VECTOR_WIDTH);
    }
    let values = vector.values;
    if vector.kind == VALUE_VECTOR_FLOAT32 {
        let mut value = f32::from_bits(array_at!(values, 0));
        let mut lane = 1;
        while lane < vector.length {
            let next = f32::from_bits(array_at!(values, lane));
            value = if verb == SUM_VERB {
                value + next
            } else if next > value {
                next
            } else {
                value
            };
            lane += 1;
        }
        if !is_finite_f32(value) {
            return Err(ERROR_FLOAT_DOMAIN);
        }
        return Ok(ProgramletNumber {
            bits: value.to_bits(),
            high: 0,
            kind: VALUE_FLOAT32,
        });
    }
    #[cfg(feature = "shader-f64")]
    if vector.kind == VALUE_VECTOR_FLOAT64 {
        let bits = u64::from(array_at!(values, 0)) | (u64::from(array_at!(values, 1)) << 32);
        let mut value = f64::from_bits(bits);
        let mut lane = 1;
        while lane < vector.length {
            let offset = lane * 2;
            let next = f64::from_bits(
                u64::from(array_at!(values, offset))
                    | (u64::from(array_at!(values, offset + 1)) << 32),
            );
            value = if verb == SUM_VERB {
                value + next
            } else if next > value {
                next
            } else {
                value
            };
            lane += 1;
        }
        if !is_finite_f64(value) {
            return Err(ERROR_FLOAT_DOMAIN);
        }
        let bits = value.to_bits();
        return Ok(ProgramletNumber {
            bits: bits as u32,
            high: (bits >> 32) as u32,
            kind: VALUE_FLOAT64,
        });
    }
    Err(ERROR_UNSUPPORTED)
}

fn maximum_index(vector: &ProgramletVectorValue) -> Result<u32, u32> {
    if vector.length == 0 {
        return Err(ERROR_VECTOR_WIDTH);
    }
    if vector.kind == VALUE_VECTOR_FLOAT32 {
        let mut maximum = f32::from_bits(array_at!(vector.values, 0));
        if !is_finite_f32(maximum) {
            return Err(ERROR_FLOAT_DOMAIN);
        }
        let mut maximum_index = 0;
        let mut lane = 1;
        while lane < vector.length {
            let value = f32::from_bits(array_at!(vector.values, lane));
            if !is_finite_f32(value) {
                return Err(ERROR_FLOAT_DOMAIN);
            }
            if value > maximum {
                maximum = value;
                maximum_index = lane;
            }
            lane += 1;
        }
        return Ok(maximum_index as u32);
    }
    #[cfg(feature = "shader-f64")]
    if vector.kind == VALUE_VECTOR_FLOAT64 {
        let mut maximum = f64::from_bits(
            u64::from(array_at!(vector.values, 0)) | (u64::from(array_at!(vector.values, 1)) << 32),
        );
        if !is_finite_f64(maximum) {
            return Err(ERROR_FLOAT_DOMAIN);
        }
        let mut maximum_index = 0;
        let mut lane = 1;
        while lane < vector.length {
            let offset = lane * 2;
            let value = f64::from_bits(
                u64::from(array_at!(vector.values, offset))
                    | (u64::from(array_at!(vector.values, offset + 1)) << 32),
            );
            if !is_finite_f64(value) {
                return Err(ERROR_FLOAT_DOMAIN);
            }
            if value > maximum {
                maximum = value;
                maximum_index = lane;
            }
            lane += 1;
        }
        return Ok(maximum_index as u32);
    }
    Err(ERROR_UNSUPPORTED)
}

pub(super) fn is_programlet_vector_verb(
    sentence: &ParsedSentence,
    tokens: &SentenceView<'_, impl VliwWord>,
) -> bool {
    matches!(
        (
            sentence.verb_length,
            sentence_at!(*tokens, sentence.verb_start)
        ),
        (
            1,
            PLUS | MULTIPLY | SUBTRACT_VERB | EXPONENTIAL_VERB | DIVIDE_VERB | SUM_VERB
        ) | (2, MAXIMUM_FIRST)
    )
}

pub(super) fn is_programlet_vector_action(
    sentence: &ParsedSentence,
    tokens: &SentenceView<'_, impl VliwWord>,
    symbols: &[u32; MAX_PROGRAMLET_SYMBOL_WORDS],
    arguments: &[ProgramletArgument; MAX_PROGRAMLET_ARGUMENTS],
    vector_slots: &[ProgramletVectorSlot; MAX_PROGRAMLET_VECTOR_LOCAL_SLOTS],
) -> bool {
    if !is_programlet_vector_verb(sentence, tokens) {
        return false;
    }
    match sentence.field(SOURCE) {
        Some(source) => {
            programlet_field_is_vector(tokens, source, symbols, arguments, vector_slots)
        }
        None => false,
    }
}

#[inline(never)]
pub(super) fn apply_programlet_vector_action(
    sentence: &ParsedSentence,
    tokens: &SentenceView<'_, impl VliwWord>,
    symbols: &mut [u32; MAX_PROGRAMLET_SYMBOL_WORDS],
    symbol_used: &mut usize,
    arguments: &[ProgramletArgument; MAX_PROGRAMLET_ARGUMENTS],
    frame_index: usize,
    slots: &mut [NumericLocalSlot; MAX_PROGRAMLET_LOCAL_SLOTS],
    vector_slots: &mut [ProgramletVectorSlot; MAX_PROGRAMLET_VECTOR_LOCAL_SLOTS],
    vector_values: &mut [u32; MAX_PROGRAMLET_VECTOR_VALUE_WORDS],
) -> Result<(), u32> {
    if sentence.mood != DEONTIC || !is_programlet_vector_verb(sentence, tokens) {
        return Err(ERROR_UNSUPPORTED);
    }
    let source = sentence.field(SOURCE).ok_or(ERROR_MALFORMED)?;
    let output = sentence.field(DESTINATION).ok_or(ERROR_MALFORMED)?;
    if sentence.field_count
        != if sentence.field(COMITATIVE).is_some() {
            3
        } else {
            2
        }
    {
        return Err(ERROR_MALFORMED);
    }
    let vector = read_programlet_vector(
        tokens,
        source,
        symbols,
        arguments,
        frame_index,
        vector_slots,
        vector_values,
    )?;
    let verb = if sentence.verb_length == 2 {
        if sentence_at!(*tokens, sentence.verb_start + 1) != MAXIMUM_SECOND {
            return Err(ERROR_MALFORMED);
        }
        MAXIMUM_FIRST
    } else {
        sentence_at!(*tokens, sentence.verb_start)
    };
    let elementwise_maximum = verb == MAXIMUM_FIRST && sentence.field(COMITATIVE).is_some();
    if verb == SUM_VERB || (verb == MAXIMUM_FIRST && !elementwise_maximum) {
        if sentence.field(COMITATIVE).is_some() {
            return Err(ERROR_MALFORMED);
        }
        let mut output_name = NameSpan::EMPTY;
        let mut output_kind = 0;
        let output_error = typed_output_name(tokens, output, &mut output_name, &mut output_kind);
        if output_error != 0 {
            return Err(output_error);
        }
        if verb == MAXIMUM_FIRST && output_kind == VALUE_NUMBER {
            let index = maximum_index(&vector)?;
            write_programlet_float(
                tokens,
                output,
                symbols,
                symbol_used,
                VALUE_NUMBER,
                ProgramletNumber {
                    bits: index,
                    high: 0,
                    kind: VALUE_UNSIGNED,
                },
                slots,
            )?;
            return Ok(());
        }
        let result = reduce_vector(&vector, verb)?;
        write_programlet_float(
            tokens,
            output,
            symbols,
            symbol_used,
            output_kind,
            result,
            slots,
        )?;
        return Ok(());
    }
    let other = if matches!(verb, PLUS | MAXIMUM_FIRST) {
        let field = sentence.field(COMITATIVE).ok_or(ERROR_MALFORMED)?;
        Some(read_programlet_vector(
            tokens,
            field,
            symbols,
            arguments,
            frame_index,
            vector_slots,
            vector_values,
        )?)
    } else {
        None
    };
    if let Some(other) = other
        && (other.kind != vector.kind || other.length != vector.length)
    {
        return Err(if other.kind != vector.kind {
            ERROR_UNSUPPORTED
        } else {
            ERROR_VECTOR_WIDTH
        });
    }
    let scalar = if matches!(verb, PLUS | MAXIMUM_FIRST) {
        None
    } else {
        match sentence.field(COMITATIVE) {
            Some(field) => Some(read_programlet_float(
                tokens, field, symbols, arguments, slots,
            )?),
            None => None,
        }
    };
    if !matches!(verb, EXPONENTIAL_VERB | PLUS | MAXIMUM_FIRST) && scalar.is_none() {
        return Err(ERROR_MALFORMED);
    }
    if let Some(value) = scalar
        && !vector_precision_matches(vector.kind, value.kind)
    {
        return Err(ERROR_UNSUPPORTED);
    }
    let result = if vector.kind == VALUE_VECTOR_FLOAT32 {
        apply_f32(&vector, scalar, other.as_ref(), verb)?
    } else {
        #[cfg(not(feature = "shader-f64"))]
        return Err(ERROR_UNSUPPORTED);
        #[cfg(feature = "shader-f64")]
        apply_f64(&vector, scalar, other.as_ref(), verb)?
    };
    write_programlet_vector(
        tokens,
        output,
        symbols,
        symbol_used,
        result,
        vector_slots,
        vector_values,
        frame_index,
    )?;
    Ok(())
}
