//! Operation signatures: the dispatch key, its slots, and its results.

use super::errors::SourceError;
use super::records::{ArgumentRecord, SignatureRecord};
use super::values::take_sequence;

pub(super) fn parse_signature(words: &[u16], line: usize) -> Result<SignatureRecord, SourceError> {
    let mut cursor = 0usize;
    let unsupported = || SourceError::Unsupported {
        line,
        text: "operation signature has an invalid dispatch key".to_owned(),
    };
    let verb = take_sequence(words, &mut cursor).ok_or_else(unsupported)?;
    let mood = take_sequence(words, &mut cursor).ok_or_else(unsupported)?;
    let count = *words.get(cursor).ok_or_else(unsupported)? as usize;
    cursor += 1;
    let mut arguments = Vec::with_capacity(count);
    for _ in 0..count {
        let grammatical_case = take_sequence(words, &mut cursor).ok_or_else(unsupported)?;
        let noun_class = take_sequence(words, &mut cursor).ok_or_else(unsupported)?;
        arguments.push(ArgumentRecord {
            grammatical_case,
            noun_class,
            tag_label: String::new(),
            descriptive_specification: String::new(),
        });
    }
    if cursor != words.len() {
        return Err(unsupported());
    }
    Ok(SignatureRecord {
        verb,
        mood,
        arguments,
        results: Vec::new(),
    })
}
pub(super) fn parse_slot(words: &[u16], line: usize) -> Result<(Vec<u16>, Vec<u16>), SourceError> {
    let mut cursor = 0usize;
    let unsupported = || SourceError::Unsupported {
        line,
        text: "operation argument descriptor has trailing words".to_owned(),
    };
    let grammatical_case = take_sequence(words, &mut cursor).ok_or_else(unsupported)?;
    let noun_class = take_sequence(words, &mut cursor).ok_or_else(unsupported)?;
    if cursor != words.len() {
        return Err(unsupported());
    }
    Ok((grammatical_case, noun_class))
}
