//! The container header and the codelet reader.

use super::super::reader::DictionaryReadError;
use crate::{CODELET_BYTES, FORMAT_VERSION, HEADER_BYTES, MAGIC, PyacError, WORDS_PER_CODELET};
use std::io::{Read, Seek, SeekFrom};

pub(crate) fn validate_header(header: &[u8; HEADER_BYTES]) -> Result<u32, DictionaryReadError> {
    if &header[..8] != MAGIC {
        return Err(PyacError::InvalidMagic.into());
    }
    let u16_at = |offset| u16::from_le_bytes([header[offset], header[offset + 1]]);
    let u32_at =
        |offset| u32::from_le_bytes(header[offset..offset + 4].try_into().expect("header field"));
    let version = u16_at(8);
    if version != FORMAT_VERSION {
        return Err(PyacError::UnsupportedVersion(version).into());
    }
    let word_bits = u16_at(10);
    let words = u16_at(12);
    if word_bits != 16 || words != WORDS_PER_CODELET as u16 {
        return Err(PyacError::UnsupportedWordLayout {
            word_bits,
            words_per_codelet: words,
        }
        .into());
    }
    let header_bytes = u16_at(14);
    let count = u32_at(16);
    let codelet_bytes = u32_at(20);
    let total_bytes = u32_at(24);
    let reserved = u32_at(28);
    let payload_bytes =
        (count as usize)
            .checked_mul(CODELET_BYTES)
            .ok_or(PyacError::LengthMismatch {
                expected: usize::MAX,
                declared: total_bytes,
                actual: 0,
            })?;
    let expected = HEADER_BYTES
        .checked_add(payload_bytes)
        .ok_or(PyacError::LengthMismatch {
            expected: usize::MAX,
            declared: total_bytes,
            actual: 0,
        })?;
    if header_bytes as usize != HEADER_BYTES
        || codelet_bytes as usize != CODELET_BYTES
        || reserved != 0
    {
        return Err(PyacError::InvalidHeader {
            header_bytes,
            codelet_bytes,
            reserved,
        }
        .into());
    }
    if total_bytes as usize != expected {
        return Err(PyacError::LengthMismatch {
            expected,
            declared: total_bytes,
            actual: expected,
        }
        .into());
    }
    Ok(count)
}
pub(crate) fn read_codelet<R: Read + Seek>(
    source: &mut R,
    ordinal: u32,
) -> Result<[u16; WORDS_PER_CODELET], DictionaryReadError> {
    let offset = HEADER_BYTES as u64 + ordinal as u64 * CODELET_BYTES as u64;
    source.seek(SeekFrom::Start(offset))?;
    let mut bytes = [0; CODELET_BYTES];
    source.read_exact(&mut bytes)?;
    let mut words = [0; WORDS_PER_CODELET];
    for (index, word) in words.iter_mut().enumerate() {
        *word = u16::from_le_bytes([bytes[index * 2], bytes[index * 2 + 1]]);
    }
    Ok(words)
}
