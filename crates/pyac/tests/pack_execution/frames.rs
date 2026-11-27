//! Pack execution: ceremony calls, local frames, and case-argument passing.

use pyac::{
    PyacValue, VectorElements, WORDS_PER_CODELET, decode_pack, encode_pack, execute_pack,
    execute_pack_sections,
};

use super::fixtures::*;

const FORWARDED_CASE_ARGUMENT_PROGRAMLET: [[u16; WORDS_PER_CODELET]; 11] = [
    [
        0x2891, 0x225e, 0x8a64, 0xaa0a, 0x24de, 0x881d, 0x0000, 0x245e, 0x327e, 0x225e, 0x4930,
        0x243e, 0xe8c9, 0x5217, 0x0000, 0x0000,
    ],
    [
        0x5209, 0x225e, 0xaa90, 0x24de, 0x4a57, 0x215e, 0x245e, 0x215e, 0x327e, 0x245e, 0x225e,
        0xc8c4, 0x414f, 0xea30, 0x217e, 0,
    ],
    [
        0x0289, 0x881d, 0x0002, 0x245e, 0x327e, 0x225e, 0xaa90, 0x243e, 0x8a64, 0x295e, 0x0000,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x0019, 0x225e, 0xaa90, 0x245e, 0xa828, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x0051, 0x225e, 0x8a64, 0xaa0a, 0x24de, 0xe8c9, 0x4e27, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x2891, 0x225e, 0xaa90, 0x892a, 0x24de, 0x881d, 0x0000, 0x245e, 0x327e, 0x225e, 0x4930,
        0x243e, 0xe8c9, 0x5217, 0x0000, 0x0000,
    ],
    [
        0x5209, 0x225e, 0xaa90, 0x24de, 0x4a57, 0x215e, 0x245e, 0x215e, 0x327e, 0x245e, 0x225e,
        0xc8c4, 0x414f, 0xea30, 0x217e, 0,
    ],
    [
        0x0489, 0x225e, 0xaa90, 0x245e, 0x327e, 0x225e, 0xaa90, 0x243e, 0x8a64, 0xaa0a, 0x295e,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x0019, 0x225e, 0xaa90, 0x245e, 0xa828, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x0051, 0x225e, 0xaa90, 0x892a, 0x24de, 0xe8c9, 0x4e27, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x0489, 0x881d, 0x0005, 0x245e, 0x327e, 0x225e, 0x4930, 0x243e, 0xaa90, 0x892a, 0x295e,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
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

const NESTED_NUMERIC_PROGRAMLET: [[u16; WORDS_PER_CODELET]; 11] = [
    [
        0x0511, 0x225e, 0xaa90, 0x8918, 0x24de, 0x327e, 0x225e, 0x4930, 0x243e, 0xe8c9, 0x5217,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
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
        0x0091, 0x327e, 0x225e, 0x0920, 0x243e, 0xaa90, 0x8918, 0x295e, 0x0000, 0x0000, 0x0000,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x0091, 0x327e, 0x225e, 0xa86a, 0x243e, 0xaa90, 0x8918, 0x295e, 0x0000, 0x0000, 0x0000,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x0019, 0x225e, 0xa86a, 0x24de, 0xa828, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x0051, 0x225e, 0xaa90, 0x892a, 0x24de, 0xe8c9, 0x4e27, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x0091, 0x327e, 0x225e, 0x4930, 0x243e, 0xaa90, 0x892a, 0x295e, 0x0000, 0x0000, 0x0000,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
];

const PARAMETERIZED_NESTED_PROGRAMLET: [[u16; WORDS_PER_CODELET]; 6] = [
    [
        0x2891, 0x225e, 0x8a64, 0xaa0a, 0x24de, 0x881d, 0x0000, 0x245e, 0x327e, 0x225e, 0x4930,
        0x243e, 0xe8c9, 0x5217, 0x0000, 0x0000,
    ],
    [
        0x5209, 0x225e, 0xaa90, 0x24de, 0x4a57, 0x215e, 0x245e, 0x215e, 0x327e, 0x245e, 0x225e,
        0xc8c4, 0x414f, 0xea30, 0x217e, 0,
    ],
    [
        0x0289, 0x881d, 0x0002, 0x245e, 0x327e, 0x225e, 0xaa90, 0x243e, 0x8a64, 0x295e, 0x0000,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
    [
        0x0019, 0x225e, 0xaa90, 0x245e, 0xa828, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x0051, 0x225e, 0x8a64, 0xaa0a, 0x24de, 0xe8c9, 0x4e27, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0x0489, 0x881d, 0x0005, 0x245e, 0x327e, 0x225e, 0x4930, 0x243e, 0x8a64, 0xaa0a, 0x295e,
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
];

#[test]
fn executes_nested_ceremony_calls_with_isolated_local_frames() {
    let bytes = encode_pack(&NESTED_NUMERIC_PROGRAMLET).expect("nested programlet pack fits");
    let pack = decode_pack(&bytes).expect("nested programlet pack decodes");
    let result = run_numeric_programlet(
        &NESTED_NUMERIC_PROGRAMLET,
        pyac_vliw_core::DEFAULT_PROGRAMLET_STEP_BUDGET,
    );
    assert_eq!(result.error, 0, "{result:?}");

    assert_eq!(execute_pack(&pack), Ok(vec![PyacValue::Unsigned(1)]));
}

#[test]
fn passes_typed_case_arguments_through_nested_programlet_frames() {
    let bytes = encode_pack(&PARAMETERIZED_NESTED_PROGRAMLET).expect("parameterized pack fits");
    let pack = decode_pack(&bytes).expect("parameterized pack decodes");
    let result = run_numeric_programlet(
        &PARAMETERIZED_NESTED_PROGRAMLET,
        pyac_vliw_core::DEFAULT_PROGRAMLET_STEP_BUDGET,
    );
    assert_eq!(result.error, 0, "{result:?}");
    assert_eq!(result.values[0], 7);
    assert_eq!(execute_pack(&pack), Ok(vec![PyacValue::Unsigned(7)]));
}

#[test]
fn forwards_a_case_argument_from_one_programlet_to_a_nested_call() {
    let bytes = encode_pack(&FORWARDED_CASE_ARGUMENT_PROGRAMLET).expect("forwarding pack fits");
    let pack = decode_pack(&bytes).expect("forwarding pack decodes");
    let result = run_numeric_programlet(
        &FORWARDED_CASE_ARGUMENT_PROGRAMLET,
        pyac_vliw_core::DEFAULT_PROGRAMLET_STEP_BUDGET,
    );
    assert_eq!(result.error, 0, "{result:?}");
    assert_eq!(result.values[0], 7);
    assert_eq!(execute_pack(&pack), Ok(vec![PyacValue::Unsigned(7)]));

    let mut local_genitive_argument = FORWARDED_CASE_ARGUMENT_PROGRAMLET;
    local_genitive_argument[7] = [
        0x0911, 0xaa90, 0x215e, 0x327e, 0x245e, 0x327e, 0x225e, 0xaa90, 0x243e, 0x8a64, 0xaa0a,
        0x295e, 0x0000, 0x0000, 0x0000, 0x0000,
    ];
    let result = run_numeric_programlet(
        &local_genitive_argument,
        pyac_vliw_core::DEFAULT_PROGRAMLET_STEP_BUDGET,
    );
    assert_eq!(result.error, 0, "{result:?}");
    assert_eq!(result.values[0], 7);
}

#[test]
fn canonical_programlet_rejects_constant_writes() {
    let mut constant_write = LOCAL_NUMBER_PROGRAMLET;
    constant_write[1] = [
        0x1449, 0x225e, 0xaa90, 0x24de, 0x881d, 0, 0x245e, 0x225e, 0x0b50, 0xea30, 0x414f, 0xea30,
        0x217e, 0, 0, 0,
    ];
    let result = run_numeric_programlet(
        &constant_write,
        pyac_vliw_core::DEFAULT_PROGRAMLET_STEP_BUDGET,
    );
    assert_eq!(result.error, pyac_vliw_core::ERROR_CONSTANT_WRITE);
}

#[test]
fn executes_independent_named_programlet_sections_and_preserves_hnuc_labels() {
    fn section_marker(name: u16, mood: u16) -> [u16; WORDS_PER_CODELET] {
        let mut codelet = [0; WORDS_PER_CODELET];
        codelet[0] = 0x0049;
        codelet[1..7].copy_from_slice(&[0x225e, name, 0x24de, 0xec01, 0x6a84, mood]);
        codelet
    }

    let mut second_programlet = LOCAL_NUMBER_PROGRAMLET;
    second_programlet[2][2] = 2;

    let mut codelets = Vec::new();
    for (name, programlet) in [
        (0xaa90, &LOCAL_NUMBER_PROGRAMLET),
        (0xa86a, &second_programlet),
    ] {
        codelets.push(section_marker(name, 0x5217));
        codelets.extend(*programlet);
        codelets.push(section_marker(name, 0x4e27));
    }
    let bytes = encode_pack(&codelets).expect("sectioned pack encodes");
    let pack = decode_pack(&bytes).expect("sectioned pack decodes");

    let sections = execute_pack_sections(&pack).expect("CPU shared runtime executes sections");
    assert_eq!(sections.len(), 2);
    let sections_by_index = sections
        .iter()
        .map(|section| (section.section_index, section))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        sections_by_index.keys().copied().collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(sections_by_index.get(&0).unwrap().name_words, vec![0xaa90]);
    assert_eq!(sections_by_index.get(&1).unwrap().name_words, vec![0xa86a]);
    assert_eq!(
        sections_by_index.get(&0).unwrap().value,
        PyacValue::Unsigned(1)
    );
    assert_eq!(
        sections_by_index.get(&1).unwrap().value,
        PyacValue::Unsigned(2)
    );
    assert_eq!(
        execute_pack(&pack),
        Ok(vec![PyacValue::Unsigned(1), PyacValue::Unsigned(2)])
    );
}

#[test]
fn pluk_tlas_counts_exact_matches_from_named_float32_vectors_in_a_pack() {
    let prediction = [
        0x0c05, 0x68b1, 0x24de, 0xd71d, 0, 0x3f80, 0, 0x4000, 0, 0x4040, 0x245e, 0x217e, 0, 0, 0, 0,
    ];
    let answer = [
        0x0c05, 0x4930, 0x24de, 0xd71d, 0, 0x3f80, 0, 0x4000, 0, 0x4080, 0x245e, 0x217e, 0, 0, 0, 0,
    ];
    let score = [
        0x2489, 0x225e, 0x68b1, 0x245e, 0x327e, 0x225e, 0xaa90, 0x243e, 0x225e, 0x4930, 0x287e,
        0x2a64, 0x896a, 0x295e, 0x0000, 0x0000,
    ];
    let bytes = encode_pack(&[prediction, answer, score]).expect("evaluator pack encodes");
    let pack = decode_pack(&bytes).expect("evaluator pack decodes");

    assert_eq!(
        execute_pack(&pack),
        Ok(vec![
            PyacValue::Vector(VectorElements::Float32(vec![
                1.0f32.to_bits(),
                2.0f32.to_bits(),
                3.0f32.to_bits(),
            ])),
            PyacValue::Vector(VectorElements::Float32(vec![
                1.0f32.to_bits(),
                2.0f32.to_bits(),
                4.0f32.to_bits(),
            ])),
            PyacValue::Unsigned(2),
        ])
    );
}
