//! What a record is about: a name, or a text.

use super::super::reader::DictionaryReadError;
use super::super::words::{NAME_MARKER, NOMINATIVE, TEXT_QUOTE};
use crate::literal::decode_quote_header;

use super::sentences::Sentence;

use super::values::{expect_word, take_text, take_vector};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RecordSubject {
    Name(Vec<u16>),
    Text(String),
    Uint16Vector(Vec<u16>),
}
pub(crate) fn take_record_subject(
    sentence: &Sentence,
    cursor: &mut usize,
) -> Result<RecordSubject, DictionaryReadError> {
    let words = &sentence.words;
    match words.get(*cursor).copied() {
        Some(NAME_MARKER) => {
            // The key runs from after NAME_MARKER to the first word whose boundary
            // flag is set. Scanning for the NOMINATIVE *value* is wrong: a key word may
            // itself be NOMINATIVE, which is exactly the `na` case (0x24DE).
            *cursor += 1;
            let start = *cursor;
            let end = sentence
                .boundaries
                .iter()
                .enumerate()
                .skip(start)
                .find(|(_, boundary)| **boundary)
                .map(|(index, _)| index)
                .ok_or(DictionaryReadError::InvalidRecord(
                    "name subject has no case boundary",
                ))?;
            if end == start {
                return Err(DictionaryReadError::InvalidRecord("empty name subject"));
            }
            if words.get(end) != Some(&NOMINATIVE) {
                return Err(DictionaryReadError::InvalidRecord(
                    "name subject does not end at a nominative boundary",
                ));
            }
            *cursor = end + 1;
            Ok(RecordSubject::Name(words[start..end].to_vec()))
        }
        Some(TEXT_QUOTE) => {
            let value = take_text(words, cursor)?;
            expect_word(words, cursor, NOMINATIVE)?;
            Ok(RecordSubject::Text(value))
        }
        Some(header)
            if decode_quote_header(header)
                .is_some_and(|item| item.sort == 4 && item.scalar_bytes == 2) =>
        {
            let value = take_vector(words, cursor)?;
            expect_word(words, cursor, NOMINATIVE)?;
            Ok(RecordSubject::Uint16Vector(value))
        }
        _ => Err(DictionaryReadError::InvalidRecord(
            "unsupported dictionary index subject type",
        )),
    }
}
