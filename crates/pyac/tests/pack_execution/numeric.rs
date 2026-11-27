//! Pack execution: literal and arithmetic codelets through the shared pack runtime.

use pyac::{PyacValue, VectorElements, WORDS_PER_CODELET, decode_pack, encode_pack, execute_pack};

use super::fixtures::*;

const DATE: [u16; WORDS_PER_CODELET] = [
    0x00c1, 0x981d, 0x9c00, 0x8924, 0x0196, 0, 0x245e, 0x217e, 0, 0, 0, 0, 0, 0, 0, 0,
];

const FLOAT_LITERAL: [u16; WORDS_PER_CODELET] = [
    0x0031, 0xd01d, 0x0000, 0x3fa0, 0x245e, 0x217e, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

const INVERT: [u16; WORDS_PER_CODELET] = [
    0x0149, 0x881d, 5, 0x245e, 0x881d, 0, 0x207e, 0xaad2, 0x295e, 0, 0, 0, 0, 0, 0, 0,
];

const LOCAL_NUMBER_PROGRAMLET: [[u16; WORDS_PER_CODELET]; 6] = [
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
        0x0091, 0x327e, 0x225e, 0x4930, 0x243e, 0xaa90, 0x8918, 0x295e, 0x0000, 0x0000, 0x0000,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
];

const TRUTH: [u16; WORDS_PER_CODELET] = [
    0x0015, 0x6909, 0x245e, 0xea30, 0x217e, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

const VECTOR: [u16; WORDS_PER_CODELET] = [
    0x0061, 0x8f1d, 1, 2, 3, 0x245e, 0x217e, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

#[test]
fn executes_mixed_supported_codelet_families_in_pack_order() {
    let bytes = encode_pack(&[PLUS, INVERT, DATE, VECTOR, TRUTH]).expect("pack fits");
    let pack = decode_pack(&bytes).expect("pack decodes");

    assert_eq!(
        execute_pack(&pack),
        Ok(vec![
            PyacValue::Unsigned(2),
            PyacValue::Signed(-5),
            PyacValue::DateEpochMs(1_746_057_600_000),
            PyacValue::Vector(VectorElements::Numbers(vec![1, 2, 3])),
            PyacValue::Boolean(true),
        ])
    );
}

#[test]
fn executes_f32_quotes_and_arithmetic_through_the_shared_pack_runtime() {
    let cases = [
        (0.1, 0.2, 0x8a64, 0.1f32 + 0.2f32),
        (5.5, 2.0, 0xabd2, 5.5f32 - 2.0f32),
        (1.5, 2.0, 0xec90, 1.5f32 * 2.0f32),
        (1.5, 4.0, 0x8893, 1.5f32 / 4.0f32),
    ];
    let codelets = cases
        .iter()
        .map(|(left, right, operator, _)| float_arithmetic(*left, *right, *operator))
        .collect::<Vec<_>>();
    let pack =
        decode_pack(&encode_pack(&codelets).expect("f32 pack fits")).expect("f32 pack decodes");

    assert_eq!(
        execute_pack(&pack),
        Ok(cases
            .iter()
            .map(|(_, _, _, result)| PyacValue::Float32(result.to_bits()))
            .collect())
    );

    let literal_pack = decode_pack(&encode_pack(&[FLOAT_LITERAL]).expect("literal pack fits"))
        .expect("literal pack decodes");
    assert_eq!(
        execute_pack(&literal_pack),
        Ok(vec![PyacValue::Float32(1.25f32.to_bits())])
    );
}

#[test]
fn executes_float32_and_float64_vectors_through_the_shared_pack_runtime() {
    let f32_values = [1.25f32.to_bits(), (-0.5f32).to_bits()];
    let f32_vector = [
        0x00c1,
        0xd11d,
        f32_values[0] as u16,
        (f32_values[0] >> 16) as u16,
        f32_values[1] as u16,
        (f32_values[1] >> 16) as u16,
        0x245e,
        0x217e,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    let f64_values = [std::f64::consts::PI.to_bits(), (-0.0f64).to_bits()];
    let f64_vector = [
        0x0c01,
        0xd91d,
        f64_values[0] as u16,
        (f64_values[0] >> 16) as u16,
        (f64_values[0] >> 32) as u16,
        (f64_values[0] >> 48) as u16,
        f64_values[1] as u16,
        (f64_values[1] >> 16) as u16,
        (f64_values[1] >> 32) as u16,
        (f64_values[1] >> 48) as u16,
        0x245e,
        0x217e,
        0,
        0,
        0,
        0,
    ];
    let pack =
        decode_pack(&encode_pack(&[f32_vector, f64_vector]).expect("float vector pack fits"))
            .expect("float vector pack decodes");

    assert_eq!(
        execute_pack(&pack),
        Ok(vec![
            PyacValue::Vector(VectorElements::Float32(f32_values.to_vec())),
            PyacValue::Vector(VectorElements::Float64(f64_values.to_vec())),
        ])
    );
}

#[test]
fn executes_fixed16_quotes_and_arithmetic_through_the_shared_pack_runtime() {
    let fixed = |left, right, operator| {
        [
            0x0149, 0xc81d, left, 0x245e, 0xc81d, right, 0x243e, operator, 0x295e, 0, 0, 0, 0, 0,
            0, 0,
        ]
    };
    let codelets = [
        fixed(23_328, 11_664, 0x8a64),
        fixed(23_328, 11_664, 0xabd2),
        fixed(23_328, 23_328, 0xec90),
        fixed(11_664, 23_328, 0x8893),
    ];
    let pack = decode_pack(&encode_pack(&codelets).expect("fixed16 pack fits"))
        .expect("fixed16 pack decodes");
    assert_eq!(
        execute_pack(&pack),
        Ok(vec![
            PyacValue::Fixed16(34_992),
            PyacValue::Fixed16(11_664),
            PyacValue::Fixed16(11_664),
            PyacValue::Fixed16(23_328),
        ])
    );

    let literal = [
        0x0019, 0xc81d, 5_832, 0x245e, 0x217e, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let pack = decode_pack(&encode_pack(&[literal]).expect("literal pack fits"))
        .expect("literal pack decodes");
    assert_eq!(execute_pack(&pack), Ok(vec![PyacValue::Fixed16(5_832)]));
}

#[test]
fn shared_pack_runtime_reports_fixed16_range_errors() {
    let arithmetic = |left, right, operator| {
        [
            0x0149, 0xc81d, left, 0x245e, 0xc81d, right, 0x243e, operator, 0x295e, 0, 0, 0, 0, 0,
            0, 0,
        ]
    };
    let overflow = decode_pack(&encode_pack(&[arithmetic(u16::MAX, 1, 0x8a64)]).unwrap()).unwrap();
    assert_eq!(
        execute_pack(&overflow).unwrap_err().code(),
        "fixed16-domain"
    );
    let underflow = decode_pack(&encode_pack(&[arithmetic(0, 1, 0xabd2)]).unwrap()).unwrap();
    assert_eq!(
        execute_pack(&underflow).unwrap_err().code(),
        "fixed16-domain"
    );
}

#[test]
fn shared_pack_runtime_rejects_non_finite_f32_values_and_results() {
    let infinity = [
        0x0031, 0xd01d, 0, 0x7f80, 0x245e, 0x217e, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let pack = decode_pack(&encode_pack(&[infinity]).expect("pack fits")).expect("pack decodes");
    let error = execute_pack(&pack).expect_err("infinity is rejected");
    assert_eq!(error.code(), "non-finite-float");

    let overflow = float_arithmetic(f32::MAX, 2.0, 0xec90);
    let pack = decode_pack(&encode_pack(&[overflow]).expect("pack fits")).expect("pack decodes");
    let error = execute_pack(&pack).expect_err("overflow is rejected");
    assert_eq!(error.code(), "non-finite-float");
}

#[test]
fn executes_canonical_numeric_local_declaration_write_and_return() {
    let bytes = encode_pack(&LOCAL_NUMBER_PROGRAMLET).expect("programlet pack fits");
    let pack = decode_pack(&bytes).expect("programlet pack decodes");
    let result = run_numeric_programlet(
        &LOCAL_NUMBER_PROGRAMLET,
        pyac_vliw_core::DEFAULT_PROGRAMLET_STEP_BUDGET,
    );
    assert_eq!(result.error, 0, "{result:?}");
    assert_eq!(result.values[0], 1);
    let repeated = run_numeric_programlet(
        &LOCAL_NUMBER_PROGRAMLET,
        pyac_vliw_core::DEFAULT_PROGRAMLET_STEP_BUDGET,
    );
    assert_eq!(repeated.error, 0, "{repeated:?}");
    assert_eq!(
        repeated.values[0], 1,
        "each invocation starts with a fresh local frame"
    );

    assert_eq!(execute_pack(&pack), Ok(vec![PyacValue::Unsigned(1)]));
}
