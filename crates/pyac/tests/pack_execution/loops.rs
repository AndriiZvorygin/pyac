//! Pack execution: bounded loops, the loop supervisor, and nested frames.

use pyac::{PyacValue, WORDS_PER_CODELET, decode_pack, encode_pack, execute_pack};

use super::fixtures::*;

const EMPTY_NESTED_LOOP_PROGRAMLET: [[u16; WORDS_PER_CODELET]; 10] = [
    [
        0x2911, 0x225e, 0xaa90, 0x8918, 0x24de, 0x327e, 0x225e, 0x4930, 0x243e, 0x881d, 0x0000,
        0x6357, 0xe8c9, 0x5217, 0x0000, 0x0000,
    ],
    [
        0x0a49, 0x225e, 0xaa90, 0x24de, 0x881d, 0, 0x245e, 0x225e, 0xc8c4, 0x414f, 0xea30, 0x217e,
        0, 0, 0, 0,
    ],
    [
        0x0289, 0x881d, 0x0001, 0x245e, 0x327e, 0x225e, 0xaa90, 0x243e, 0x8a64, 0x295e, 0x0000,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x0019, 0x225e, 0xaa90, 0x24de, 0xa828, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x0051, 0x225e, 0xaa90, 0x8918, 0x24de, 0xe8c9, 0x4e27, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x0511, 0x225e, 0xaa90, 0x892a, 0x24de, 0x327e, 0x225e, 0x4930, 0x243e, 0xe8c9, 0x5217,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x2491, 0x327e, 0x225e, 0x4930, 0x243e, 0x881d, 0x0002, 0x6357, 0x881d, 0x0002, 0x4e4f,
        0xaa90, 0x8918, 0x295e, 0x0000, 0x0000,
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

const LOOPING_NUMERIC_PROGRAMLET: [[u16; WORDS_PER_CODELET]; 5] = [
    [
        0x0511, 0x225e, 0xaa90, 0x8918, 0x24de, 0x327e, 0x225e, 0x4930, 0x243e, 0xe8c9, 0x5217,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x0289, 0x881d, 0x0001, 0x245e, 0x327e, 0x225e, 0x4930, 0x243e, 0x8a64, 0x295e, 0x0000,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x0019, 0x225e, 0x4930, 0x24de, 0xa828, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x0051, 0x225e, 0xaa90, 0x8918, 0x24de, 0xe8c9, 0x4e27, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x2491, 0x327e, 0x225e, 0x4930, 0x243e, 0x881d, 0x0001, 0x6357, 0x881d, 0x0004, 0x4e4f,
        0xaa90, 0x8918, 0x295e, 0x0000, 0x0000,
    ],
];

const LOOP_CURSOR_PROGRAMLET: [[u16; WORDS_PER_CODELET]; 5] = [
    [
        0x2911, 0x225e, 0xaa90, 0x8918, 0x24de, 0x327e, 0x225e, 0x4930, 0x243e, 0x881d, 0x0000,
        0x6357, 0xe8c9, 0x5217, 0x0000, 0x0000,
    ],
    [
        0x5209, 0x225e, 0xaa90, 0x24de, 0x4a57, 0x215e, 0x6357, 0x215e, 0x327e, 0x245e, 0x225e,
        0xc8c4, 0x414f, 0xea30, 0x217e, 0,
    ],
    [
        0x0019, 0x225e, 0xaa90, 0x24de, 0xa828, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x0051, 0x225e, 0xaa90, 0x8918, 0x24de, 0xe8c9, 0x4e27, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x2491, 0x327e, 0x225e, 0x4930, 0x243e, 0x881d, 0x0001, 0x6357, 0x881d, 0x0004, 0x4e4f,
        0xaa90, 0x8918, 0x295e, 0x0000, 0x0000,
    ],
];

const LOOP_EARLY_RETURN_PROGRAMLET: [[u16; WORDS_PER_CODELET]; 6] = [
    // gi hgut hyas na | do 4930 ma | Qyis 0 fromindex | Qyis 0 toindex
    // | ceremony def   (one codelet: 15 payload words)
    [
        0xa491, 0x225e, 0xaa90, 0x8918, 0x24de, 0x327e, 0x4930, 0x243e, 0x881d, 0x0000, 0x6357,
        0x881d, 0x0000, 0x4e4f, 0xe8c9, 0x5217,
    ],
    [
        0x5209, 0x225e, 0x4930, 0x24de, 0x4a57, 0x215e, 0x6357, 0x215e, 0x327e, 0x245e, 0x225e,
        0xc8c4, 0x414f, 0xea30, 0x217e, 0,
    ],
    [
        0xc149, 0x225e, 0x4930, 0x245e, 0x881d, 2, 0x313e, 0x4809, 0x29be, 0x4a57, 0x215e, 0x4e4f,
        0x215e, 0x327e, 0x6357, 0xa828,
    ],
    [
        0x0019, 0x225e, 0x4930, 0x24de, 0xa828, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x0051, 0x225e, 0xaa90, 0x8918, 0x24de, 0xe8c9, 0x4e27, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x2491, 0x327e, 0x225e, 0x4930, 0x243e, 0x881d, 0x0001, 0x6357, 0x881d, 0x0005, 0x4e4f,
        0xaa90, 0x8918, 0x295e, 0x0000, 0x0000,
    ],
];

const NESTED_BOUNDED_LOOP_PROGRAMLET: [[u16; WORDS_PER_CODELET]; 11] = [
    // gi hgut hyas na | do 4930 ma | Qyis 0 fromindex | Qyis 0 toindex
    // | ceremony def
    [
        0xa491, 0x225e, 0xaa90, 0x8918, 0x24de, 0x327e, 0x4930, 0x243e, 0x881d, 0x0000, 0x6357,
        0x881d, 0x0000, 0x4e4f, 0xe8c9, 0x5217,
    ],
    [
        0x0289, 0x881d, 0x0001, 0x245e, 0x327e, 0x225e, 0x4930, 0x243e, 0x8a64, 0x295e, 0x0000,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x0019, 0x225e, 0x4930, 0x24de, 0xa828, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x0051, 0x225e, 0xaa90, 0x8918, 0x24de, 0xe8c9, 0x4e27, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // the callee: gi hgut hyas hu na | do 4930 ma | Qyis 0 fromindex
    // | Qyis 0 toindex | ceremony def
    [
        0xa491, 0x225e, 0xaa90, 0x892a, 0x24de, 0x327e, 0x4930, 0x243e, 0x881d, 0x0000, 0x6357,
        0x881d, 0x0000, 0x4e4f, 0xe8c9, 0x5217,
    ],
    [
        0x0a49, 0x225e, 0xa86a, 0x24de, 0x881d, 0, 0x245e, 0x225e, 0xc8c4, 0x414f, 0xea30, 0x217e,
        0, 0, 0, 0,
    ],
    [
        0x2491, 0x327e, 0x225e, 0xa86a, 0x243e, 0x881d, 0x0000, 0x6357, 0x881d, 0x0003, 0x4e4f,
        0xaa90, 0x8918, 0x295e, 0x0000, 0x0000,
    ],
    [
        0x0289, 0x225e, 0xa86a, 0x245e, 0x327e, 0x225e, 0x4930, 0x243e, 0x8a64, 0x295e, 0x0000,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x0019, 0x225e, 0x4930, 0x24de, 0xa828, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x0051, 0x225e, 0xaa90, 0x892a, 0x24de, 0xe8c9, 0x4e27, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x2491, 0x327e, 0x225e, 0x4930, 0x243e, 0x881d, 0x0001, 0x6357, 0x881d, 0x0004, 0x4e4f,
        0xaa90, 0x892a, 0x295e, 0x0000, 0x0000,
    ],
];

#[test]
fn executes_bounded_fromindex_toindex_programlet_loops() {
    let result = run_numeric_programlet(
        &LOOPING_NUMERIC_PROGRAMLET,
        pyac_vliw_core::DEFAULT_PROGRAMLET_STEP_BUDGET,
    );
    assert_eq!(result.error, 0, "{result:?}");
    assert_eq!(result.values[0], 3);

    let exhausted = run_numeric_programlet(&LOOPING_NUMERIC_PROGRAMLET, 5);
    assert_eq!(exhausted.error, pyac_vliw_core::ERROR_STEP_BUDGET);

    for budget in 6..=8 {
        let exhausted = run_numeric_programlet(&LOOPING_NUMERIC_PROGRAMLET, budget);
        assert_eq!(exhausted.error, pyac_vliw_core::ERROR_STEP_BUDGET);
    }

    let exact_budget = run_numeric_programlet(&LOOPING_NUMERIC_PROGRAMLET, 9);
    assert_eq!(exact_budget.error, 0, "{exact_budget:?}");
    assert_eq!(exact_budget.values[0], 3);
}

#[test]
fn exposes_the_current_loop_index_and_skips_equal_bounds() {
    let result = run_numeric_programlet(
        &LOOP_CURSOR_PROGRAMLET,
        pyac_vliw_core::DEFAULT_PROGRAMLET_STEP_BUDGET,
    );
    assert_eq!(result.error, 0, "{result:?}");
    assert_eq!(result.values[0], 3, "fromindex 1 toindex 4 visits 1, 2, 3");

    // Equal bounds: the `fromindex` value is at slot 6 and the `toindex` value
    // at slot 9, so copying one to the other makes the loop visit nothing.
    let mut empty_range = LOOP_CURSOR_PROGRAMLET;
    empty_range[4][9] = empty_range[4][6];
    let result =
        run_numeric_programlet(&empty_range, pyac_vliw_core::DEFAULT_PROGRAMLET_STEP_BUDGET);
    assert_eq!(result.error, 0, "{result:?}");
    assert_eq!(result.values[0], 0, "equal loop bounds skip the body");
}

#[test]
fn explicit_fromindex_return_updates_loop_supervisor_state() {
    let bytes = encode_pack(&LOOP_EARLY_RETURN_PROGRAMLET).expect("programlet pack fits");
    let pack = decode_pack(&bytes).expect("programlet pack decodes");
    let result = run_numeric_programlet(
        &LOOP_EARLY_RETURN_PROGRAMLET,
        pyac_vliw_core::DEFAULT_PROGRAMLET_STEP_BUDGET,
    );

    assert_eq!(result.error, 0, "{result:?}");
    assert_eq!(result.values[0], 2, "cursor 2 returns the loop to bound 5");
    assert_eq!(execute_pack(&pack), Ok(vec![PyacValue::Unsigned(2)]));
}

#[test]
fn nested_empty_loop_skips_the_callee_body() {
    let result = run_numeric_programlet(
        &EMPTY_NESTED_LOOP_PROGRAMLET,
        pyac_vliw_core::DEFAULT_PROGRAMLET_STEP_BUDGET,
    );
    assert_eq!(result.error, 0, "{result:?}");
    assert_eq!(
        result.values[0], 0,
        "equal nested loop bounds skip the callee body"
    );
}

#[test]
fn nested_bounded_loops_keep_independent_frame_cursors() {
    let bytes = encode_pack(&NESTED_BOUNDED_LOOP_PROGRAMLET).expect("nested loop pack fits");
    let pack = decode_pack(&bytes).expect("nested loop pack decodes");
    let result = run_numeric_programlet(
        &NESTED_BOUNDED_LOOP_PROGRAMLET,
        pyac_vliw_core::DEFAULT_PROGRAMLET_STEP_BUDGET,
    );

    assert_eq!(result.error, 0, "{result:?}");
    assert_eq!(
        result.values[0], 9,
        "three inner iterations per outer iteration"
    );
    assert_eq!(execute_pack(&pack), Ok(vec![PyacValue::Unsigned(9)]));

    // The outer invocation is the last codelet; its `toindex` value is slot 9.
    // `fromindex 1 toindex 300` is 299 outer iterations, three inner each.
    let mut repeated_calls = NESTED_BOUNDED_LOOP_PROGRAMLET;
    repeated_calls[10][9] = 300;
    let repeated = run_numeric_programlet(
        &repeated_calls,
        pyac_vliw_core::DEFAULT_PROGRAMLET_STEP_BUDGET,
    );
    assert_eq!(repeated.error, 0, "{repeated:?}");
    assert_eq!(
        repeated.values[0], 897,
        "299 outer iterations times 3 inner iterations"
    );
}
