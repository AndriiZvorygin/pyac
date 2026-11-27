//! Word identity: the HNUC constants, the width table, and classification.
//!
//! Ported from `program/encoding.c` `process_full_word`. This ladder decides which
//! appender runs, and therefore whether the slot's boundary bit is set.

use super::errors::*;
use super::glyph::*;
use super::numbers::*;
/// Quote header words and the frame marker.
pub(super) const NUMBER_QUOTE: u32 = 0x881d;
/// Opens a text quote: the length, then the UTF-16 units.
pub(super) const TEXT_QUOTE: u32 = 0x0e1d;
pub(super) const NUMBER_WORD: u32 = 0x327e;
pub(super) const REFERENTIAL_WORD: u32 = 0xc8c4;
/// A vector grammar word (`ve`): `ve <count> <lanes…>` emits a uint16 vector
/// quote header, the count when the header form needs it, then the lanes — the
/// same layout `vector_quote_word` writes.
pub(super) const VECTOR_WORD: u32 = 0x2ede;
/// The name noun class (`gi`). It introduces a name: every word after it is name
/// payload, whatever its class, until `na` closes the name.
pub(super) const NAME_WORD: u32 = 0x225e;
/// The nominative case (`na`), which closes a name and marks a field boundary.
pub(super) const NOMINATIVE_WORD: u32 = 0x24de;
/// The word quote (`wo`). It marks the word after it as data: `wo li` is the word
/// `li` as a value, not the realis mood. Both words are pushed as non-boundary.
pub(super) const WORD_QUOTE: u32 = 0x30be;
/// Bit-width words that type a numeric literal: `po` 8, `do` 16, `pu` 32, `tl6h`
/// 64. `do` is the same word the literal already opened with, so it is not listed
/// twice; a width word opens the literal exactly as `do` does.
///
/// The width is carried by the type word itself on the codelet today, so the
/// number quote's value is unchanged. Wider and narrower widths select which
/// codelet profile the literal belongs to; that profile is not yet emitted here.
pub(super) const WIDTH_WORDS: &[u32] = &[
    0x309e, // po   — 8-bit
    0x289e, // pu   — 32-bit
    0x6b57, // tl6h — 64-bit
];
/// The classification a derived word falls into, which picks the appender.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WordClass {
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
pub(super) const CASE_WORDS: &[u32] = &[
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
pub(super) const PERSPECTIVE_WORDS: &[u32] = &[
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
pub(super) const NUMBER_DIGITS: &[(u32, u32)] = &[
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
pub(super) fn classify(honuc: u32) -> WordClass {
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
