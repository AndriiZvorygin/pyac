use super::super::reader::{DictionaryReadError, DictionarySection};
use super::super::reader_codec::{
    expect_end, expect_word, read_sentence, subject_end, take_text, take_vector,
};
use super::super::volume_records::INDEX_SUBJECT;
use super::super::words::{ACCUSATIVE, COMITATIVE, DESTINATION, ESSIVE, INSTRUMENTAL, SOURCE};
use super::super::{ArgumentSlot, OperationSignatureKey};
use std::io::{Read, Seek};
use unicode_normalization::UnicodeNormalization;

pub(crate) fn lookup_legacy_surface<R: Read + Seek>(
    source: &mut R,
    section: &DictionarySection,
    limit: u32,
    language: &str,
    surface: &str,
) -> Result<Vec<Vec<u16>>, DictionaryReadError> {
    let normalized: String = surface.nfc().collect();
    let mut matches = Vec::new();
    for_each_sentence(source, section, limit, |_, words| {
        let mut cursor = index_subject(words)?;
        expect_text(words, &mut cursor, ACCUSATIVE, "language-surface")?;
        let candidate_language = take_text(words, &mut cursor)?;
        expect_word(words, &mut cursor, ESSIVE)?;
        let candidate_surface = take_text(words, &mut cursor)?;
        expect_word(words, &mut cursor, SOURCE)?;
        let key = take_vector(words, &mut cursor)?;
        expect_word(words, &mut cursor, DESTINATION)?;
        expect_end(words, &mut cursor)?;
        if candidate_language == language && candidate_surface == normalized {
            matches.push(key);
        }
        Ok(())
    })?;
    matches.sort_unstable();
    matches.dedup();
    Ok(matches)
}
pub(crate) fn lookup_legacy_source<R: Read + Seek>(
    source: &mut R,
    section: &DictionarySection,
    limit: u32,
    dataset: &str,
    release: &str,
    identifier: &str,
) -> Result<Vec<Vec<u16>>, DictionaryReadError> {
    let mut matches = Vec::new();
    for_each_sentence(source, section, limit, |_, words| {
        let mut cursor = index_subject(words)?;
        expect_text(words, &mut cursor, ACCUSATIVE, "external-source")?;
        let candidate_dataset = take_text(words, &mut cursor)?;
        expect_word(words, &mut cursor, ESSIVE)?;
        let candidate_release = take_text(words, &mut cursor)?;
        expect_word(words, &mut cursor, SOURCE)?;
        let candidate_identifier = take_text(words, &mut cursor)?;
        expect_word(words, &mut cursor, DESTINATION)?;
        let key = take_vector(words, &mut cursor)?;
        expect_word(words, &mut cursor, INSTRUMENTAL)?;
        let _relation = take_text(words, &mut cursor)?;
        expect_word(words, &mut cursor, COMITATIVE)?;
        expect_end(words, &mut cursor)?;
        if candidate_dataset == dataset
            && candidate_release == release
            && candidate_identifier == identifier
        {
            matches.push(key);
        }
        Ok(())
    })?;
    matches.sort_unstable();
    matches.dedup();
    Ok(matches)
}
pub(crate) fn lookup_legacy_operation<R: Read + Seek>(
    source: &mut R,
    section: &DictionarySection,
    limit: u32,
    lookup: &OperationSignatureKey,
) -> Result<Vec<Vec<u16>>, DictionaryReadError> {
    let mut matches = Vec::new();
    for_each_sentence(source, section, limit, |_, words| {
        let mut cursor = index_subject(words)?;
        expect_text(words, &mut cursor, ACCUSATIVE, "operation-signature")?;
        let dispatch = take_vector(words, &mut cursor)?;
        expect_word(words, &mut cursor, SOURCE)?;
        let key = take_vector(words, &mut cursor)?;
        expect_word(words, &mut cursor, DESTINATION)?;
        expect_end(words, &mut cursor)?;
        if &decode_signature(&dispatch)? == lookup {
            matches.push(key);
        }
        Ok(())
    })?;
    matches.sort_unstable();
    matches.dedup();
    Ok(matches)
}
pub(crate) fn for_each_sentence<R: Read + Seek>(
    source: &mut R,
    section: &DictionarySection,
    limit: u32,
    mut visit: impl FnMut(u32, &[u16]) -> Result<(), DictionaryReadError>,
) -> Result<(), DictionaryReadError> {
    let range = section.range();
    let mut ordinal = range.start;
    while ordinal < range.end {
        let sentence = read_sentence(source, ordinal, limit)?;
        visit(ordinal, &sentence.words)?;
        ordinal = ordinal
            .checked_add(sentence.consumed)
            .ok_or_else(|| invalid("index sentence ordinal overflow"))?;
    }
    if ordinal != range.end {
        return Err(invalid("index section ends inside a sentence"));
    }
    Ok(())
}
pub(crate) fn index_subject(words: &[u16]) -> Result<usize, DictionaryReadError> {
    subject_end(words, &INDEX_SUBJECT)
}
pub(crate) fn expect_text(
    words: &[u16],
    cursor: &mut usize,
    case: u16,
    expected: &str,
) -> Result<(), DictionaryReadError> {
    let actual = take_text(words, cursor)?;
    expect_word(words, cursor, case)?;
    if actual != expected {
        return Err(invalid("index record has an unexpected record kind"));
    }
    Ok(())
}
pub(crate) fn decode_signature(
    words: &[u16],
) -> Result<OperationSignatureKey, DictionaryReadError> {
    let mut cursor = 0;
    let verb = take_sequence(words, &mut cursor)?;
    let mood = take_sequence(words, &mut cursor)?;
    let count = *words
        .get(cursor)
        .ok_or_else(|| invalid("operation signature has no argument count"))?
        as usize;
    cursor += 1;
    let mut arguments = Vec::with_capacity(count);
    for _ in 0..count {
        let grammatical_case = take_sequence(words, &mut cursor)?;
        let noun_class = take_sequence(words, &mut cursor)?;
        if grammatical_case.is_empty() || noun_class.is_empty() {
            return Err(invalid("operation argument type is empty"));
        }
        arguments.push(ArgumentSlot {
            grammatical_case,
            noun_class,
        });
    }
    if cursor != words.len() || verb.is_empty() || mood.is_empty() {
        return Err(invalid("operation signature has trailing or missing words"));
    }
    Ok(OperationSignatureKey::new(verb, mood, arguments))
}
pub(crate) fn take_sequence(
    words: &[u16],
    cursor: &mut usize,
) -> Result<Vec<u16>, DictionaryReadError> {
    let length = *words
        .get(*cursor)
        .ok_or_else(|| invalid("operation signature sequence is truncated"))?
        as usize;
    *cursor += 1;
    let end = (*cursor)
        .checked_add(length)
        .ok_or_else(|| invalid("operation signature sequence length overflow"))?;
    let sequence = words
        .get(*cursor..end)
        .ok_or_else(|| invalid("operation signature sequence is truncated"))?
        .to_vec();
    *cursor = end;
    Ok(sequence)
}
pub(crate) fn invalid(reason: &'static str) -> DictionaryReadError {
    DictionaryReadError::InvalidRecord(reason)
}
