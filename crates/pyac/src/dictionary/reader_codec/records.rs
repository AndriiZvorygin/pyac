//! The volume records: marker, metadata, table of contents, locations.

use super::super::reader::{DictionaryReadError, DictionarySection};
use super::super::volume::DictionaryMetadata;
use super::super::volume_records::{
    CONTENTS_SUBJECT, INDEX_SUBJECT, PROFILE_SUBJECT, PROFILE_VERSION, PROFILE_VERSION_V2,
    PROFILE_VERSION_V3,
};
use super::super::words::{
    ACCUSATIVE, COMITATIVE, DESTINATION, ESSIVE, INSTRUMENTAL, LOCATIVE, NUMBER_QUOTE, SOURCE,
};
use std::ops::Range;

use super::values::{expect_end, expect_word, subject_end, take_text, take_uint32, take_vector};

pub(crate) fn parse_marker(words: &[u16]) -> Result<String, DictionaryReadError> {
    let mut cursor = subject_end(words, &PROFILE_SUBJECT)?;
    let kind = take_text(words, &mut cursor)?;
    expect_word(words, &mut cursor, ACCUSATIVE)?;
    expect_end(words, &mut cursor)?;
    Ok(kind)
}
pub(crate) fn take_number(words: &[u16], cursor: &mut usize) -> Result<u16, DictionaryReadError> {
    expect_word(words, cursor, NUMBER_QUOTE)?;
    let value = *words
        .get(*cursor)
        .ok_or(DictionaryReadError::InvalidRecord("truncated uint16 quote"))?;
    *cursor += 1;
    Ok(value)
}
pub(crate) fn parse_metadata(
    words: &[u16],
) -> Result<(DictionaryMetadata, usize, String, Option<u32>), DictionaryReadError> {
    let mut cursor = subject_end(words, &PROFILE_SUBJECT)?;
    let title = take_text(words, &mut cursor)?;
    expect_word(words, &mut cursor, ACCUSATIVE)?;
    let revision_words = take_vector(words, &mut cursor)?;
    if revision_words.len() != 2 {
        return Err(DictionaryReadError::InvalidRecord(
            "revision must be a uint32 pair",
        ));
    }
    expect_word(words, &mut cursor, ESSIVE)?;
    let language_text = take_text(words, &mut cursor)?;
    expect_word(words, &mut cursor, SOURCE)?;
    let languages = language_text
        .split(',')
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let normalization_policy = take_text(words, &mut cursor)?;
    expect_word(words, &mut cursor, DESTINATION)?;
    let toc_count = take_number(words, &mut cursor)? as usize;
    expect_word(words, &mut cursor, INSTRUMENTAL)?;
    let profile_version = take_text(words, &mut cursor)?;
    expect_word(words, &mut cursor, COMITATIVE)?;
    let entry_count = if profile_version == PROFILE_VERSION {
        let value = take_uint32(words, &mut cursor)?;
        expect_word(words, &mut cursor, LOCATIVE)?;
        Some(value)
    } else {
        None
    };
    expect_end(words, &mut cursor)?;
    if ![PROFILE_VERSION, PROFILE_VERSION_V3, PROFILE_VERSION_V2]
        .contains(&profile_version.as_str())
        || languages.iter().any(String::is_empty)
    {
        return Err(DictionaryReadError::InvalidRecord(
            "unsupported profile revision or invalid language list",
        ));
    }
    Ok((
        DictionaryMetadata {
            kind: String::new(),
            title,
            revision: revision_words[0] as u32 | ((revision_words[1] as u32) << 16),
            languages,
            normalization_policy,
        },
        toc_count,
        profile_version,
        entry_count,
    ))
}
pub(crate) fn parse_toc_v4(
    words: &[u16],
    expected_ordinal: u16,
    role: &str,
) -> Result<DictionarySection, DictionaryReadError> {
    let mut cursor = subject_end(words, &CONTENTS_SUBJECT)?;
    let ordinal = take_number(words, &mut cursor)?;
    expect_word(words, &mut cursor, ACCUSATIVE)?;
    let range = take_vector(words, &mut cursor)?;
    expect_word(words, &mut cursor, SOURCE)?;
    expect_end(words, &mut cursor)?;
    if ordinal != expected_ordinal || range.len() != 4 {
        return Err(DictionaryReadError::InvalidRecord(
            "v4 TOC ordinal or range shape is invalid",
        ));
    }
    Ok(DictionarySection {
        role: role.to_owned(),
        codelet_start: range[0] as u32 | ((range[1] as u32) << 16),
        codelet_count: range[2] as u32 | ((range[3] as u32) << 16),
    })
}
pub(crate) fn parse_toc(words: &[u16]) -> Result<DictionarySection, DictionaryReadError> {
    let mut cursor = subject_end(words, &CONTENTS_SUBJECT)?;
    let role = take_text(words, &mut cursor)?;
    expect_word(words, &mut cursor, ACCUSATIVE)?;
    let range = take_vector(words, &mut cursor)?;
    expect_word(words, &mut cursor, SOURCE)?;
    expect_end(words, &mut cursor)?;
    if range.len() != 4 {
        return Err(DictionaryReadError::InvalidRecord(
            "TOC range must contain four uint16 words",
        ));
    }
    Ok(DictionarySection {
        role,
        codelet_start: range[0] as u32 | ((range[1] as u32) << 16),
        codelet_count: range[2] as u32 | ((range[3] as u32) << 16),
    })
}
pub(crate) fn parse_location(
    words: &[u16],
) -> Result<(Vec<u16>, Range<u32>, Range<u32>), DictionaryReadError> {
    let mut cursor = subject_end(words, &INDEX_SUBJECT)?;
    if take_text(words, &mut cursor)? != "entry-location" {
        return Err(DictionaryReadError::InvalidRecord(
            "expected an entry-location posting",
        ));
    }
    expect_word(words, &mut cursor, ACCUSATIVE)?;
    let key = take_vector(words, &mut cursor)?;
    expect_word(words, &mut cursor, SOURCE)?;
    let range = take_vector(words, &mut cursor)?;
    expect_word(words, &mut cursor, DESTINATION)?;
    let definition_words = take_vector(words, &mut cursor)?;
    expect_word(words, &mut cursor, COMITATIVE)?;
    expect_end(words, &mut cursor)?;
    if key.is_empty() || range.len() != 4 || definition_words.len() != 4 {
        return Err(DictionaryReadError::InvalidRecord(
            "invalid entry-location key or ranges",
        ));
    }
    let decode_range = |range: &[u16]| -> Result<Range<u32>, DictionaryReadError> {
        let start = range[0] as u32 | ((range[1] as u32) << 16);
        let count = range[2] as u32 | ((range[3] as u32) << 16);
        Ok(start
            ..start
                .checked_add(count)
                .ok_or(DictionaryReadError::InvalidRecord("entry range overflow"))?)
    };
    let entry_range = decode_range(&range)?;
    let definition_range = decode_range(&definition_words)?;
    if entry_range.is_empty()
        || definition_range.start < entry_range.start
        || definition_range.end > entry_range.end
    {
        return Err(DictionaryReadError::InvalidRecord(
            "definition range is outside its dictionary entry",
        ));
    }
    Ok((key, entry_range, definition_range))
}
