//! The lwonprom port, pinned against real records from the published volume.
//!
//! `encode_sentence` is the ported tablet parser. A record is
//! `gi <key> na <value> <case> … hwus li`; text values are `zi.kyit. … .kyit.zi`.
//! The writer marks boundary bits positively in a sentence's final codelet and
//! negatively in earlier ones (`reader_codec::slot_is_boundary`), so a sentence
//! longer than one codelet exercises that rule.
//!
//! These pin the exact codelets the dictionary writer produced, so a regression in
//! splitting, polarity, or vector headers fails here rather than everywhere.

use pyac::dictionary::DictionaryFileReader;
use pyac_vliw_core::text_encode::{CODELET_WIDTH, encode_sentence};

use super::fixtures;

#[test]
fn a_two_codelet_record_reproduces_the_published_codelets() {
    let Some(path) = fixtures::reference_volume() else {
        return;
    };
    let mut reader = DictionaryFileReader::open(std::fs::File::open(path).expect("open volume"))
        .expect("read volume");
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

#[test]
fn a_vector_record_reproduces_the_published_codelet() {
    let Some(path) = fixtures::reference_volume() else {
        return;
    };
    let mut reader = DictionaryFileReader::open(std::fs::File::open(path).expect("open volume"))
        .expect("read volume");
    let original = reader
        .read_entry(&[0x0801, 0x280e])
        .expect("entry myim ryik");

    // The first record is lexical-form text; the component vector record is later.
    // Render just the component record and compare it to its own codelet.
    let line = "gi myim ryik na ve 1 2049 so hwus li";
    let mut out = [[0u32; CODELET_WIDTH]; 8];
    let count = encode_sentence(line, &mut out).expect("parses");
    assert_eq!(count, 1);

    // Find the codelet in the entry that holds 0x8e1d (the vector header).
    let vector_codelet = original
        .iter()
        .find(|codelet| codelet.contains(&0x8e1d))
        .expect("a vector codelet in the entry");
    let want: [u32; CODELET_WIDTH] = std::array::from_fn(|slot| vector_codelet[slot] as u32);
    assert_eq!(
        out[0], want,
        "vector record differs\n  want {want:04x?}\n  got  {:04x?}",
        out[0]
    );
}
