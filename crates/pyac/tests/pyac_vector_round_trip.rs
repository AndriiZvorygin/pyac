//! A real vector-bearing record reproduces byte for byte.
//!
//! Entry `myim ryik` (key `[0x0801, 0x280e]`) carries a component vector.
//! Its first record is `gi <key> na <vector> so hwus li`, written as
//! `gi myim ryik na ve 1 2049 so hwus li`. `ve` is the vector grammar word, so
//! the encoder emits the header `0x8e1d`, the count, then the lane.

mod common;

use std::fs::File;

use pyac::dictionary::DictionaryFileReader;
use pyac_vliw_core::text_encode::{CODELET_WIDTH, encode_sentence};

#[test]
fn a_vector_record_reproduces_the_published_codelet() {
    let Some(path) = common::reference_volume_or_skip() else {
        return;
    };
    let mut reader = DictionaryFileReader::open(File::open(&path).expect("open volume"))
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
