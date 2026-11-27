//! Conditional comparison: decode, then select an arm.

use super::errors::*;
use super::types::*;
use super::words::*;
use crate::WORDS_PER_CODELET;

/// Decode a Pyash conditional with an inline numeric equality test and two
/// inline numeric-literal arms. The input vectors are one completion-marked
/// sentence group, as emitted by the reference VLIW sentence encoder.
pub fn decode_conditional_number(
    codelets: &[[u16; WORDS_PER_CODELET]],
) -> Result<ConditionalNumberInstruction, SemanticError> {
    if codelets.len() < 2
        || codelets.first().is_some_and(|codelet| codelet[0] & 1 != 0)
        || codelets
            .iter()
            .take(codelets.len() - 1)
            .any(|codelet| codelet[0] & 1 != 0)
        || codelets.last().is_some_and(|codelet| codelet[0] & 1 == 0)
    {
        return Err(SemanticError::UnsupportedBranch);
    }

    let final_head = codelets.last().expect("non-empty branch group")[0];
    let last_slot = (1..WORDS_PER_CODELET)
        .rev()
        .find(|slot| final_head & (1 << slot) != 0)
        .ok_or(SemanticError::UnsupportedBranch)?;
    if codelets.last().expect("non-empty branch group")[last_slot + 1..]
        .iter()
        .any(|word| *word != 0)
    {
        return Err(SemanticError::UnsupportedBranch);
    }

    let mut words = Vec::with_capacity((codelets.len() - 1) * (WORDS_PER_CODELET - 1) + last_slot);
    let mut boundaries = Vec::new();
    for (vector_index, codelet) in codelets.iter().enumerate() {
        let completion = codelet[0] & 1 != 0;
        let end_slot = if vector_index + 1 == codelets.len() {
            last_slot
        } else {
            WORDS_PER_CODELET - 1
        };
        for slot in 1..=end_slot {
            words.push(codelet[slot]);
            if (codelet[0] & (1 << slot) != 0) == completion {
                boundaries.push(vector_index * (WORDS_PER_CODELET - 1) + slot);
            }
        }
    }

    const EXPECTED_BOUNDARIES: [usize; 7] = [3, 6, 8, 11, 13, 16, 18];
    if words.len() != 18 || boundaries != EXPECTED_BOUNDARIES {
        return Err(SemanticError::UnsupportedBranch);
    }

    for (index, expected) in [
        (0, NUMBER_QUOTE),
        (2, ACCUSATIVE_CASE),
        (3, NUMBER_QUOTE),
        (5, SOURCE_CASE),
        (6, EQUALLY),
        (7, CONDITIONAL_MOOD),
        (8, NUMBER_QUOTE),
        (10, ACCUSATIVE_CASE),
        (11, NUMBER_VERB),
        (12, REALIS_MOOD),
        (13, NUMBER_QUOTE),
        (15, ACCUSATIVE_CASE),
        (16, NUMBER_VERB),
        (17, REALIS_MOOD),
    ] {
        let actual = words[index];
        if actual != expected {
            return Err(SemanticError::InvalidBranchShape {
                index,
                expected,
                actual,
            });
        }
    }

    Ok(ConditionalNumberInstruction {
        left: words[1],
        right: words[4],
        consequence: words[9],
        alternative: words[14],
    })
}
pub fn execute_conditional_number(instruction: &ConditionalNumberInstruction) -> u16 {
    if instruction.left == instruction.right {
        instruction.consequence
    } else {
        instruction.alternative
    }
}
pub fn execute_conditional_number_codelets(
    codelets: &[[u16; WORDS_PER_CODELET]],
) -> Result<u16, SemanticError> {
    let instruction = decode_conditional_number(codelets)?;
    Ok(execute_conditional_number(&instruction))
}
