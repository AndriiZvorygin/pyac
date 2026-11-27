//! Reading a clause value: text, number, vector, key sequence.

use super::errors::SourceError;
use super::lexing::{CASES, ClauseValue, unescape_frame};

/// Parse one clause value starting at `index`, returning how many tokens it used.
pub(super) fn parse_value(
    token: &str,
    tokens: &[String],
    index: usize,
    case: &'static str,
    line: usize,
) -> Result<(ClauseValue, usize), SourceError> {
    let value = match token {
        "text" => {
            let raw = tokens.get(index + 1).ok_or(SourceError::Unsupported {
                line,
                text: token.to_owned(),
            })?;
            let parsed: String =
                serde_json::from_str(raw).map_err(|_| SourceError::Unsupported {
                    line,
                    text: raw.to_owned(),
                })?;
            (ClauseValue::Text(parsed), 2)
        }
        _ if token.starts_with("zi.") => (ClauseValue::Text(unescape_frame(token, line)?), 1),
        "num" => {
            let raw = tokens.get(index + 1).ok_or(SourceError::Unsupported {
                line,
                text: token.to_owned(),
            })?;
            let parsed: u32 = raw.parse().map_err(|_| SourceError::Unsupported {
                line,
                text: raw.to_owned(),
            })?;
            (ClauseValue::Number(parsed), 2)
        }
        "ve" => {
            // `ve <count> <words…>` — the count is redundant with the run length,
            // so consume exactly it.
            let count_raw = tokens.get(index + 1).ok_or(SourceError::Unsupported {
                line,
                text: token.to_owned(),
            })?;
            let count: usize = count_raw.parse().map_err(|_| SourceError::Unsupported {
                line,
                text: count_raw.to_owned(),
            })?;
            let mut words = Vec::with_capacity(count);
            for offset in 0..count {
                let raw = tokens
                    .get(index + 2 + offset)
                    .ok_or(SourceError::Unsupported {
                        line,
                        text: token.to_owned(),
                    })?;
                let parsed = parse_vector_word(raw).ok_or_else(|| SourceError::Unsupported {
                    line,
                    text: raw.to_owned(),
                })?;
                words.push(parsed);
            }
            (ClauseValue::Vector(words), count + 2)
        }
        // A bare word after a name case is the name itself. `su name lyin kwon`
        // carries a two-word name, so consume until the next case word.
        other if case == "su" || case == "name" => {
            let mut words: Vec<&str> = vec![other];
            let mut consumed = 1usize;
            while let Some(next) = tokens.get(index + consumed) {
                if CASES.contains(&next.as_str()) || next == "be" {
                    break;
                }
                words.push(next);
                consumed += 1;
            }
            // `su name X` declares X: `name` is the subject's noun class, not
            // part of the name itself.
            if words.first() == Some(&"name") && words.len() > 1 {
                words.remove(0);
            }
            (ClauseValue::Text(words.join(" ")), consumed)
        }
        other => (ClauseValue::Text(other.to_owned()), 1),
    };
    Ok(value)
}
/// One uint16 in a `ve` run.
///
/// Decimal first, because that is what the source writes: `10590` is the number
/// ten thousand five hundred and ninety, not an oversized hex literal. A `0x`
/// prefix or a value too large for decimal is read as hex, matching the
/// JavaScript parser this replaces.
pub(super) fn parse_vector_word(word: &str) -> Option<u16> {
    let trimmed = word.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return u16::from_str_radix(hex, 16).ok();
    }
    if let Ok(value) = trimmed.parse::<u16>() {
        return Some(value);
    }
    u16::from_str_radix(trimmed, 16).ok()
}
pub(super) fn take_sequence(words: &[u16], cursor: &mut usize) -> Option<Vec<u16>> {
    let count = *words.get(*cursor)? as usize;
    *cursor += 1;
    let end = cursor.checked_add(count)?;
    let sequence = words.get(*cursor..end)?.to_vec();
    *cursor = end;
    Some(sequence)
}
