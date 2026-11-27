//! Literals: `zi` text frames, `ve` vectors, and the word reader.
//!
//! `zi` is for strings and binary only. A payload word that looks like a quote
//! cannot move the boundary, which is why a frame is parsed whole rather than
//! scanned word by word.

use super::super::word_code;
use super::builder::*;
use super::errors::*;
use super::glyph::*;
use super::numbers::*;
use super::words::*;
/// Find a `zi.<kind>.<payload>.<kind>.zi` frame at the start of `text`.
///
/// Returns `(consumed_bytes, kind, payload_bytes)`. `None` when the run is not a
/// complete frame, so a bare `zi` word still parses as itself. The payload is the
/// raw bytes between the second and third dots; `encode_zi_text` unescapes it.
pub(super) fn parse_zi_frame(text: &[u8]) -> Option<(usize, &[u8], &[u8])> {
    let rest = text.strip_prefix(b"zi.")?;
    let dot = rest.iter().position(|byte| *byte == b'.')?;
    if dot == 0 {
        return None;
    }
    let kind = &rest[..dot];
    let payload_start = dot + 1;
    let mut index = payload_start;
    while index + kind.len() + 4 <= rest.len() {
        if rest[index] == b'.'
            && &rest[index + 1..index + 1 + kind.len()] == kind
            && rest[index + 1 + kind.len()..].starts_with(b".zi")
        {
            let payload = &rest[payload_start..index];
            let consumed = 3 + index + 1 + kind.len() + 3;
            return Some((consumed, kind, payload));
        }
        index += 1;
    }
    None
}
/// Encode a text quote payload: `TEXT_QUOTE`, the UTF-16 unit count, then units.
///
/// A backslash escapes the next character, so a payload can carry the frame's own
/// closing marker literally. Length is counted in UTF-16 units, matching the
/// dictionary writer. Fifteen or more units spill into raw continuation codelets
/// where every slot is payload, as `pack_words` writes and the reader expects.
pub(super) fn encode_zi_text(builder: &mut Builder<'_>, payload: &[u8]) -> Result<(), ParseError> {
    let text = core::str::from_utf8(payload).map_err(|_| ParseError::BadGlyph)?;
    let count = zi_unit_count(text)?;
    builder.push(TEXT_QUOTE, false)?;
    builder.push(count, false)?;
    let raw = count > 14;
    if raw {
        builder.begin_raw_region()?;
    }
    let mut chars = text.chars();
    while let Some(character) = chars.next() {
        let value = if character == '\\' {
            chars.next().ok_or(ParseError::BadGlyph)?
        } else {
            character
        };
        let mut buffer = [0u16; 2];
        for unit in value.encode_utf16(&mut buffer) {
            if raw {
                builder.push_raw(u32::from(*unit))?;
            } else {
                builder.push(u32::from(*unit), false)?;
            }
        }
    }
    if raw {
        builder.end_raw_region()?;
    }
    Ok(())
}
/// The UTF-16 unit count of an (optionally backslash-escaped) frame payload.
pub(super) fn zi_unit_count(text: &str) -> Result<u32, ParseError> {
    let mut count = 0u32;
    let mut chars = text.chars();
    while let Some(character) = chars.next() {
        let value = if character == '\\' {
            chars.next().ok_or(ParseError::BadGlyph)?
        } else {
            character
        };
        count += value.len_utf16() as u32;
    }
    Ok(count)
}
/// The header word for a vector of `count` lanes, and whether the count must
/// follow it. `sort` is 4 (uint) or 5 (int); `scalar_bytes` is 1, 2, 4 or 8.
/// Mirrors `vector_quote_word` in the dictionary writer, generalized to width and
/// sign so `pu ve` (uint32) and `tc6h pu ve` (int32) type the same way.
pub(super) fn vector_header(count: usize, scalar_bytes: u32, sort: u32) -> (u32, bool) {
    let (thickness, explicit) = match count {
        2 => (1u32, false),
        4 => (2, false),
        8 => (3, false),
        16 => (4, false),
        3 => (7, false),
        _ => (6, true),
    };
    let scalar_thickness = match scalar_bytes {
        1 => 0u32,
        2 => 1,
        4 => 2,
        8 => 3,
        _ => 1,
    };
    (
        (sort << 13) | (scalar_thickness << 11) | (thickness << 8) | 0x001d,
        explicit,
    )
}
/// Scalar width, in bytes, named by a width word. `do`/`ve` are 16-bit.
pub(super) fn vector_width(word: u32) -> Option<u32> {
    match word {
        0x309e => Some(1), // po   — 8-bit
        0x327e => Some(2), // do   — 16-bit
        0x289e => Some(4), // pu   — 32-bit
        0x6b57 => Some(8), // tl6h — 64-bit
        _ => None,
    }
}
/// The signed marker, `tc6h`.
pub(super) const SIGN_WORD: u32 = 0x6d57;
/// The float marker, `fyih`. Before a vector (or inline in a value) it selects
/// the float sort.
pub(super) const FLOAT_WORD: u32 = 0x4067;
/// Peek the next glyph word without consuming the main scan: its HNUC and the
/// index just past it.
pub(super) fn peek_word(letters: &[u8], mut index: usize) -> Option<(u32, usize)> {
    while index < letters.len() && !is_letter(letters[index]) {
        index += 1;
    }
    let rest = &letters[index..];
    let length = first_word_length(rest)?;
    if length == 0 {
        return None;
    }
    let word = core::str::from_utf8(&rest[..length]).ok()?;
    let honuc = u32::from(word_code::encode(word).ok()?);
    Some((honuc, index + length))
}
/// Read a whitespace-delimited decimal integer at `*index`, advancing past it.
pub(super) fn read_decimal(letters: &[u8], index: &mut usize) -> Result<u32, ParseError> {
    while *index < letters.len() && letters[*index].is_ascii_whitespace() {
        *index += 1;
    }
    let start = *index;
    let mut value = 0u32;
    while *index < letters.len() && letters[*index].is_ascii_digit() {
        value = value
            .checked_mul(10)
            .and_then(|acc| acc.checked_add(u32::from(letters[*index] - b'0')))
            .ok_or(ParseError::Unencodable)?;
        *index += 1;
    }
    if *index == start {
        return Err(ParseError::BadGlyph);
    }
    Ok(value)
}
