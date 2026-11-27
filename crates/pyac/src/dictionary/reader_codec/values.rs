//! Reading a value: text frames, vectors, numbers.

use super::super::reader::DictionaryReadError;
use super::super::words::{NAME_MARKER, NOMINATIVE, REALIS, TEXT_QUOTE, TEXT_VERB, UINT32_QUOTE};
use crate::literal::decode_quote_header;

pub(crate) fn subject_end(words: &[u16], expected: &[u16]) -> Result<usize, DictionaryReadError> {
    if words.len() < expected.len() + 2
        || words[0] != NAME_MARKER
        || words[1..1 + expected.len()] != *expected
    {
        return Err(DictionaryReadError::InvalidRecord(
            "sentence has an unexpected Pyash subject",
        ));
    }
    // `expected` is a fixed constant that never contains a structural marker, so
    // the value check is unambiguous here. Variable dict keys go through
    // `take_record_subject`, which uses the boundary flags instead.
    let case = 1 + expected.len();
    if words[case] != NOMINATIVE {
        return Err(DictionaryReadError::InvalidRecord(
            "subject is missing its nominative case boundary",
        ));
    }
    Ok(case + 1)
}
pub(crate) fn take_text(words: &[u16], cursor: &mut usize) -> Result<String, DictionaryReadError> {
    if words.get(*cursor) != Some(&TEXT_QUOTE) {
        return Err(DictionaryReadError::InvalidRecord(
            "expected a typed text quote",
        ));
    }
    let count = *words
        .get(*cursor + 1)
        .ok_or(DictionaryReadError::InvalidRecord("truncated text length"))?
        as usize;
    let end = cursor
        .checked_add(2 + count)
        .ok_or(DictionaryReadError::InvalidRecord("text length overflow"))?;
    let payload = words
        .get(*cursor + 2..end)
        .ok_or(DictionaryReadError::InvalidRecord("truncated text payload"))?;
    let value = String::from_utf16(payload)
        .map_err(|_| DictionaryReadError::InvalidRecord("text quote has invalid UTF-16"))?;
    *cursor = end;
    Ok(value)
}
pub(crate) fn take_vector(
    words: &[u16],
    cursor: &mut usize,
) -> Result<Vec<u16>, DictionaryReadError> {
    let header_word = *words
        .get(*cursor)
        .ok_or(DictionaryReadError::InvalidRecord("missing vector quote"))?;
    let header = decode_quote_header(header_word).ok_or(DictionaryReadError::InvalidRecord(
        "invalid vector quote header",
    ))?;
    if header.sort != 4 || header.scalar_bytes != 2 {
        return Err(DictionaryReadError::InvalidRecord(
            "expected a uint16 vector quote",
        ));
    }
    *cursor += 1;
    let count = if header.explicit_vector_length {
        let count = *words
            .get(*cursor)
            .ok_or(DictionaryReadError::InvalidRecord("missing vector length"))?
            as usize;
        *cursor += 1;
        count
    } else {
        header
            .vector_words
            .ok_or(DictionaryReadError::InvalidRecord(
                "unsupported vector thickness",
            ))?
    };
    let end = cursor
        .checked_add(count)
        .ok_or(DictionaryReadError::InvalidRecord("vector length overflow"))?;
    let values = words
        .get(*cursor..end)
        .ok_or(DictionaryReadError::InvalidRecord(
            "truncated vector payload",
        ))?
        .to_vec();
    *cursor = end;
    Ok(values)
}
pub(crate) fn take_uint32(words: &[u16], cursor: &mut usize) -> Result<u32, DictionaryReadError> {
    expect_word(words, cursor, UINT32_QUOTE)?;
    let low = *words
        .get(*cursor)
        .ok_or(DictionaryReadError::InvalidRecord("truncated uint32 quote"))?;
    let high = *words
        .get(*cursor + 1)
        .ok_or(DictionaryReadError::InvalidRecord("truncated uint32 quote"))?;
    *cursor += 2;
    Ok(low as u32 | ((high as u32) << 16))
}
pub(crate) fn take_uint32_vector(
    words: &[u16],
    cursor: &mut usize,
) -> Result<Vec<u32>, DictionaryReadError> {
    let header_word = *words
        .get(*cursor)
        .ok_or(DictionaryReadError::InvalidRecord(
            "missing uint32 vector quote",
        ))?;
    let header = decode_quote_header(header_word).ok_or(DictionaryReadError::InvalidRecord(
        "invalid uint32 vector quote header",
    ))?;
    if header.sort != 4 || header.scalar_bytes != 4 {
        return Err(DictionaryReadError::InvalidRecord(
            "expected a uint32 vector quote",
        ));
    }
    *cursor += 1;
    let count = if header.explicit_vector_length {
        let count = *words
            .get(*cursor)
            .ok_or(DictionaryReadError::InvalidRecord(
                "missing uint32 vector length",
            ))? as usize;
        *cursor += 1;
        count
    } else {
        header
            .vector_words
            .ok_or(DictionaryReadError::InvalidRecord(
                "unsupported uint32 vector thickness",
            ))?
    };
    let word_count = count
        .checked_mul(2)
        .ok_or(DictionaryReadError::InvalidRecord(
            "uint32 vector length overflow",
        ))?;
    let end = cursor
        .checked_add(word_count)
        .ok_or(DictionaryReadError::InvalidRecord(
            "uint32 vector length overflow",
        ))?;
    let payload = words
        .get(*cursor..end)
        .ok_or(DictionaryReadError::InvalidRecord(
            "truncated uint32 vector payload",
        ))?;
    let values = payload
        .chunks_exact(2)
        .map(|pair| pair[0] as u32 | ((pair[1] as u32) << 16))
        .collect();
    *cursor = end;
    Ok(values)
}
pub(crate) fn expect_word(
    words: &[u16],
    cursor: &mut usize,
    expected: u16,
) -> Result<(), DictionaryReadError> {
    if words.get(*cursor) != Some(&expected) {
        return Err(DictionaryReadError::UnexpectedWord {
            expected,
            actual: words.get(*cursor).copied(),
            offset: *cursor,
        });
    }
    *cursor += 1;
    Ok(())
}
pub(crate) fn expect_end(words: &[u16], cursor: &mut usize) -> Result<(), DictionaryReadError> {
    expect_word(words, cursor, TEXT_VERB)?;
    expect_word(words, cursor, REALIS)?;
    if *cursor != words.len() {
        return Err(DictionaryReadError::InvalidRecord(
            "record has trailing words",
        ));
    }
    Ok(())
}
