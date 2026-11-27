//! Pack execution, driven concurrently and against golden codelets.

use crate::*;
use std::sync::{Arc, Barrier};
use std::time::Duration;

const ONE_PLUS: [u16; WORDS_PER_CODELET] = [
    0x0149, 0x881d, 0x0001, 0x245e, 0x881d, 0x0001, 0x243e, 0x8a64, 0x295e, 0, 0, 0, 0, 0, 0, 0,
];

#[test]
fn independent_cpu_sections_are_returned_in_completion_order() {
    let start = Arc::new(Barrier::new(3));
    let completed = map_independent_sections(3, 3, {
        let start = Arc::clone(&start);
        move |section_index| {
            start.wait();
            let delay_ms = [40, 1, 20][section_index];
            std::thread::sleep(Duration::from_millis(delay_ms));
            section_index
        }
    });
    assert_eq!(completed, vec![1, 2, 0]);
}

#[test]
fn encodes_the_shared_one_plus_one_fixture() {
    let bytes = encode_pack(&[ONE_PLUS]).expect("fixture fits");
    assert_eq!(&bytes[..8], b"PYACVLIW");
    assert_eq!(read_u16(&bytes, 8), 1);
    assert_eq!(read_u16(&bytes, 10), 16);
    assert_eq!(read_u16(&bytes, 12), 16);
    assert_eq!(read_u16(&bytes, 14), 32);
    assert_eq!(read_u32(&bytes, 16), 1);
    assert_eq!(read_u32(&bytes, 20), 32);
    assert_eq!(read_u32(&bytes, 24), 64);
    assert_eq!(read_u32(&bytes, 28), 0);
    assert_eq!(&bytes[32..34], &[0x49, 0x01]);
    assert_eq!(&bytes[46..48], &[0x64, 0x8a]);
    assert_eq!(&bytes[48..50], &[0x5e, 0x29]);
}

#[test]
fn decodes_little_endian_codelets_and_round_trips() {
    let bytes = encode_pack(&[ONE_PLUS, [0; WORDS_PER_CODELET]]).expect("fixtures fit");
    let decoded = decode_pack(&bytes).expect("valid pack");
    assert_eq!(decoded.codelets, vec![ONE_PLUS, [0; WORDS_PER_CODELET]]);
    assert_eq!(decoded.bytes, bytes);
    assert_eq!(encode_pack(&decoded.codelets).expect("round trip"), bytes);
}

#[test]
fn rejects_invalid_headers_with_stable_codes() {
    let bytes = encode_pack(&[ONE_PLUS]).expect("fixture fits");

    let mut bad_magic = bytes.clone();
    bad_magic[0] = b'X';
    assert_eq!(decode_pack(&bad_magic).unwrap_err().code(), "invalid-magic");

    let mut bad_version = bytes.clone();
    write_u16(&mut bad_version, 8, 2);
    assert_eq!(
        decode_pack(&bad_version).unwrap_err().code(),
        "unsupported-version"
    );

    let mut bad_layout = bytes.clone();
    write_u16(&mut bad_layout, 10, 8);
    assert_eq!(
        decode_pack(&bad_layout).unwrap_err().code(),
        "unsupported-word-layout"
    );

    let mut bad_reserved = bytes.clone();
    write_u32(&mut bad_reserved, 28, 1);
    assert_eq!(
        decode_pack(&bad_reserved).unwrap_err().code(),
        "invalid-header"
    );
}

#[test]
fn rejects_truncation_and_declared_length_mismatch() {
    let bytes = encode_pack(&[ONE_PLUS]).expect("fixture fits");
    assert_eq!(
        decode_pack(&bytes[..31]).unwrap_err().code(),
        "truncated-pack"
    );

    let mut bad_length = bytes.clone();
    write_u32(&mut bad_length, 24, 32);
    assert_eq!(
        decode_pack(&bad_length).unwrap_err().code(),
        "truncated-pack"
    );
}

#[test]
fn executes_numeric_codelets_after_pack_decode() {
    let mut subtract = ONE_PLUS;
    subtract[2] = 2;
    subtract[5] = 9;
    subtract[7] = 0xabd2;
    let bytes = encode_pack(&[ONE_PLUS, subtract]).expect("fixtures fit");
    let pack = decode_pack(&bytes).expect("valid pack");

    assert_eq!(execute_numeric_arithmetic_pack(&pack), Ok(vec![2, 0]));
}

#[test]
fn preserves_the_failed_codelet_index() {
    let mut malformed = ONE_PLUS;
    malformed[7] = 0x1234;
    let bytes = encode_pack(&[ONE_PLUS, malformed]).expect("fixtures fit");
    let pack = decode_pack(&bytes).expect("valid pack");
    let error = execute_numeric_arithmetic_pack(&pack).expect_err("unknown operator");

    assert_eq!(error.index, 1);
    assert_eq!(error.code(), "unsupported-operator");
}
