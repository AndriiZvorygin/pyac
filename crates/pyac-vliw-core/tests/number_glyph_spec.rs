//! Numeric literal glyph forms, per the owner's spec:
//!
//! ```text
//! do hyik tyut         -> uint16(12)    `do` = 16-bit
//! do hyik fyih tyut    -> float16(1.2)  digits, `fyih` = the fractional separator
//! tc6h do hyik tyut    -> int16(12)     `tc6h` = signed
//! po ...               ->  8-bit
//! do ...               -> 16-bit
//! pu ...               -> 32-bit
//! tl6h ...             -> 64-bit
//! ```
//!
//! Numbers are written in decimal digit words. The digit glyphs are `zron`..`twun`
//! for 0..9 in decimal reading order, so `hyik tyut` is twelve, not `1*6+2`.
//!
//! This is the typed-glyph spec for numeric literals. `zi` framing is for strings
//! and binary only and must never carry a number.

use pyac_vliw_core::text_encode::{CODELET_WIDTH, encode_sentence};

fn value_of(text: &str) -> u32 {
    let mut codelets = [[0u32; CODELET_WIDTH]; 4];
    let count = encode_sentence(text, &mut codelets).expect("encodes");
    let flat: Vec<u32> = codelets[..count]
        .iter()
        .flat_map(|c| c.iter().copied())
        .collect();
    // The number quote is 0x881d and its value follows.
    let quote = flat
        .iter()
        .position(|word| *word == 0x881d)
        .expect("a number quote");
    flat[quote + 1]
}

#[test]
fn decimal_digit_words_read_as_decimal() {
    // `hyik tyut` is twelve. The old default read it base 6 and produced 8.
    assert_eq!(value_of("dohyiktyut"), 12);
    assert_eq!(value_of("dohyik"), 1);
    assert_eq!(value_of("dohyikzron"), 10);
    assert_eq!(value_of("dohfak"), 5);
    // Positions carry weight ten, so `hyik tyut tyin` is one hundred twenty-three.
    assert_eq!(value_of("dohyiktyuttyin"), 123);
}

#[test]
fn the_bit_width_words_name_a_width() {
    // po=8, do=16, pu=32, tl6h=64. The width word is not folded into the value.
    for (text, expected) in [
        ("pohyiktyut", 12u32),
        ("dohyiktyut", 12),
        ("puhyiktyut", 12),
        ("tl6hhyiktyut", 12),
    ] {
        assert_eq!(value_of(text), expected, "{text}");
    }
}

#[test]
fn the_signed_word_marks_the_literal_and_is_not_the_value() {
    // tc6h = int16; it types the literal, it is not a digit.
    assert_eq!(value_of("tc6hdohyiktyut"), 12);
}
