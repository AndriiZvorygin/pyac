//! `ve` is the vector grammar word (0x2EDE); it is a directive, not a payload
//! word. Like `do` (which emits NUMBER_QUOTE rather than itself), `ve` emits the
//! vector quote header plus the count and lanes, so a record's component vector
//! round-trips through the glyph text.
//!
//! The header is chosen from the lane count exactly as the dictionary writer does
//! (`vector_quote_word`): 2→thickness 1, 4→2, 8→3, 16→4, 3→7, otherwise 6
//! (explicit count). Scalar width 2 bytes.

use pyac_vliw_core::text_encode::{CODELET_WIDTH, encode_sentence};

fn words(text: &str) -> Vec<u32> {
    let mut codelets = [[0u32; CODELET_WIDTH]; 8];
    let count = encode_sentence(text, &mut codelets).unwrap_or_else(|error| {
        panic!("{text:?} does not parse: {error:?}");
    });
    codelets[..count]
        .iter()
        .flat_map(|codelet| [codelet[0]].into_iter().chain(codelet[1..].iter().copied()))
        .collect()
}

#[test]
fn ve_emits_a_single_lane_vector_header() {
    // `ve 1 2049` -> header 0x8e1d (sort 4, scalar 2, explicit), count 1, lane 0x0801
    let flat = words("gi myim ryik na ve 1 2049 so hwus li");
    // Skip the summary word: gi myim ryik na [8e1d 0001 0801] so hwus li
    let body = &flat[1..];
    assert_eq!(
        &body[..10],
        &[
            0x225e, 0x0801, 0x280e, 0x24de, 0x8e1d, 0x0001, 0x0801, 0x313e, 0x8a28, 0x217e
        ]
    );
}

#[test]
fn ve_uses_a_fixed_width_header_for_two_lanes() {
    // Two lanes fit a thickness-2 header with no explicit count: 0x891d.
    // Lanes 27403 (0x6b0b) and 27682 (0x6c22).
    let flat = words("gi sa na ve 2 27403 27682 so hwus li");
    let body = &flat[1..];
    // gi sa na [891d 6b0b 6c22] so hwus li
    assert_eq!(
        &body[..8],
        &[
            0x225e, 0x253e, 0x24de, 0x891d, 0x6b0b, 0x6c22, 0x313e, 0x8a28
        ]
    );
}

#[test]
fn a_width_word_types_the_vector() {
    // `pu ve` is a uint32 vector: sort 4, scalar 4 bytes, thickness 2 -> 0x911d.
    let flat = words("gi sa na pu ve 2 27403 27682 so hwus li");
    let body = &flat[1..];
    assert_eq!(body[3], 0x911d, "uint32 vector header");
    // Lanes are two words each, low then high.
    assert_eq!(&body[4..10], &[0x6b0b, 0, 0x6c22, 0, 0x313e, 0x8a28]);
}

#[test]
fn a_sign_word_types_the_vector_as_signed() {
    // `tc6h pu ve` is an int32 vector: sort 5, scalar 4 bytes, thickness 2.
    // (5 << 13) | (2 << 11) | (1 << 8) | 0x1d = 0xb11d.
    let flat = words("gi sa na tc6h pu ve 2 27403 27682 so hwus li");
    let body = &flat[1..];
    assert_eq!(body[3], 0xb11d, "int32 vector header");
}

#[test]
fn a_float_word_types_the_vector_as_float() {
    // `fyih ve` is a float16 vector: sort 6, scalar 2 bytes, thickness 2.
    // (6 << 13) | (1 << 11) | (1 << 8) | 0x1d = 0xc91d.
    let flat = words("gi sa na fyih ve 2 27403 27682 so hwus li");
    let body = &flat[1..];
    assert_eq!(body[3], 0xc91d, "float16 vector header");

    // `fyih pu ve` is float32: sort 6, scalar 4 bytes, thickness 2 =
    // (6 << 13) | (2 << 11) | (1 << 8) | 0x1d = 0xd11d.
    let flat = words("gi sa na fyih pu ve 2 27403 27682 so hwus li");
    let body = &flat[1..];
    assert_eq!(body[3], 0xd11d, "float32 vector header");
}
