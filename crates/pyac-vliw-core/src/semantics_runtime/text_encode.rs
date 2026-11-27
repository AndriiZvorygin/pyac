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

/// Payload slots per codelet; slot 0 is the summary word.
pub const CODELET_PAYLOAD: usize = 15;
/// A full codelet, summary word included.
pub const CODELET_WIDTH: usize = CODELET_PAYLOAD + 1;

/// Glyphs that start or continue a consonant.
///
/// The C's `consonant_group` also lists `1`, but the word codec maps `1` to vowel
/// code 6, so classifying it as a consonant here would make words like `l1`
/// unparseable while the codec happily encodes them. The codec's assignment wins:
/// `1` is a vowel.
const CONSONANT_GLYPHS: &[u8] = b"ptkfscxbdgvzjnmqrlyw8h.";
/// Vowels, in the codec's order. `1` and `3` are vowel codes the C's
/// `vowel_group` omits but the word codec assigns.
const VOWEL_GLYPHS: &[u8] = b"iaueo631";
/// Tone marks. `M` is the unmarked tone for the codec but a real glyph here.
const TONE_GLYPHS: &[u8] = b"M72";

/// Quote header words and the frame marker.
const NUMBER_QUOTE: u32 = 0x881d;
/// Opens a text quote: the length, then the UTF-16 units.
const TEXT_QUOTE: u32 = 0x0e1d;
const NUMBER_WORD: u32 = 0x327e;
const REFERENTIAL_WORD: u32 = 0xc8c4;
/// A vector grammar word (`ve`): `ve <count> <lanes…>` emits a uint16 vector
/// quote header, the count when the header form needs it, then the lanes — the
/// same layout `vector_quote_word` writes.
const VECTOR_WORD: u32 = 0x2ede;
/// The name noun class (`gi`). It introduces a name: every word after it is name
/// payload, whatever its class, until `na` closes the name.
const NAME_WORD: u32 = 0x225e;
/// The nominative case (`na`), which closes a name and marks a field boundary.
const NOMINATIVE_WORD: u32 = 0x24de;

/// Bit-width words that type a numeric literal: `po` 8, `do` 16, `pu` 32, `tl6h`
/// 64. `do` is the same word the literal already opened with, so it is not listed
/// twice; a width word opens the literal exactly as `do` does.
///
/// The width is carried by the type word itself on the codelet today, so the
/// number quote's value is unchanged. Wider and narrower widths select which
/// codelet profile the literal belongs to; that profile is not yet emitted here.
const WIDTH_WORDS: &[u32] = &[
    0x309e, // po   — 8-bit
    0x289e, // pu   — 32-bit
    0x6b57, // tl6h — 64-bit
];

/// Why a sentence could not be encoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// A glyph that begins no word.
    BadGlyph,
    /// A word-shaped run the codec could not encode.
    Unencodable,
    /// The caller's codelet buffer filled up.
    CodeletCapacity,
    /// Nothing to parse.
    Empty,
}

fn is_consonant(glyph: u8) -> bool {
    CONSONANT_GLYPHS.contains(&glyph)
}

fn is_vowel(glyph: u8) -> bool {
    VOWEL_GLYPHS.contains(&glyph)
}

fn is_tone(glyph: u8) -> bool {
    TONE_GLYPHS.contains(&glyph)
}

/// A glyph that belongs to a word; anything else separates words.
pub fn is_letter(glyph: u8) -> bool {
    is_consonant(glyph) || is_vowel(glyph) || is_tone(glyph)
}

/// Derive the first word's length from its glyph pattern.
///
/// Mirrors `first_word_derive`: a leading consonant is `CV`, `CVT`, `CCVC` or
/// `CCVT`; a leading vowel is a glottal-stopped word, which the C skips into (see
/// [`encode_sentence`]). Returns `None` when no word starts at `text[0]`.
pub fn first_word_length(text: &[u8]) -> Option<usize> {
    if text.len() < 2 {
        return None;
    }
    let first = text[0];
    if is_vowel(first) {
        // Vowel-initial: the next two glyphs must be consonants, and the C then
        // advances into the word rather than returning a length.
        if text.len() < 3 || !is_consonant(text[1]) || !is_consonant(text[2]) {
            return None;
        }
        return Some(0);
    }
    if !is_consonant(first) {
        return None;
    }
    if is_consonant(text[1]) {
        // CCV(T)C or CCVT
        if text.len() < 3 || !is_vowel(text[2]) {
            return None;
        }
        if text.len() >= 4 && is_tone(text[3]) {
            if text.len() < 5 || !is_consonant(text[4]) {
                return None;
            }
            return Some(5);
        }
        if text.len() >= 4 && is_consonant(text[3]) {
            return Some(4);
        }
        return None;
    }
    if is_vowel(text[1]) {
        // CV or CVT
        if text.len() >= 3 && is_tone(text[2]) {
            return Some(3);
        }
        return Some(2);
    }
    None
}

/// The classification a derived word falls into, which picks the appender.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WordClass {
    /// A numeric literal terminator (`do`), which collects the digit run before it.
    Number,
    /// A number-word digit, buffered until a terminator arrives.
    Digit,
    /// A hash name, appended like a lexical value.
    Referential,
    /// A grammatical case word, appended as grammar (sets a boundary).
    Case,
    /// A mood, appended as grammar (sets a boundary).
    Perspective,
    /// Any other word, appended as a lexical value.
    Lexical,
}

/// Grammatical case words, by HNUC, from `grammaticalCase_word_sequence`.
const CASE_WORDS: &[u32] = &[
    0x24de, // nominative
    0x245e, // accusative
    0x255e, // topic
    0x28be, // vocative
    0x215e, // genitive
    0x243e, // destination
    0x207e, // dative
    0x287e, // instrumental
    0x490f, // comitative
    0x4927, // benefactive
    0x414f, // essive
    0x267e, // adpositional
    0x2d5e, // location
    0x265e, // way
    0x313e, // source
    0x4127, // ablative
    0x4957, // elative
    0x4157, // illative
];

/// Mood words, by HNUC, from `perspective_word_sequence`.
const PERSPECTIVE_WORDS: &[u32] = &[
    0x217e, // realis
    0x295e, // deontic
    0x29be, // conditional
    0x213e, // epistemic
    0x21de, // interrogative
    0x227e, // directive
    0x5217, // declarative
    0x4e27, // paragraph
];

/// Number-word digits, `pyash_number_dictionary` in the C. A digit word's index in
/// this list is its value, so a run of them reads as digits in [`NumberFormat`].
const NUMBER_DIGITS: &[(u32, u32)] = &[
    (0x6cd4, 0),  // zron
    (0x2818, 1),  // hyik
    (0xaa0a, 2),  // tyut
    (0x680a, 3),  // tyin
    (0x8942, 4),  // ksas
    (0x2960, 5),  // hfak
    (0x8858, 6),  // hlis
    (0x4848, 7),  // hsip
    (0x4928, 8),  // hwap
    (0x6a2a, 9),  // twun
    (0x4850, 10), // htip
    (0x6b69, 11), // slen
    (0xa98a, 12), // tfat
    (0x8b4a, 13), // tses
    (0x8b48, 14), // hses
    (0xab20, 15), // hpet
];

/// How a numeric literal's digit words combine into a value.
///
/// Decimal: `hyik tyut` is twelve, because the digit words are the ten decimal
/// digits and a positional run reads in written order. The digit table also holds
/// ten through fifteen (`htip`..`hpet`) for a hexadecimal profile, but those are
/// rejected under the decimal default where the value is not below the radix.
///
/// Big-endian is the default because it reads in the order it is written —
/// `do hyik tyut` is 1*10 + 2 — and because the leading terminator means the
/// digits arrive already in that order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NumberFormat {
    /// Radix. Digit words with a value at or above this are rejected.
    pub base: u32,
    /// Whether the first digit is the most significant.
    pub big_endian: bool,
}

impl NumberFormat {
    /// Decimal, most significant digit first: `do hyik tyut` is twelve.
    ///
    /// Big-endian because it reads in the order it is written and the leading
    /// terminator means the digits arrive already in that order.
    pub const DEFAULT: Self = Self {
        base: 10,
        big_endian: true,
    };

    /// Base 16, most significant digit first. A hexadecimal profile uses the
    /// digit words through `hpet` (15).
    pub const HEX: Self = Self {
        base: 16,
        big_endian: true,
    };

    /// The historical C encoding: base 16, least significant digit first.
    pub const LEGACY_HEX_LITTLE_ENDIAN: Self = Self {
        base: 16,
        big_endian: false,
    };
}

fn number_digit(honuc: u32) -> Option<u32> {
    NUMBER_DIGITS
        .iter()
        .find(|(word, _)| *word == honuc)
        .map(|(_, digit)| *digit)
}

fn classify(honuc: u32) -> WordClass {
    if honuc == NUMBER_WORD {
        WordClass::Number
    } else if honuc == REFERENTIAL_WORD {
        WordClass::Referential
    } else if number_digit(honuc).is_some() {
        // A digit is only a digit after `do` introduces a number; on its own it is
        // an ordinary word (e.g. `hyik` alone is the word "one").
        WordClass::Digit
    } else if CASE_WORDS.contains(&honuc) {
        WordClass::Case
    } else if PERSPECTIVE_WORDS.contains(&honuc) {
        WordClass::Perspective
    } else {
        WordClass::Lexical
    }
}

/// A codelet under construction, writing straight into the caller's buffer.
struct Builder<'a> {
    codelets: &'a mut [[u32; CODELET_WIDTH]],
    count: usize,
    summary: u32,
    used: usize,
}

impl<'a> Builder<'a> {
    fn new(codelets: &'a mut [[u32; CODELET_WIDTH]]) -> Self {
        Self {
            codelets,
            count: 1,
            summary: 0,
            used: 0,
        }
    }

    fn current(&mut self) -> &mut [u32; CODELET_WIDTH] {
        &mut self.codelets[self.count - 1]
    }

    /// Append a payload word, optionally marking it a boundary.
    fn push(&mut self, word: u32, boundary: bool) -> Result<(), ParseError> {
        if self.used == CODELET_PAYLOAD {
            if self.count == self.codelets.len() {
                return Err(ParseError::CodeletCapacity);
            }
            self.seal();
            self.count += 1;
            self.summary = 0;
            self.used = 0;
        }
        let slot = self.used + 1;
        if boundary {
            self.summary |= 1 << slot;
        }
        self.current()[slot] = word;
        self.used += 1;
        Ok(())
    }

    /// Write the summary word for a codelet that continues into a later one.
    ///
    /// The writer marks boundaries **negatively** on an incomplete codelet: a
    /// slot that closes a field has its bit clear and every other used slot has
    /// its bit set. `reader_codec::slot_is_boundary` reads exactly that (`!set`
    /// when incomplete). The completion bit stays clear. The final codelet uses
    /// the positive form instead; see [`Builder::finish`].
    fn seal(&mut self) {
        let used = if self.used == 0 {
            0
        } else {
            ((1u32 << (self.used + 1)) - 1) & !1
        };
        self.current()[0] = used & !self.summary;
    }

    /// Seal the indexed codelet and open a raw continuation region.
    ///
    /// A text quote longer than fourteen units carries its units in codelets whose
    /// every slot is payload — no summary word. `pack_words` chunks them sixteen
    /// words at a time and `read_sentence_with_raw_width` reads sixteen per codelet,
    /// so slot 0 is data here.
    fn begin_raw_region(&mut self) -> Result<(), ParseError> {
        self.seal();
        self.advance_blank()
    }

    /// Close a raw region: start a fresh indexed codelet for the words that follow.
    fn end_raw_region(&mut self) -> Result<(), ParseError> {
        self.advance_blank()
    }

    /// Append a raw payload word, starting a new codelet when the current fills.
    fn push_raw(&mut self, word: u32) -> Result<(), ParseError> {
        if self.used == CODELET_WIDTH {
            if self.count == self.codelets.len() {
                return Err(ParseError::CodeletCapacity);
            }
            self.count += 1;
            self.used = 0;
            let codelet = self.current();
            for slot in codelet.iter_mut() {
                *slot = 0;
            }
        }
        let used = self.used;
        self.current()[used] = word;
        self.used += 1;
        Ok(())
    }

    /// Move to a fresh, zeroed codelet with an empty summary.
    fn advance_blank(&mut self) -> Result<(), ParseError> {
        if self.count == self.codelets.len() {
            return Err(ParseError::CodeletCapacity);
        }
        self.count += 1;
        self.used = 0;
        self.summary = 0;
        let codelet = self.current();
        for slot in codelet.iter_mut() {
            *slot = 0;
        }
        Ok(())
    }

    /// Fold one digit into the numeric literal already on the codelet.
    ///
    /// The literal's words were pushed when `do` opened it, so the value word is
    /// the last one written. `digits` counts how many have been folded already,
    /// tracked explicitly because leading zeros carry no information in the value.
    fn accumulate_number(
        &mut self,
        digit: u32,
        format: NumberFormat,
        digits: &mut u32,
    ) -> Result<(), ParseError> {
        let last = match self.count.checked_sub(1) {
            Some(index) => index,
            None => return Err(ParseError::CodeletCapacity),
        };
        let slot = self.used;
        if slot == 0 {
            return Err(ParseError::Empty);
        }
        let value = self.codelets[last][slot];
        let folded = if format.big_endian {
            value.wrapping_mul(format.base).wrapping_add(digit)
        } else {
            // Little endian: the first digit is least significant, so each new
            // digit is scaled by the base raised to the count already folded.
            let scale = format.base.saturating_pow(*digits);
            value.wrapping_add(digit.wrapping_mul(scale))
        };
        self.codelets[last][slot] = folded;
        *digits = digits.saturating_add(1);
        Ok(())
    }

    /// Seal the last codelet, marking it complete.
    fn finish(mut self) -> usize {
        let summary = self.summary | 1;
        self.current()[0] = summary;
        self.count
    }
}

/// Find a `zi.<kind>.<payload>.<kind>.zi` frame at the start of `text`.
///
/// Returns `(consumed_bytes, kind, payload_bytes)`. `None` when the run is not a
/// complete frame, so a bare `zi` word still parses as itself. The payload is the
/// raw bytes between the second and third dots; `encode_zi_text` unescapes it.
fn parse_zi_frame(text: &[u8]) -> Option<(usize, &[u8], &[u8])> {
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
fn encode_zi_text(builder: &mut Builder<'_>, payload: &[u8]) -> Result<(), ParseError> {
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
fn zi_unit_count(text: &str) -> Result<u32, ParseError> {
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
fn vector_header(count: usize, scalar_bytes: u32, sort: u32) -> (u32, bool) {
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
fn vector_width(word: u32) -> Option<u32> {
    match word {
        0x309e => Some(1), // po   — 8-bit
        0x327e => Some(2), // do   — 16-bit
        0x289e => Some(4), // pu   — 32-bit
        0x6b57 => Some(8), // tl6h — 64-bit
        _ => None,
    }
}

/// The signed marker, `tc6h`.
const SIGN_WORD: u32 = 0x6d57;
/// The float marker, `fyih`. Before a vector (or inline in a value) it selects
/// the float sort.
const FLOAT_WORD: u32 = 0x4067;

/// Peek the next glyph word without consuming the main scan: its HNUC and the
/// index just past it.
fn peek_word(letters: &[u8], mut index: usize) -> Option<(u32, usize)> {
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
fn read_decimal(letters: &[u8], index: &mut usize) -> Result<u32, ParseError> {
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

        // Inside a name (`gi … na`), every word is name payload regardless of its
        // class or whether it looks like a directive. A key can be `li`, `pu`, `ve`
        // or any other grammar word, and as a key it is data. This must run before
        // the vector and number openers so a `ve` or `pu` *key* is not a directive.
        //
        // The name ends at the first boundary word, and `take_record_subject` reads
        // exactly that: the first word whose boundary flag is set. The closing `na`
        // is that word, so it alone takes the boundary here; a key word that happens
        // to be a case or mood word stays data.
        if in_name {
            let closes = honuc == NOMINATIVE_WORD;
            builder.push(honuc, closes)?;
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
mod tests {
    use super::*;

    #[test]
    fn a_cv_word_is_two_glyphs() {
        assert_eq!(first_word_length(b"doka"), Some(2));
    }

    #[test]
    fn a_cvt_word_is_three_glyphs() {
        assert_eq!(first_word_length(b"no7ka"), Some(3));
    }

    #[test]
    fn a_ccvc_word_is_four_glyphs() {
        assert_eq!(first_word_length(b"plustu"), Some(4));
    }

    #[test]
    fn a_ccvt_word_is_five_glyphs() {
        assert_eq!(first_word_length(b"kse7m"), Some(5));
    }

    #[test]
    fn a_vowel_initial_word_asks_the_caller_to_advance() {
        assert_eq!(first_word_length(b"ipla"), Some(0));
    }

    #[test]
    fn separators_are_not_letters() {
        assert!(!is_letter(b' '));
        assert!(is_letter(b'k'));
        assert!(is_letter(b'a'));
        assert!(is_letter(b'7'));
    }
}
