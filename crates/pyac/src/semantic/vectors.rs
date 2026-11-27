//! Vector arithmetic: elementwise, with the width and sign rules.

use super::arithmetic::*;
use super::arithmetic::{execute_numeric_pair, expect};
use super::errors::*;
use super::types::*;
use super::words::*;
use crate::literal::{UINT_SORT_DENOTE, decode_quote_header, index_word};
use crate::{VectorElements, WORDS_PER_CODELET};

/// Decode elementwise arithmetic over established unsigned vector quotes.
/// The right operand may be a scalar or a same-width vector quote.
pub fn decode_numeric_vector_arithmetic(
    words: &[u16; WORDS_PER_CODELET],
) -> Result<NumericVectorArithmeticInstruction, SemanticError> {
    let header = decode_quote_header(words[1]).ok_or(SemanticError::UnsupportedCodelet)?;
    if header.sort != UINT_SORT_DENOTE || header.scalar_bytes != 2 || header.vector_thickness == 0 {
        return Err(SemanticError::UnsupportedCodelet);
    }
    let count = if header.explicit_vector_length {
        usize::from(words[2])
    } else {
        header
            .vector_words
            .ok_or(SemanticError::UnsupportedCodelet)?
    };
    if !(2..=8).contains(&count) {
        return Err(SemanticError::UnsupportedVectorWidth {
            actual: count,
            maximum: 8,
        });
    }

    let payload_start = if header.explicit_vector_length { 3 } else { 2 };
    let payload_end = payload_start + count;
    let accusative_offset = payload_end;
    let right_quote_offset = accusative_offset + 1;
    let (operand, case_offset) = if words[right_quote_offset] == NUMBER_QUOTE {
        (
            NumericVectorOperand::Scalar(words[right_quote_offset + 1]),
            right_quote_offset + 2,
        )
    } else {
        let right_header = decode_quote_header(words[right_quote_offset])
            .ok_or(SemanticError::UnsupportedCodelet)?;
        if right_header.sort != UINT_SORT_DENOTE
            || right_header.scalar_bytes != 2
            || right_header.vector_thickness == 0
        {
            return Err(SemanticError::UnsupportedCodelet);
        }
        let right_count = if right_header.explicit_vector_length {
            usize::from(words[right_quote_offset + 1])
        } else {
            right_header
                .vector_words
                .ok_or(SemanticError::UnsupportedCodelet)?
        };
        if right_count != count {
            return Err(SemanticError::VectorWidthMismatch {
                left: count,
                right: right_count,
            });
        }
        if !(2..=4).contains(&count) {
            return Err(SemanticError::UnsupportedVectorWidth {
                actual: count,
                maximum: 4,
            });
        }
        let right_payload_start = right_quote_offset
            + if right_header.explicit_vector_length {
                2
            } else {
                1
            };
        let right_end = right_payload_start + right_count;
        if right_end > WORDS_PER_CODELET {
            return Err(SemanticError::UnsupportedVectorWidth {
                actual: count,
                maximum: 4,
            });
        }
        (
            NumericVectorOperand::Vector(words[right_payload_start..right_end].to_vec()),
            right_end,
        )
    };
    let operator_offset = case_offset + 1;
    let mood_offset = operator_offset + 1;
    if mood_offset >= WORDS_PER_CODELET {
        return Err(SemanticError::UnsupportedVectorWidth {
            actual: count,
            maximum: if matches!(&operand, NumericVectorOperand::Vector(_)) {
                4
            } else {
                8
            },
        });
    }

    expect(words, accusative_offset, ACCUSATIVE_CASE)?;
    if matches!(&operand, NumericVectorOperand::Scalar(_)) {
        expect(words, right_quote_offset, NUMBER_QUOTE)?;
    }
    expect(words, mood_offset, DEONTIC_MOOD)?;
    let operand_case = NumericOperandCase::from_word(words[case_offset]).ok_or(
        SemanticError::UnsupportedCase {
            actual: words[case_offset],
        },
    )?;
    let operator = NumericOperator::from_word(words[operator_offset]).ok_or(
        SemanticError::UnsupportedOperator {
            actual: words[operator_offset],
        },
    )?;
    expect(
        words,
        0,
        index_word(&[accusative_offset, case_offset, mood_offset]),
    )?;
    for (index, actual) in words.iter().enumerate().skip(mood_offset + 1) {
        if *actual != 0 {
            return Err(SemanticError::NonzeroTail {
                index,
                actual: *actual,
            });
        }
    }

    Ok(NumericVectorArithmeticInstruction {
        operator,
        elements: words[payload_start..payload_end].to_vec(),
        operand,
        operand_case,
    })
}
pub fn execute_numeric_vector_arithmetic(
    instruction: &NumericVectorArithmeticInstruction,
) -> Result<VectorElements, SemanticError> {
    if let NumericVectorOperand::Vector(values) = &instruction.operand {
        if values.len() != instruction.elements.len() {
            return Err(SemanticError::VectorWidthMismatch {
                left: instruction.elements.len(),
                right: values.len(),
            });
        }
    }
    let values = instruction
        .elements
        .iter()
        .enumerate()
        .map(|(index, left)| {
            let right = match &instruction.operand {
                NumericVectorOperand::Scalar(value) => *value,
                NumericVectorOperand::Vector(values) => values[index],
            };
            execute_numeric_pair(instruction.operator, *left, right)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if instruction.operator == NumericOperator::Multiply {
        Ok(VectorElements::Numbers32(
            values.into_iter().map(|value| value as u32).collect(),
        ))
    } else {
        Ok(VectorElements::Numbers(
            values.into_iter().map(|value| value as u16).collect(),
        ))
    }
}
pub(super) fn looks_like_numeric_vector_arithmetic(words: &[u16; WORDS_PER_CODELET]) -> bool {
    let Some(header) = decode_quote_header(words[1]) else {
        return false;
    };
    if header.sort != UINT_SORT_DENOTE || header.scalar_bytes != 2 || header.vector_thickness == 0 {
        return false;
    }
    let count = if header.explicit_vector_length {
        usize::from(words[2])
    } else {
        header.vector_words.unwrap_or(0)
    };
    if !(2..=8).contains(&count) {
        return false;
    }
    let payload_end = (if header.explicit_vector_length { 3 } else { 2 }) + count;
    if words.get(payload_end) != Some(&ACCUSATIVE_CASE) {
        return false;
    }
    let right_offset = payload_end + 1;
    let case_offset = if words.get(right_offset) == Some(&NUMBER_QUOTE) {
        right_offset + 2
    } else {
        let Some(right_header) = decode_quote_header(words[right_offset]) else {
            return false;
        };
        if right_header.sort != UINT_SORT_DENOTE
            || right_header.scalar_bytes != 2
            || right_header.vector_thickness == 0
        {
            return false;
        }
        let right_count = if right_header.explicit_vector_length {
            usize::from(words[right_offset + 1])
        } else {
            right_header.vector_words.unwrap_or(0)
        };
        right_offset
            + (if right_header.explicit_vector_length {
                2
            } else {
                1
            })
            + right_count
    };
    let operator_offset = case_offset + 1;
    let mood_offset = operator_offset + 1;
    words.get(payload_end) == Some(&ACCUSATIVE_CASE)
        && words
            .get(case_offset)
            .and_then(|word| NumericOperandCase::from_word(*word))
            .is_some()
        && words
            .get(operator_offset)
            .and_then(|word| NumericOperator::from_word(*word))
            .is_some()
        && words.get(mood_offset) == Some(&DEONTIC_MOOD)
}
