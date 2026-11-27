//! The lwonprom port must reproduce a real dictionary record byte for byte.
//!
//! `encode_sentence` is the ported tablet parser. A record is
//! `gi <key> na <value> <case> … hwus li`; text values are `zi.kyit. … .kyit.zi`.
//! The writer marks boundary bits positively in a sentence's final codelet and
//! negatively in earlier ones (`reader_codec::slot_is_boundary`), so a sentence
//! longer than one codelet exercises that rule. This pins the exact codelets the
//! dictionary writer produced, so a regression in splitting or polarity fails here.

mod common;

use std::fs::File;

use pyac::dictionary::DictionaryFileReader;
use pyac_vliw_core::text_encode::{CODELET_WIDTH, encode_sentence};

#[test]
fn a_two_codelet_record_reproduces_the_published_codelets() {
    let Some(path) = common::reference_volume_or_skip() else {
        return;
    };
    let mut reader =
        DictionaryFileReader::open(File::open(&path).expect("open volume")).expect("read volume");
    let original = reader.read_entry(&[0x253e]).expect("entry sa"); // "sa"

    // The record, written as Pyac: gi sa na <lang> swih <surface> so <norm> ma hwus li
    let line = "gi sa na \
                zi.kyit.en.kyit.zi swih \
                zi.kyit.you_.kyit.zi so \
                zi.kyit.you_.kyit.zi ma \
                hwus li";

    let mut out = [[0u32; CODELET_WIDTH]; 8];
    let count = encode_sentence(line, &mut out).expect("parses");
    assert_eq!(count, original.len(), "codelet count");

    for (index, expected) in original.iter().enumerate() {
        let want: [u32; CODELET_WIDTH] = std::array::from_fn(|slot| expected[slot] as u32);
        assert_eq!(
            out[index], want,
            "codelet {index} differs\n  want {:04x?}\n  got  {:04x?}",
            want, out[index]
        );
    }
}
