//! Pack execution: signed arithmetic across frames, and error indices.

use pyac::{PyacValue, SemanticError, WORDS_PER_CODELET, decode_pack, encode_pack, execute_pack};

use super::fixtures::*;

const INVERT: [u16; WORDS_PER_CODELET] = [
    0x0149, 0x881d, 5, 0x245e, 0x881d, 0, 0x207e, 0xaad2, 0x295e, 0, 0, 0, 0, 0, 0, 0,
];

const MULTIPLY_LOCAL_PROGRAMLET: [[u16; WORDS_PER_CODELET]; 6] = [
    [
        0x0511, 0x225e, 0xaa90, 0x8918, 0x24de, 0x327e, 0x225e, 0x4930, 0x243e, 0xe8c9, 0x5217,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x1449, 0x225e, 0xaa90, 0x24de, 0x881d, 6, 0x245e, 0x225e, 0x0b50, 0xea30, 0x414f, 0xea30,
        0x217e, 0, 0, 0,
    ],
    [
        0x1489, 0x225e, 0xaa90, 0x313e, 0x327e, 0x225e, 0x4930, 0x243e, 0x881d, 0x0007, 0x287e,
        0xec90, 0x295e, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x0019, 0x225e, 0x4930, 0x24de, 0xa828, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x0051, 0x225e, 0xaa90, 0x8918, 0x24de, 0xe8c9, 0x4e27, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x0091, 0x327e, 0x225e, 0x4930, 0x243e, 0xaa90, 0x8918, 0x295e, 0x0000, 0x0000, 0x0000,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
];

const NESTED_SIGNED_PROGRAMLET: [[u16; WORDS_PER_CODELET]; 10] = [
    SIGNED_INVERT_PROGRAMLET[0],
    SIGNED_INVERT_PROGRAMLET[1],
    SIGNED_INVERT_PROGRAMLET[2],
    SIGNED_INVERT_PROGRAMLET[3],
    SIGNED_INVERT_PROGRAMLET[4],
    [
        0x0511, 0x225e, 0xaa90, 0x892a, 0x24de, 0x327e, 0x225e, 0x4930, 0x243e, 0xe8c9, 0x5217,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x0091, 0x327e, 0x225e, 0x4930, 0x243e, 0xaa90, 0x8918, 0x295e, 0x0000, 0x0000, 0x0000,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x0019, 0x225e, 0x4930, 0x24de, 0xa828, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x0051, 0x225e, 0xaa90, 0x892a, 0x24de, 0xe8c9, 0x4e27, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x0091, 0x327e, 0x225e, 0x4930, 0x243e, 0xaa90, 0x892a, 0x295e, 0x0000, 0x0000, 0x0000,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
];

const NUMERIC_OPERATOR_PROGRAMLET: [[u16; WORDS_PER_CODELET]; 13] = [
    [
        0x0511, 0x225e, 0xaa90, 0x8918, 0x24de, 0x327e, 0x225e, 0x4930, 0x243e, 0xe8c9, 0x5217,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x1449, 0x225e, 0xaa90, 0x24de, 0x881d, 6, 0x245e, 0x225e, 0x0b50, 0xea30, 0x414f, 0xea30,
        0x217e, 0, 0, 0,
    ],
    [
        0x0a49, 0x225e, 0x4930, 0x24de, 0x881d, 0, 0x245e, 0x225e, 0xc8c4, 0x414f, 0xea30, 0x217e,
        0, 0, 0, 0,
    ],
    [
        0x1489, 0x225e, 0xaa90, 0x313e, 0x327e, 0x225e, 0x4930, 0x243e, 0x881d, 0x0007, 0x287e,
        0xec90, 0x295e, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x1489, 0x225e, 0x4930, 0x245e, 0x327e, 0x225e, 0x4930, 0x243e, 0x881d, 0x0002, 0x287e,
        0x8893, 0x295e, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x1449, 0x225e, 0x4930, 0x245e, 0x881d, 0x0002, 0x313e, 0x327e, 0x225e, 0x4930, 0x243e,
        0x8c29, 0x295e, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x1449, 0x225e, 0x4930, 0x245e, 0x881d, 0x0005, 0x313e, 0x327e, 0x225e, 0x4930, 0x243e,
        0xe82a, 0x295e, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x1489, 0x225e, 0x4930, 0x245e, 0x327e, 0x225e, 0x4930, 0x243e, 0x881d, 0x0007, 0x490f,
        0xa810, 0x295e, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x0289, 0x881d, 0x0002, 0x245e, 0x327e, 0x225e, 0x4930, 0x243e, 0xabd2, 0x295e, 0x0000,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x0289, 0x881d, 0x0003, 0x245e, 0x327e, 0x225e, 0x4930, 0x243e, 0x8a64, 0x295e, 0x0000,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x0019, 0x225e, 0x4930, 0x24de, 0xa828, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x0051, 0x225e, 0xaa90, 0x8918, 0x24de, 0xe8c9, 0x4e27, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x0091, 0x327e, 0x225e, 0x4930, 0x243e, 0xaa90, 0x8918, 0x295e, 0x0000, 0x0000, 0x0000,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
];

const SIGNED_COMPARISON_PROGRAMLET: [[u16; WORDS_PER_CODELET]; 9] = [
    SIGNED_INVERT_PROGRAMLET[0],
    SIGNED_INVERT_PROGRAMLET[1],
    SIGNED_INVERT_PROGRAMLET[2],
    // gi hgut ka | Qyis 0 so | tiny cu | Qyis 1 ka | do gi 4930 ma | plus tu
    [
        0x76b6, 0x225e, 0xaa90, 0x245e, 0x881d, 0x0000, 0x313e, 0x6c0d, 0x29be, 0x881d, 0x0001,
        0x245e, 0x327e, 0x225e, 0x4930, 0x243e,
    ],
    [
        0x0005, 0x8a64, 0x295e, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x0289, 0x881d, 0x0002, 0x245e, 0x327e, 0x225e, 0x4930, 0x243e, 0x8a64, 0x295e, 0x0000,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x0019, 0x225e, 0x4930, 0x24de, 0xa828, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    SIGNED_INVERT_PROGRAMLET[4],
    SIGNED_INVERT_PROGRAMLET[5],
];

const SIGNED_INVERT_PROGRAMLET: [[u16; WORDS_PER_CODELET]; 6] = [
    [
        0x0511, 0x225e, 0xaa90, 0x8918, 0x24de, 0x327e, 0x225e, 0x4930, 0x243e, 0xe8c9, 0x5217,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x0a49, 0x225e, 0xaa90, 0x24de, 0x881d, 5, 0x245e, 0x225e, 0xc8c4, 0x414f, 0xea30, 0x217e,
        0, 0, 0, 0,
    ],
    [
        0x0289, 0x225e, 0xaa90, 0x245e, 0x327e, 0x225e, 0xaa90, 0x243e, 0xaad2, 0x295e, 0x0000,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x0019, 0x225e, 0xaa90, 0x24de, 0xa828, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x0051, 0x225e, 0xaa90, 0x8918, 0x24de, 0xe8c9, 0x4e27, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x0091, 0x327e, 0x225e, 0x4930, 0x243e, 0xaa90, 0x8918, 0x295e, 0x0000, 0x0000, 0x0000,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
];

#[test]
fn executes_multiply_using_named_local_operands() {
    let result = run_numeric_programlet(
        &MULTIPLY_LOCAL_PROGRAMLET,
        pyac_vliw_core::DEFAULT_PROGRAMLET_STEP_BUDGET,
    );
    assert_eq!(result.error, 0, "{result:?}");
    assert_eq!(result.values[0], 42);
}

#[test]
fn preserves_signed_numeric_values_through_programlet_calls_and_returns() {
    let result = run_numeric_programlet(
        &SIGNED_INVERT_PROGRAMLET,
        pyac_vliw_core::DEFAULT_PROGRAMLET_STEP_BUDGET,
    );
    assert_eq!(result.error, 0, "{result:?}");
    assert_eq!(result.value_kind, pyac_vliw_core::VALUE_SIGNED);
    assert_eq!(result.values[0] as i32, -5);

    let bytes = encode_pack(&SIGNED_INVERT_PROGRAMLET).expect("signed programlet pack fits");
    assert_eq!(
        execute_pack(&decode_pack(&bytes).expect("signed pack decodes")),
        Ok(vec![PyacValue::Signed(-5)])
    );
}

#[test]
fn executes_chained_programlet_arithmetic_operators_and_preserves_intermediates() {
    let result = run_numeric_programlet(
        &NUMERIC_OPERATOR_PROGRAMLET,
        pyac_vliw_core::DEFAULT_PROGRAMLET_STEP_BUDGET,
    );
    assert_eq!(result.error, 0, "{result:?}");
    assert_eq!(result.values[0], 7);

    let bytes = encode_pack(&NUMERIC_OPERATOR_PROGRAMLET).expect("operator programlet pack fits");
    assert_eq!(
        execute_pack(&decode_pack(&bytes).expect("operator pack decodes")),
        Ok(vec![PyacValue::Unsigned(7)])
    );
}

#[test]
fn propagates_signed_results_through_nested_programlet_frames() {
    let result = run_numeric_programlet(
        &NESTED_SIGNED_PROGRAMLET,
        pyac_vliw_core::DEFAULT_PROGRAMLET_STEP_BUDGET,
    );
    assert_eq!(result.error, 0, "{result:?}");
    assert_eq!(result.value_kind, pyac_vliw_core::VALUE_SIGNED);
    assert_eq!(result.values[0] as i32, -5);

    let bytes = encode_pack(&NESTED_SIGNED_PROGRAMLET).expect("nested signed pack fits");
    assert_eq!(
        execute_pack(&decode_pack(&bytes).expect("nested signed pack decodes")),
        Ok(vec![PyacValue::Signed(-5)])
    );
}

#[test]
fn compares_signed_local_values_in_programlet_branches() {
    let result = run_numeric_programlet(
        &SIGNED_COMPARISON_PROGRAMLET,
        pyac_vliw_core::DEFAULT_PROGRAMLET_STEP_BUDGET,
    );
    assert_eq!(result.error, 0, "{result:?}");
    assert_eq!(
        result.values[0], 3,
        "the following sentence is sequential, not an else arm"
    );
}

#[test]
fn mixed_pack_failure_reports_the_original_codelet_index() {
    let bytes = encode_pack(&[PLUS, [0; WORDS_PER_CODELET]]).expect("pack fits");
    let pack = decode_pack(&bytes).expect("pack decodes");

    let error = execute_pack(&pack).expect_err("unknown codelet is unsupported");
    assert_eq!(error.index, 1);
    assert_eq!(error.source, SemanticError::UnsupportedCodelet);
    assert_eq!(error.code(), "unsupported-codelet");
}

#[test]
fn mixed_pack_preserves_semantic_errors_with_their_codelet_index() {
    let divide_by_zero = [
        0x0149, 0x881d, 8, 0x245e, 0x881d, 0, 0x243e, 0x8893, 0x295e, 0, 0, 0, 0, 0, 0, 0,
    ];
    let bytes = encode_pack(&[PLUS, divide_by_zero]).expect("pack fits");
    let pack = decode_pack(&bytes).expect("pack decodes");

    let error = execute_pack(&pack).expect_err("division by zero fails");
    assert_eq!(error.index, 1);
    assert_eq!(error.source, SemanticError::DivisionByZero);
}
