//! Text to codelets: the front half of the PyAC pipeline.
//!
//! Ported from `~/pyac/lwonprom`:
//!
//! - `program/parser.c`   `first_word_derive`   — word boundary by glyph shape
//! - `program/analyst.c`  `derive_first_word`   — the same, copying the word out
//! - `program/encoding.c` `process_full_word`   — classify a derived word
//! - `program/encoding.c` `neo_phrase_encode`   — place it in the phrase
//! - `program/encoding.c` `neo_independentClause_encoding` — the clause loop
//! - `program/sort.c`     `tablet_word_write` / `tablet_grammar_write`
//!
//! The shape of the thing: scan glyphs into a word, derive that word's length
//! from its glyph pattern, encode it to an HNUC, then append it to the codelet
//! being built. A case word or a mood sets the boundary bit for its slot; every
//! other word leaves it clear. Which appender is used decides that, so the
//! classification ladder is the heart of it.
//!
//! Codelet layout mirrors the C: slot 0 is the summary word carrying the
//! boundary bitmap and the completion bit, slots 1..=15 are payload.
//!
//! The crate is `no_std` with no allocator, so this writes into a caller-supplied
//! codelet buffer and returns how many codelets it filled — the same discipline
//! as `decode_sentence`.

use super::word_code;

#[path = "text_encode/builder.rs"]
mod builder;
#[path = "text_encode/errors.rs"]
mod errors;
#[path = "text_encode/glyph.rs"]
mod glyph;
#[path = "text_encode/literals.rs"]
mod literals;
#[path = "text_encode/numbers.rs"]
mod numbers;
#[path = "text_encode/words.rs"]
mod words;

pub use errors::{CODELET_PAYLOAD, CODELET_WIDTH, ParseError};
pub use glyph::{first_word_length, is_letter};
pub use numbers::NumberFormat;
use numbers::number_digit;

use builder::Builder;
use literals::{
    FLOAT_WORD, SIGN_WORD, encode_zi_text, parse_zi_frame, peek_word, read_decimal, vector_header,
    vector_width, zi_unit_count,
};
use words::{
    CASE_WORDS, NAME_WORD, NOMINATIVE_WORD, NUMBER_QUOTE, NUMBER_WORD, PERSPECTIVE_WORDS,
    REFERENTIAL_WORD, TEXT_QUOTE, VECTOR_WORD, WIDTH_WORDS, WORD_QUOTE, WordClass, classify,
};

/// Encode one PyAC sentence into `codelets`, returning how many were filled.
///
/// The leading-word conventions keep the scan forward-only: `do` introduces a
/// numeric literal and `gi` introduces a name, so neither has to look backwards to
/// find what it applies to. A case word closes the field before it and a mood
/// closes the sentence; both set their slot's boundary bit.
///
/// Uses [`NumberFormat::DEFAULT`]. For the historical C encoding, see
/// [`encode_sentence_with`].
pub fn encode_sentence(
    text: &str,
    codelets: &mut [[u32; CODELET_WIDTH]],
) -> Result<usize, ParseError> {
    encode_sentence_with(text, codelets, NumberFormat::DEFAULT)
}

/// Encode with an explicit number format.
pub fn encode_sentence_with(
    text: &str,
    codelets: &mut [[u32; CODELET_WIDTH]],
    format: NumberFormat,
) -> Result<usize, ParseError> {
    let letters = text.as_bytes();
    if letters.is_empty() {
        return Err(ParseError::Empty);
    }
    if codelets.is_empty() {
        return Err(ParseError::CodeletCapacity);
    }
    let mut builder = Builder::new(codelets);
    let mut index = 0usize;
    // Whether `do` has introduced a numeric literal that is still collecting
    // digits. A leading terminator is what makes the scan forward-only.
    let mut in_number = false;
    let mut number_digits = 0u32;
    // Whether `gi` has opened a name. A name's payload words are *values*, not
    // grammar, even when a payload word is itself a case or mood word: a dictionary
    // key can be `li` (the realis mood) or `na` (the nominative case), and as a key
    // it must not close a field. `na` closes the name and is the only word that
    // ends this state.
    let mut in_name = false;
    // Whether the next word begins a value: true at the sentence start and right
    // after a case word. `gi` opens a name only here — a `gi` inside a lexical word
    // run is just that word, not a name opener. The boundary words themselves set
    // this true; every other word clears it.
    let mut at_value = true;
    // Set by a `wo` just consumed: the next name word is data, so it must not be read
    // as the closing `na`, though it keeps its class boundary bit. That is the whole
    // point of `wo` — `giwona na` is key `na` then the closer, not two closers.
    let mut quoted_next = false;

    while index < letters.len() {
        while index < letters.len() && !is_letter(letters[index]) {
            index += 1;
        }
        if index >= letters.len() {
            break;
        }
        let rest = &letters[index..];

        // A `zi.<kind>.<payload>.<kind>.zi` frame is a text value. It spans the
        // dots and any spaces inside, so it is taken whole before the word scan.
        if let Some((consumed, kind, payload)) = parse_zi_frame(rest) {
            if kind == b"kyit" {
                encode_zi_text(&mut builder, payload)?;
                index += consumed;
                continue;
            }
        }

        let Some(length) = first_word_length(rest) else {
            return Err(ParseError::BadGlyph);
        };
        if length == 0 {
            // Vowel-initial: step past the leading vowel and re-derive.
            index += 1;
            continue;
        }
        let word = core::str::from_utf8(&rest[..length]).map_err(|_| ParseError::BadGlyph)?;
        let honuc = u32::from(word_code::encode(word).map_err(|_| ParseError::Unencodable)?);
        index += length;

        // `wo` word-quotes the root after it: it emits **no word itself**. Outside a
        // name it suppresses the case reading of the following root; inside a name it
        // flags that root as data so a key `na` is not taken for the closer. A `wo`
        // used *as a key* is not a directive — inside a name every word is data, so
        // this check must come after the name handling below.
        if honuc == WORD_QUOTE && !in_name {
            let Some((next, after)) = peek_word(letters, index) else {
                return Err(ParseError::BadGlyph);
            };
            let boundary = matches!(classify(next), WordClass::Case | WordClass::Perspective);
            builder.push(next, boundary)?;
            index = after;
            continue;
        }

        // Inside a name (`gi … na`), every word is name payload regardless of its
        // class or whether it looks like a directive. A key can be `li`, `pu`, `ve`
        // or any other grammar word, and as a key it is data. This must run before
        // the vector and number openers so a `ve` or `pu` *key* is not a directive.
        //
        // The name ends at the first boundary word, and `take_record_subject` reads
        // exactly that: the first word whose boundary flag is set. The closing `na`
        // is that word, so it alone takes the boundary here; a key word that happens
        // to be a case or mood word stays data.
        // A `wo` inside a name flags the next word as quoted data and emits nothing —
        // unless the key *is* `wo`, in which case the renderer writes `wo wo` and the
        // second is the data word. `quoted_next` already distinguishes that.
        if in_name && honuc == WORD_QUOTE && !quoted_next {
            quoted_next = true;
            continue;
        }

        if in_name {
            // A key word is *data*: it never carries a boundary, whatever its class.
            // Only the closing `na` closes the name. `wo` does not change this — it
            // exists so a key that spells a case word is not mistaken for one, which
            // the `quoted_next` flag records for the closer check.
            let closes = honuc == NOMINATIVE_WORD && !quoted_next;
            builder.push(honuc, closes)?;
            quoted_next = false;
            if closes {
                in_name = false;
                at_value = true;
            }
            continue;
        }

        // `gi` opens a name only at a value position: the sentence start or right
        // after a case word. Inside a lexical run (`prifgika` contains `gi`) it is
        // just that word.
        if honuc == NAME_WORD && at_value {
            builder.push(honuc, false)?;
            in_name = true;
            at_value = false;
            continue;
        }

        // A vector literal opens with `ve`, optionally typed: `ve` is uint16,
        // `pu ve` uint32, `tc6h pu ve` int32, and so on with `po`/`tl6h`. Detect
        // the optional sign/width prefix and `ve`, then emit the vector header,
        // the count when the header form needs it, and the lanes. If it is not a
        // vector opener, fall through so a bare width word still opens a number.
        if honuc == VECTOR_WORD
            || honuc == SIGN_WORD
            || honuc == FLOAT_WORD
            || vector_width(honuc).is_some()
        {
            let mut cursor = index;
            // sort: 4 uint, 5 int, 6 float.
            let mut sort = 4u32;
            let mut scalar_bytes = 2u32;
            let mut current = honuc;
            let mut is_vector = false;
            loop {
                if current == SIGN_WORD && sort == 4 {
                    sort = 5;
                    match peek_word(letters, cursor) {
                        Some((next, after)) => {
                            current = next;
                            cursor = after;
                        }
                        None => break,
                    }
                } else if current == FLOAT_WORD && sort == 4 {
                    sort = 6;
                    match peek_word(letters, cursor) {
                        Some((next, after)) => {
                            current = next;
                            cursor = after;
                        }
                        None => break,
                    }
                } else if current == VECTOR_WORD {
                    is_vector = true;
                    break;
                } else if let Some(bytes) = vector_width(current) {
                    scalar_bytes = bytes;
                    match peek_word(letters, cursor) {
                        Some((next, after)) if next == VECTOR_WORD => {
                            cursor = after;
                            is_vector = true;
                            break;
                        }
                        _ => break,
                    }
                } else {
                    break;
                }
            }
            if is_vector {
                let count = read_decimal(letters, &mut cursor)? as usize;
                let (header, explicit) = vector_header(count, scalar_bytes, sort);
                builder.push(header, false)?;
                if explicit {
                    builder.push(count as u32, false)?;
                }
                for _ in 0..count {
                    let lane = read_decimal(letters, &mut cursor)?;
                    // A wide lane is little-endian, low word first, matching
                    // `encode_value` in the dictionary writer.
                    let lane_words = (scalar_bytes / 2) as usize;
                    for word in 0..lane_words.max(1) {
                        let shift = (word as u32) * 16;
                        builder.push((lane >> shift) & 0xffff, false)?;
                    }
                }
                index = cursor;
                continue;
            }
        }

        // `do` (16-bit) opens a numeric literal, and `po`/`pu`/`tl6h` type it at
        // another width. Everything numeric after the opener belongs to that
        // literal, so the digits arrive already in written order.
        let width_word = honuc == NUMBER_WORD || WIDTH_WORDS.contains(&honuc);
        if width_word {
            // A width word that is not `do` also names the type on the codelet.
            if honuc != NUMBER_WORD {
                builder.push(honuc, false)?;
            }
            in_number = true;
            number_digits = 0;
            builder.push(NUMBER_QUOTE, false)?;
            builder.push(0, false)?; // placeholder value, filled as digits arrive
            continue;
        }

        if in_number {
            if let Some(digit) = number_digit(honuc) {
                if digit >= format.base {
                    return Err(ParseError::Unencodable);
                }
                builder.accumulate_number(digit, format, &mut number_digits)?;
                continue;
            }
            in_number = false;
        }

        match classify(honuc) {
            WordClass::Case | WordClass::Perspective => {
                builder.push(honuc, true)?;
                // A case word closes a field, so a value follows it.
                at_value = true;
            }
            _ => {
                builder.push(honuc, false)?;
                at_value = false;
            }
        }
    }

    Ok(builder.finish())
}

#[cfg(test)]
#[path = "text_encode/tests.rs"]
mod tests;
