//! Glyph classification: what a byte is, and where a word ends.
//!
//! Ported from `program/parser.c` `first_word_derive` and `program/analyst.c`
//! `derive_first_word`. A word's length follows from its glyph shapes, which is
//! what makes the scan forward-only.

/// Glyphs that start or continue a consonant.
///
/// The C's `consonant_group` also lists `1`, but the word codec maps `1` to vowel
/// code 6, so classifying it as a consonant here would make words like `l1`
/// unparseable while the codec happily encodes them. The codec's assignment wins:
/// `1` is a vowel.
pub(super) const CONSONANT_GLYPHS: &[u8] = b"ptkfscxbdgvzjnmqrlyw8h.";
/// Vowels, in the codec's order. `1` and `3` are vowel codes the C's
/// `vowel_group` omits but the word codec assigns.
pub(super) const VOWEL_GLYPHS: &[u8] = b"iaueo631";
/// Tone marks. `M` is the unmarked tone for the codec but a real glyph here.
pub(super) const TONE_GLYPHS: &[u8] = b"M72";
pub(super) fn is_consonant(glyph: u8) -> bool {
    CONSONANT_GLYPHS.contains(&glyph)
}
pub(super) fn is_vowel(glyph: u8) -> bool {
    VOWEL_GLYPHS.contains(&glyph)
}
pub(super) fn is_tone(glyph: u8) -> bool {
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
