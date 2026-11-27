//! The ported Pyac parser must handle the `zi` text frame.
//!
//! `text_encode::encode_sentence` is the lwonprom tablet parser. It already
//! handles the record sentence shape — `gi <key> na <value> <case> … hwus li` —
//! because a case word closes a field and a mood closes the sentence, which is
//! exactly the record layout. What it does not yet handle is a text value carried
//! in the `zi` frame, so this pins that gap.

use pyac_vliw_core::text_encode::{CODELET_WIDTH, encode_sentence};

fn first_codelet(text: &str) -> [u32; CODELET_WIDTH] {
    let mut codelets = [[0u32; CODELET_WIDTH]; 8];
    let count = encode_sentence(text, &mut codelets).unwrap_or_else(|error| {
        panic!("{text:?} does not parse: {error:?}");
    });
    assert_eq!(count, 1, "expected one codelet for {text:?}");
    codelets[0]
}

#[test]
fn a_zi_text_frame_parses_to_a_text_quote() {
    // `gi sa na <text "en"> swih … hwus li` — the frame becomes
    // TEXT_QUOTE, length, then the UTF-16 units.
    let codelet = first_codelet("gi sa na zi.kyit.en.kyit.zi swih hwus li");

    assert_eq!(codelet[1], 0x225e, "gi");
    assert_eq!(codelet[2], 0x253e, "sa");
    assert_eq!(codelet[3], 0x24de, "na");
    assert_eq!(codelet[4], 0x0e1d, "text quote header");
    assert_eq!(codelet[5], 2, "length");
    assert_eq!(codelet[6], b'e' as u32);
    assert_eq!(codelet[7], b'n' as u32);
    assert_eq!(codelet[8], 0x414f, "swih");
}

#[test]
fn a_zi_frame_without_text_value_still_works() {
    // The plain vocabulary path is unchanged: `sana pyanli`.
    let codelet = first_codelet("sana pyanli");
    assert_eq!(codelet[..5], [0x0015, 0x253e, 0x24de, 0x6904, 0x217e]);
}
