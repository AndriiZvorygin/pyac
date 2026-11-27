//! Pack execution: source-level conditionals lowered to pack codelets.

use pyac::{PyacValue, WORDS_PER_CODELET, decode_pack, encode_pack, execute_pack};

use super::fixtures::*;

const EQUALS_BRANCH: [[u16; WORDS_PER_CODELET]; 2] = [
    [
        0xd6b6, 0x881d, 5, 0x245e, 0x881d, 5, 0x313e, 0x4809, 0x29be, 0x881d, 7, 0x245e, 0xea30,
        0x217e, 0x881d, 9,
    ],
    [
        0x000b, 0x245e, 0xea30, 0x217e, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
];

const TINY_BOOLEAN_BRANCH: [[u16; WORDS_PER_CODELET]; 2] = [
    // Exact lowering of: ob num 3 be tiny from num 5 then ob bool truth
    // be boolean else ob bool lie be boolean ya.
    [
        0xaab6, 0x881d, 3, 0x245e, 0x881d, 5, 0x313e, 0x6c0d, 0x29be, 0x6909, 0x245e, 0x7271,
        0x217e, 0x680d, 0x245e, 0x7271,
    ],
    [0x0003, 0x217e, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
];

#[test]
fn executes_a_pyash_equal_conditional_and_selects_one_numeric_arm() {
    let equal_pack = encode_pack(&EQUALS_BRANCH).expect("conditional pack fits");
    assert_eq!(
        execute_pack(&decode_pack(&equal_pack).expect("conditional pack decodes")),
        Ok(vec![PyacValue::Unsigned(7)])
    );

    let mut unequal = EQUALS_BRANCH;
    unequal[0][5] = 4;
    let unequal_pack = encode_pack(&unequal).expect("conditional pack fits");
    assert_eq!(
        execute_pack(&decode_pack(&unequal_pack).expect("conditional pack decodes")),
        Ok(vec![PyacValue::Unsigned(9)])
    );

    let mut followed_by_plus = EQUALS_BRANCH.to_vec();
    followed_by_plus.push(PLUS);
    let mixed_pack = encode_pack(&followed_by_plus).expect("mixed pack fits");
    assert_eq!(
        execute_pack(&decode_pack(&mixed_pack).expect("mixed pack decodes")),
        Ok(vec![PyacValue::Unsigned(7), PyacValue::Unsigned(2)])
    );
}

#[test]
fn executes_a_pyash_comparison_branch_with_boolean_results() {
    let bytes = encode_pack(&TINY_BOOLEAN_BRANCH).expect("boolean branch pack fits");
    let result = execute_pack(&decode_pack(&bytes).expect("boolean branch pack decodes"));
    assert_eq!(result, Ok(vec![PyacValue::Boolean(true)]));

    let mut false_branch = TINY_BOOLEAN_BRANCH;
    false_branch[0][2] = 7;
    let bytes = encode_pack(&false_branch).expect("false boolean branch pack fits");
    let result = execute_pack(&decode_pack(&bytes).expect("false boolean branch pack decodes"));
    assert_eq!(result, Ok(vec![PyacValue::Boolean(false)]));
}

#[test]
fn conditional_pack_rejects_an_unimplemented_predicate_shape() {
    let mut unsupported = EQUALS_BRANCH;
    unsupported[0][7] = 0x8a64;
    let bytes = encode_pack(&unsupported).expect("pack fits");
    let error = execute_pack(&decode_pack(&bytes).expect("pack decodes"))
        .expect_err("arithmetic is not a conditional predicate");

    assert_eq!(error.index, 0);
    assert_eq!(error.code(), "malformed-branch");
}

#[test]
fn boolean_composition_executes_nested_boolean_facts() {
    let not_truth = [
        0x02a9, 0x257e, 0x6909, 0x245e, 0x7271, 0x217e, 0x305e, 0x245e, 0x30de, 0x295e, 0, 0, 0, 0,
        0, 0,
    ];
    let and_truth_lie = [
        [
            0xab56, 0x257e, 0x6909, 0x245e, 0x7271, 0x217e, 0x305e, 0x245e, 0x257e, 0x680d, 0x245e,
            0x7271, 0x217e, 0x305e, 0x490f, 0x24be,
        ],
        [0x0003, 0x295e, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ];
    let or_lie_truth = [
        [
            0xab56, 0x257e, 0x680d, 0x245e, 0x7271, 0x217e, 0x305e, 0x245e, 0x257e, 0x6909, 0x245e,
            0x7271, 0x217e, 0x305e, 0x490f, 0x263e,
        ],
        [0x0003, 0x295e, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ];

    let not_pack = decode_pack(&encode_pack(&[not_truth]).expect("not pack encodes"))
        .expect("not pack decodes");
    assert_eq!(execute_pack(&not_pack), Ok(vec![PyacValue::Boolean(false)]));

    let and_pack = decode_pack(&encode_pack(&and_truth_lie).expect("and pack encodes"))
        .expect("and pack decodes");
    assert_eq!(execute_pack(&and_pack), Ok(vec![PyacValue::Boolean(false)]));

    let or_pack = decode_pack(&encode_pack(&or_lie_truth).expect("or pack encodes"))
        .expect("or pack decodes");
    assert_eq!(execute_pack(&or_pack), Ok(vec![PyacValue::Boolean(true)]));
}

#[test]
fn literal_comparisons_execute_as_boolean_pack_results() {
    let equally = [
        0x0149, 0x881d, 5, 0x24de, 0x881d, 5, 0x313e, 0x4809, 0x295e, 0, 0, 0, 0, 0, 0, 0,
    ];
    let tiny = [
        0x0149, 0x881d, 4, 0x24de, 0x881d, 5, 0x313e, 0x6c0d, 0x295e, 0, 0, 0, 0, 0, 0, 0,
    ];
    let not_unequal = [
        0x2a91, 0x257e, 0x881d, 5, 0x24de, 0x881d, 4, 0x313e, 0x4809, 0x295e, 0x305e, 0x245e,
        0x30de, 0x295e, 0, 0,
    ];

    for (codelet, expected) in [(equally, true), (tiny, true), (not_unequal, true)] {
        let bytes = encode_pack(&[codelet]).expect("comparison pack encodes");
        let pack = decode_pack(&bytes).expect("comparison pack decodes");
        assert_eq!(execute_pack(&pack), Ok(vec![PyacValue::Boolean(expected)]));
    }
}

#[test]
fn combined_boolean_sentences_keep_codelet_boundaries() {
    let not_truth = [
        0x02a9, 0x257e, 0x6909, 0x245e, 0x7271, 0x217e, 0x305e, 0x245e, 0x30de, 0x295e, 0, 0, 0, 0,
        0, 0,
    ];
    let equally = [
        0x0149, 0x881d, 5, 0x24de, 0x881d, 5, 0x313e, 0x4809, 0x295e, 0, 0, 0, 0, 0, 0, 0,
    ];
    let not_unequal = [
        0x2a91, 0x257e, 0x881d, 5, 0x24de, 0x881d, 4, 0x313e, 0x4809, 0x295e, 0x305e, 0x245e,
        0x30de, 0x295e, 0, 0,
    ];
    let bytes = encode_pack(&[not_truth, equally, not_unequal]).expect("combined pack encodes");
    let pack = decode_pack(&bytes).expect("combined pack decodes");

    assert_eq!(
        execute_pack(&pack),
        Ok(vec![
            PyacValue::Boolean(false),
            PyacValue::Boolean(true),
            PyacValue::Boolean(true),
        ])
    );
}
