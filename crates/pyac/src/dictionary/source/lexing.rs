//! Tokenising one line: case words, quoted text, and `zi` frames.

use super::errors::SourceError;
use super::values::parse_value;
use std::collections::HashMap;

/// One clause: the case word and the single value that follows it.
#[derive(Debug, Clone)]
pub(super) enum ClauseValue {
    Text(String),
    Number(u32),
    Vector(Vec<u16>),
}
/// Every case word the source uses, including the two that are not cases in the
/// encoded volume: `su` carries the subject and `name` names it.
pub(super) const CASES: [&str; 10] = [
    "su", "ob", "as", "from", "to", "by", "with", "at", "be", "name",
];
pub(super) fn char_at(chars: &[char], position: usize) -> Option<char> {
    chars.get(position).copied()
}
/// Take a whole `zi.<kind>. <payload> .<kind>.zi` frame starting at `position`.
///
/// Returns the frame text and the index just past it. `None` when the run does
/// not form a frame, so a plain word beginning with `z` still tokenises normally.
pub(super) fn take_zi_frame(chars: &[char], position: usize) -> Option<(String, usize)> {
    let literal = ['z', 'i', '.'];
    if !chars[position..].iter().zip(literal).all(|(a, b)| *a == b) {
        return None;
    }
    let mut index = position + literal.len();
    // The kind root runs to the next dot.
    let kind_start = index;
    while char_at(chars, index).is_some_and(|c| c != '.') {
        index += 1;
    }
    if char_at(chars, index) != Some('.') || index == kind_start {
        return None;
    }
    let kind: String = chars[kind_start..index].iter().collect();
    index += 1; // past the dot opening the payload
    // Scan for the unescaped close marker `.<kind>.zi`.
    let close: Vec<char> = format!(".{kind}.zi").chars().collect();
    let payload_start = index;
    while index < chars.len() {
        if chars[index] == '\\' {
            index += 2; // skip the escaped character
            continue;
        }
        if chars[index..].starts_with(&close) {
            let payload: String = chars[payload_start..index].iter().collect();
            return Some((
                format!("zi.{kind}.{payload}.{kind}.zi"),
                index + close.len(),
            ));
        }
        index += 1;
    }
    None
}
/// Reverse `escape_frame`: a `zi.<kind>. <payload> .<kind>.zi` frame back to text.
pub(super) fn unescape_frame(frame: &str, line: usize) -> Result<String, SourceError> {
    let unsupported = || SourceError::Unsupported {
        line,
        text: frame.to_owned(),
    };
    let rest = frame.strip_prefix("zi.").ok_or_else(unsupported)?;
    let (kind, after) = rest.split_once('.').ok_or_else(unsupported)?;
    let payload = after
        .strip_suffix(&format!(".{kind}.zi"))
        .ok_or_else(unsupported)?;
    let mut out = String::with_capacity(payload.len());
    let mut chars = payload.chars();
    while let Some(character) = chars.next() {
        if character == '\\' {
            out.push(chars.next().ok_or_else(unsupported)?);
        } else {
            out.push(character);
        }
    }
    Ok(out)
}
/// Split one sentence into `su` plus its case clauses.
///
/// The source writes each case word immediately before its value, so a clause
/// runs from a case word to the next one. Quoted text is skipped, so a value that
/// contains a word like `from` is never mistaken for a case.
pub(super) fn parse_line(
    line: &str,
    line_number: usize,
) -> Result<HashMap<&'static str, ClauseValue>, SourceError> {
    let unsupported = || SourceError::Unsupported {
        line: line_number,
        text: line.to_owned(),
    };

    // Tokenise into (case word | bare token) while respecting quotes and `zi`
    // frames. A glyph text frame contains spaces, so it is captured whole.
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_text = false;
    let mut escaped = false;
    let chars: Vec<char> = line.chars().collect();
    let mut position = 0usize;
    while position < chars.len() {
        let character = chars[position];
        if in_text {
            current.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_text = false;
            }
            position += 1;
            continue;
        }
        if character == '"' {
            in_text = true;
            current.push(character);
            position += 1;
            continue;
        }
        if character.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            position += 1;
            continue;
        }
        // A `zi.<kind>.` frame runs to `.<kind>.zi`, spaces included.
        if current.is_empty() && char_at(&chars, position) == Some('z') {
            if let Some((frame, end)) = take_zi_frame(&chars, position) {
                tokens.push(frame);
                position = end;
                continue;
            }
        }
        current.push(character);
        position += 1;
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    if in_text {
        return Err(unsupported());
    }
    // Drop the trailing mood: `ya`.
    if tokens.last().is_some_and(|token| token == "ya") {
        tokens.pop();
    }
    if tokens.first().is_some_and(|token| token == "be") {
        tokens.drain(..2);
    }

    let mut clauses: HashMap<&'static str, ClauseValue> = HashMap::new();
    let mut index = 0usize;
    while index < tokens.len() {
        let token = tokens[index].clone();
        // `be` opens the closing verb phrase, which this parser does not model.
        if token == "be" {
            break;
        }
        let Some(case) = CASES.iter().find(|candidate| **candidate == token) else {
            return Err(unsupported());
        };
        index += 1;
        let Some(value) = tokens.get(index) else {
            return Err(unsupported());
        };
        let (parsed, consumed) = parse_value(value, &tokens, index, *case, line_number)?;
        clauses.insert(*case, parsed);
        index += consumed;
    }
    Ok(clauses)
}
