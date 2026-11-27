//! Building and comparing a routine's call signature.
//!
//! A call matches a definition when the formal and actual types line up, which is
//! what the directory sort keys on.

use super::super::programlet_dispatch_signature::{
    MAX_ROUTINE_SIGNATURE_CASES, SignatureCase, append_case, canonical_type, formal_type,
    insert_case, sort_cases,
};
use super::*;

#[cfg(not(target_arch = "spirv"))]
pub(super) fn definition_signature(
    sentence: &ParsedSentence,
    tokens: &SentenceView<'_, impl VliwWord>,
    directory: &mut ProgramletRoutineDirectory,
) -> Result<(usize, usize, NameSpan, usize, usize, usize, u32), u32> {
    let name_field = sentence.field(NOMINATIVE).ok_or(ERROR_MALFORMED)?;
    let output_field = sentence.field(DESTINATION).ok_or(ERROR_MALFORMED)?;
    let mut verb = NameSpan::EMPTY;
    check_status(name_span(tokens, name_field, false, &mut verb))?;
    let mut output_name = NameSpan::EMPTY;
    let mut output_kind = 0;
    check_status(typed_output_name(
        tokens,
        output_field,
        &mut output_name,
        &mut output_kind,
    ))?;

    let signature_start = directory.signature_used;
    let mut cases = [SignatureCase::EMPTY; MAX_ROUTINE_SIGNATURE_CASES];
    let mut case_count = 0;
    let fields = sentence.fields;
    let mut field_index = 0;
    while field_index < sentence.field_count {
        let field = array_at!(fields, field_index);
        if field.role != NOMINATIVE
            && field.role != DESTINATION
            && field.role != FROM_INDEX
            && field.role != TO_INDEX
        {
            insert_case(
                &mut cases,
                &mut case_count,
                field.role,
                formal_type(tokens, field)?,
            )?;
        }
        field_index += 1;
    }
    insert_case(
        &mut cases,
        &mut case_count,
        DESTINATION,
        canonical_type(output_kind)?,
    )?;
    sort_cases(&mut cases);
    let mut index = 0;
    while index < case_count {
        append_case(
            &mut directory.signatures,
            &mut directory.signature_used,
            array_at!(cases, index),
        )?;
        index += 1;
    }
    let verb_start = append_name(
        tokens,
        verb,
        &mut directory.signatures,
        &mut directory.signature_used,
    )?;
    let signature_length = directory.signature_used - signature_start;
    let output_name_start = append_name(
        tokens,
        output_name,
        &mut directory.signatures,
        &mut directory.signature_used,
    )?;
    Ok((
        signature_start,
        signature_length,
        verb,
        verb_start,
        output_name_start,
        output_name.length,
        output_kind,
    ))
}

#[derive(Clone, Copy)]
pub(super) struct CallSignature {
    pub(super) cases: [SignatureCase; MAX_ROUTINE_SIGNATURE_CASES],
    pub(super) case_count: usize,
    pub(super) verb_start: usize,
    pub(super) length: usize,
}

pub(super) fn make_call_signature(
    sentence: &ParsedSentence,
    arguments: &[ProgramletArgument; MAX_PROGRAMLET_ARGUMENTS],
    argument_count: u32,
    output_kind: u32,
    tokens: &SentenceView<'_, impl VliwWord>,
) -> Result<CallSignature, u32> {
    if argument_count as usize > arguments.len() {
        return Err(ERROR_FRAME_CAPACITY);
    }
    let mut cases = [SignatureCase::EMPTY; MAX_ROUTINE_SIGNATURE_CASES];
    let mut case_count = 0;
    let mut index = 0;
    while index < argument_count as usize {
        let argument = array_at!(*arguments, index);
        if argument.role != FROM_INDEX && argument.role != TO_INDEX {
            insert_case(
                &mut cases,
                &mut case_count,
                argument.role,
                canonical_type(argument.value_kind)?,
            )?;
        }
        index += 1;
    }
    insert_case(
        &mut cases,
        &mut case_count,
        DESTINATION,
        canonical_type(output_kind)?,
    )?;
    sort_cases(&mut cases);
    let mut case_words = 0;
    index = 0;
    while index < case_count {
        case_words += array_at!(cases, index).value_type.length + 1;
        index += 1;
    }
    if sentence.verb_length == 0 || case_words + sentence.verb_length > MAX_PROGRAMLET_SYMBOL_WORDS
    {
        return Err(ERROR_MALFORMED);
    }
    index = 0;
    while index < sentence.verb_length {
        let Some(token) = tokens.token(sentence.verb_start + index) else {
            return Err(ERROR_MALFORMED);
        };
        if token > MAX_U16 {
            return Err(ERROR_MALFORMED);
        }
        index += 1;
    }
    Ok(CallSignature {
        cases,
        case_count,
        verb_start: sentence.verb_start,
        length: case_words + sentence.verb_length,
    })
}
