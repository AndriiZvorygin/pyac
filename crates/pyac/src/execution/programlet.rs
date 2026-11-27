//! Programlet execution: one ceremony's body, run over its own frame.

use crate::semantic::SemanticError;
use crate::{
    PackExecutionError, PyacValue, VectorElements, execute_date_codelet, execute_pack_sections,
    execute_vector_codelet, prepare_storage_sections,
};

pub fn execute_shared_programlet(
    words: &[impl pyac_vliw_core::VliwWord],
    base_codelet: usize,
    codelet_count: usize,
    error_index: usize,
    storage: &pyac_vliw_core::StorageView<'_>,
) -> Result<PyacValue, PackExecutionError> {
    let mut symbol_words = [0; pyac_vliw_core::MAX_PROGRAMLET_SYMBOL_WORDS];
    let mut frames = pyac_vliw_core::ProgramletStack::EMPTY;
    let mut routine_directory_words = [0; pyac_vliw_core::PROGRAMLET_ROUTINE_DIRECTORY_WORDS];
    let directory_result = pyac_vliw_core::build_programlet_routine_directory_words(
        words,
        base_codelet,
        codelet_count,
        &mut routine_directory_words,
    );
    if let Err(error) = directory_result {
        return Err(PackExecutionError {
            index: error_index,
            source: programlet_error(error),
        });
    }
    let result = pyac_vliw_core::execute_numeric_programlet_at_with_directory_words_and_storage(
        words,
        base_codelet,
        codelet_count,
        pyac_vliw_core::DEFAULT_PROGRAMLET_STEP_BUDGET,
        &routine_directory_words,
        0,
        storage,
        &mut symbol_words,
        &mut frames,
    );
    if result.error != 0 {
        return Err(PackExecutionError {
            index: error_index,
            source: programlet_error(result.error),
        });
    }
    let result_length = result.length as usize;
    Ok(match result.value_kind {
        pyac_vliw_core::VALUE_BOOLEAN => PyacValue::Boolean(result.values[0] != 0),
        pyac_vliw_core::VALUE_SIGNED => PyacValue::Signed(i64::from(result.values[0] as i32)),
        pyac_vliw_core::VALUE_FLOAT32 => PyacValue::Float32(result.values[0]),
        pyac_vliw_core::VALUE_FLOAT64 => {
            PyacValue::Float64(u64::from(result.values[0]) | (u64::from(result.values[1]) << 32))
        }
        pyac_vliw_core::VALUE_VECTOR_FLOAT32
            if result_length > 0 && result_length <= pyac_vliw_core::MAX_VECTOR_LANES =>
        {
            PyacValue::Vector(VectorElements::Float32(
                result.values[..result_length].to_vec(),
            ))
        }
        pyac_vliw_core::VALUE_VECTOR_FLOAT64
            if result_length > 0 && result_length * 2 <= pyac_vliw_core::RESULT_WORDS =>
        {
            let values = result.values[..result_length * 2]
                .chunks_exact(2)
                .map(|words| u64::from(words[0]) | (u64::from(words[1]) << 32))
                .collect();
            PyacValue::Vector(VectorElements::Float64(values))
        }
        pyac_vliw_core::VALUE_VECTOR_FLOAT32 | pyac_vliw_core::VALUE_VECTOR_FLOAT64 => {
            return Err(PackExecutionError {
                index: error_index,
                source: SemanticError::MalformedProgramlet,
            });
        }
        pyac_vliw_core::VALUE_FIXED16 => PyacValue::Fixed16(result.values[0] as u16),
        _ => PyacValue::Unsigned(u64::from(result.values[0])),
    })
}

pub(crate) fn programlet_error(error: u32) -> SemanticError {
    match error {
        pyac_vliw_core::ERROR_DIVISION_BY_ZERO => SemanticError::DivisionByZero,
        pyac_vliw_core::ERROR_FLOAT_DOMAIN => SemanticError::NonFiniteFloat,
        pyac_vliw_core::ERROR_FIXED16_DOMAIN => SemanticError::Fixed16Domain,
        pyac_vliw_core::ERROR_FRAME_CAPACITY => SemanticError::ProgramletFrameCapacity,
        pyac_vliw_core::ERROR_CONSTANT_WRITE => SemanticError::ProgramletConstantWrite,
        pyac_vliw_core::ERROR_STEP_BUDGET => SemanticError::ProgramletStepBudget,
        pyac_vliw_core::ERROR_PROGRAM_CAPACITY => SemanticError::ProgramletProgramCapacity,
        pyac_vliw_core::ERROR_CALL_DEPTH => SemanticError::ProgramletCallDepth,
        pyac_vliw_core::ERROR_UNSUPPORTED => SemanticError::UnsupportedProgramlet,
        _ => SemanticError::MalformedProgramlet,
    }
}
