//! The programlet entry points and the mood router.
//!
//! `mood_class` is level one of dispatch: the mood chooses a class, and the class
//! plus the frame state chooses what the loop does next.

use super::super::*;
use super::execute_numeric_programlet_at_with_directory_view;

/// Level one of dispatch: the mood chooses a class table. `def` and the
/// conditional pre-step never reach this router, so they are not classes.
#[derive(Clone, Copy)]
pub(super) enum MoodClass {
    Static,
    Execute,
    Return,
    Close,
    Unsupported,
}

pub(super) fn mood_class(mood: u32) -> MoodClass {
    match mood {
        REALIS => MoodClass::Static,
        DEONTIC => MoodClass::Execute,
        MOOD_RETURN => MoodClass::Return,
        MOOD_PARAGRAPH => MoodClass::Close,
        _ => MoodClass::Unsupported,
    }
}

pub fn execute_numeric_programlet_at_with_directory_words_and_storage(
    words: &[impl VliwWord],
    base_codelet: usize,
    codelet_count: usize,
    step_budget: u32,
    routine_directory_words: &[u32],
    directory_offset: usize,
    storage: &StorageView<'_>,
    symbol_words: &mut [u32; MAX_PROGRAMLET_SYMBOL_WORDS],
    frames: &mut ProgramletStack,
) -> CoreResult {
    #[cfg(not(target_arch = "spirv"))]
    let directory =
        validate_programlet_routine_directory_words(routine_directory_words, directory_offset);
    #[cfg(target_arch = "spirv")]
    let directory =
        read_programlet_routine_directory_header(routine_directory_words, directory_offset);
    let (directory_entry_count, directory_signature_used) = match directory {
        Ok(directory) => directory,
        Err(error) => return CoreResult::failure(error, 0),
    };
    execute_numeric_programlet_at_with_directory_view(
        words,
        base_codelet,
        codelet_count,
        step_budget,
        routine_directory_words,
        directory_offset,
        directory_entry_count,
        directory_signature_used,
        storage,
        symbol_words,
        frames,
    )
}
