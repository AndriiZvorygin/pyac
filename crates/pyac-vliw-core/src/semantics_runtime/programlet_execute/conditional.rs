//! The `[iff …]` pre-step: which arm of a conditional sentence runs.
//!
//! A conditional names its arm before the sentence is parsed, so the selection
//! happens first and the chosen action is what the loop then executes.

use super::super::*;

/// The result of a conditional pre-step: an action to run, a skip, or an error.
pub(super) enum ConditionalSelection {
    Selected(ParsedSentence),
    Skip,
    Failed(u32),
}

/// Evaluate a `[iff …]` prefix and return the action it selected.
///
/// A conditional sentence names which arm runs; the loop, not this function,
/// decides what skipping means, because skipping has to write the frame back and
/// advance to the next codelet.
#[allow(clippy::too_many_arguments)]
pub(super) fn select_conditional_sentence<W: VliwWord>(
    sentence_view: &SentenceView<'_, W>,
    frames: &ProgramletStack,
    frame: &mut ProgramletFrame,
    frame_index: usize,
    symbol_words: &[u32; MAX_PROGRAMLET_SYMBOL_WORDS],
    steps: &mut u32,
    step_budget: u32,
) -> ConditionalSelection {
    if *steps >= step_budget {
        return ConditionalSelection::Failed(ERROR_STEP_BUDGET);
    }
    *steps += 1;
    frame.action_started = 1;
    let mut action = ParsedSentence::empty();
    let mut has_selected_action = false;
    let arguments = array_at!(frames.arguments, frame_index);
    if !frame_row_is_valid::<_, _, MAX_PROGRAMLET_LOCAL_SLOTS>(&frames.slots, frame_index) {
        return ConditionalSelection::Failed(ERROR_MALFORMED);
    }
    let slots = frame_row::<_, _, MAX_PROGRAMLET_LOCAL_SLOTS>(&frames.slots, frame_index);
    let branch_error = programlet_branch(
        sentence_view,
        symbol_words,
        &arguments,
        slots,
        &mut action,
        &mut has_selected_action,
    );
    if branch_error != 0 {
        return ConditionalSelection::Failed(branch_error);
    }
    if !has_selected_action {
        return ConditionalSelection::Skip;
    }
    if *steps >= step_budget {
        return ConditionalSelection::Failed(ERROR_STEP_BUDGET);
    }
    *steps += 1;
    ConditionalSelection::Selected(action)
}
