//! Scalar arithmetic: decode a codelet, then run it.

use super::errors::*;
use super::types::*;
use super::vectors::{
    decode_numeric_vector_arithmetic, execute_numeric_vector_arithmetic,
    looks_like_numeric_vector_arithmetic,
};
use super::words::*;
use crate::WORDS_PER_CODELET;

pub(super) fn expect(
    words: &[u16; WORDS_PER_CODELET],
    index: usize,
    expected: u16,
) -> Result<(), SemanticError> {
    let actual = words[index];
    if actual == expected {
        Ok(())
    } else {
        Err(SemanticError::InvalidShape {
            index,
            expected,
            actual,
        })
    }
}
pub fn decode_numeric_plus(
    words: &[u16; WORDS_PER_CODELET],
) -> Result<NumericPlusInstruction, SemanticError> {
    let instruction = decode_numeric_arithmetic(words)?;
    if instruction.operator != NumericOperator::Plus {
        return Err(SemanticError::UnsupportedOperator {
            actual: instruction.operator.word(),
        });
    }
    if instruction.operand_case != NumericOperandCase::To {
        return Err(SemanticError::UnsupportedCase { actual: words[6] });
    }
    Ok(NumericPlusInstruction {
        ob: instruction.ob,
        to: instruction.operand,
    })
}
pub fn decode_numeric_arithmetic(
    words: &[u16; WORDS_PER_CODELET],
) -> Result<NumericArithmeticInstruction, SemanticError> {
    expect(words, 0, HEAD)?;
    expect(words, 1, NUMBER_QUOTE)?;
    expect(words, 3, ACCUSATIVE_CASE)?;
    expect(words, 4, NUMBER_QUOTE)?;
    let operand_case = NumericOperandCase::from_word(words[6])
        .ok_or(SemanticError::UnsupportedCase { actual: words[6] })?;
    expect(words, 8, DEONTIC_MOOD)?;
    let operator = NumericOperator::from_word(words[7])
        .ok_or(SemanticError::UnsupportedOperator { actual: words[7] })?;
    for (index, actual) in words.iter().enumerate().skip(9) {
        if *actual != 0 {
            return Err(SemanticError::NonzeroTail {
                index,
                actual: *actual,
            });
        }
    }
    Ok(NumericArithmeticInstruction {
        operator,
        ob: words[2],
        operand: words[5],
        operand_case,
    })
}
/// Saturating exponentiation, matching the JavaScript reference backend:
/// the result is truncated and clamped into the unsigned 16-bit range.
fn saturating_power(base: u16, exponent: u16) -> u64 {
    let mut result: u64 = 1;
    let mut factor: u64 = u64::from(base);
    let mut remaining = exponent;
    while remaining > 0 {
        if remaining & 1 == 1 {
            result = result.saturating_mul(factor);
        }
        remaining >>= 1;
        if remaining > 0 {
            factor = factor.saturating_mul(factor);
        }
    }
    result.min(u64::from(u16::MAX))
}
pub fn execute_numeric_plus(instruction: &NumericPlusInstruction) -> u16 {
    instruction.ob.saturating_add(instruction.to)
}
pub fn execute_numeric_arithmetic(
    instruction: &NumericArithmeticInstruction,
) -> Result<u64, SemanticError> {
    execute_numeric_pair(instruction.operator, instruction.ob, instruction.operand)
}
pub(super) fn execute_numeric_pair(
    operator: NumericOperator,
    left: u16,
    right: u16,
) -> Result<u64, SemanticError> {
    match operator {
        NumericOperator::Plus => Ok(u64::from(left.saturating_add(right))),
        NumericOperator::Subtract => Ok(u64::from(left.saturating_sub(right))),
        NumericOperator::Multiply => Ok(u64::from(left) * u64::from(right)),
        NumericOperator::Divide => {
            if right == 0 {
                return Err(SemanticError::DivisionByZero);
            }
            Ok(u64::from(left / right))
        }
        NumericOperator::Xor => Ok(u64::from(left ^ right)),
        NumericOperator::Exponential => Ok(saturating_power(left, right)),
        NumericOperator::Remains => {
            if right == 0 {
                return Err(SemanticError::DivisionByZero);
            }
            Ok(u64::from(left % right))
        }
    }
}
pub fn decode_numeric_invert(
    words: &[u16; WORDS_PER_CODELET],
) -> Result<NumericInvertInstruction, SemanticError> {
    expect(words, 0, HEAD)?;
    expect(words, 1, NUMBER_QUOTE)?;
    expect(words, 3, ACCUSATIVE_CASE)?;
    expect(words, 4, NUMBER_QUOTE)?;
    expect(words, 6, DATIVE_CASE)?;
    expect(words, 7, INVERT)?;
    expect(words, 8, DEONTIC_MOOD)?;
    for (index, actual) in words.iter().enumerate().skip(9) {
        if *actual != 0 {
            return Err(SemanticError::NonzeroTail {
                index,
                actual: *actual,
            });
        }
    }
    let bound = words[5];
    if bound != 0 {
        return Err(SemanticError::UnsupportedBound { actual: bound });
    }
    Ok(NumericInvertInstruction { ob: words[2] })
}
pub fn execute_numeric_invert(instruction: &NumericInvertInstruction) -> i64 {
    -i64::from(instruction.ob)
}
pub fn execute_numeric_plus_codelet(
    words: &[u16; WORDS_PER_CODELET],
) -> Result<u16, SemanticError> {
    let instruction = decode_numeric_plus(words)?;
    Ok(execute_numeric_plus(&instruction))
}
pub fn execute_numeric_arithmetic_codelet(
    words: &[u16; WORDS_PER_CODELET],
) -> Result<u64, SemanticError> {
    let instruction = decode_numeric_arithmetic(words)?;
    execute_numeric_arithmetic(&instruction)
}
pub fn execute_numeric_invert_codelet(
    words: &[u16; WORDS_PER_CODELET],
) -> Result<i64, SemanticError> {
    let instruction = decode_numeric_invert(words)?;
    Ok(execute_numeric_invert(&instruction))
}
pub fn execute_numeric_codelet(
    words: &[u16; WORDS_PER_CODELET],
) -> Option<Result<NumericCodeletValue, SemanticError>> {
    if looks_like_numeric_vector_arithmetic(words) {
        return Some(
            decode_numeric_vector_arithmetic(words)
                .and_then(|instruction| execute_numeric_vector_arithmetic(&instruction))
                .map(NumericCodeletValue::Vector),
        );
    }
    if words[7] == INVERT || words[6] == DATIVE_CASE {
        return Some(execute_numeric_invert_codelet(words).map(NumericCodeletValue::Signed));
    }
    if words[1] == NUMBER_QUOTE
        && words[3] == ACCUSATIVE_CASE
        && words[4] == NUMBER_QUOTE
        && words[8] == DEONTIC_MOOD
    {
        return Some(execute_numeric_arithmetic_codelet(words).map(NumericCodeletValue::Unsigned));
    }
    None
}
