//! The backend's golden programs, checked against the interpreter.
//!
//! Every operator the interpreter proves is also compiled and executed here, so a
//! divergence between the two shows up as one failing operator rather than as a
//! silently wrong program.

use super::*;
use crate::WORDS_PER_CODELET;
use crate::semantic::{
    NumericArithmeticInstruction, NumericOperandCase, NumericOperator, decode_numeric_arithmetic,
    execute_numeric_arithmetic, execute_numeric_arithmetic_codelet, execute_numeric_invert_codelet,
};

const HEAD: u16 = 0x0149;
const NUMBER_QUOTE: u16 = 0x881d;
const ACCUSATIVE_CASE: u16 = 0x245e;
const SOURCE_CASE: u16 = 0x313e;
const DESTINATION_CASE: u16 = 0x243e;
const DATIVE_CASE: u16 = 0x207e;
const DEONTIC_MOOD: u16 = 0x295e;

fn arithmetic_codelet(operator: u16, ob: u16, to: u16) -> [u16; WORDS_PER_CODELET] {
    arithmetic_codelet_with_case(operator, ob, to, DESTINATION_CASE)
}

fn arithmetic_codelet_with_case(
    operator: u16,
    ob: u16,
    to: u16,
    operand_case: u16,
) -> [u16; WORDS_PER_CODELET] {
    [
        HEAD,
        NUMBER_QUOTE,
        ob,
        ACCUSATIVE_CASE,
        NUMBER_QUOTE,
        to,
        operand_case,
        operator,
        DEONTIC_MOOD,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ]
}

fn invert_codelet(ob: u16, bound: u16) -> [u16; WORDS_PER_CODELET] {
    [
        HEAD,
        NUMBER_QUOTE,
        ob,
        ACCUSATIVE_CASE,
        NUMBER_QUOTE,
        bound,
        DATIVE_CASE,
        0xaad2,
        DEONTIC_MOOD,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ]
}

/// The encodings the instruction set fixes, computed from the field layout
/// of each instruction format. They are the independent check on the
/// compiler's own encoder: the table comes from the specification, not from
/// the encoder that produces the words.
const GOLDEN_INSTRUCTIONS: [(&str, u32); 13] = [
    ("addi t0, zero, 1", 0x0010_0293),
    ("addi t1, zero, 1", 0x0010_0313),
    ("add t2, t0, t1", 0x0062_83b3),
    ("lui t4, 0x10", 0x0001_0eb7),
    ("addi t4, t4, -1", 0xfffe_8e93),
    ("sltu t5, t4, t2", 0x007e_bf33),
    ("sub t5, zero, t5", 0x41e0_0f33),
    ("xori t6, t5, -1", 0xffff_4f93),
    ("and t2, t2, t6", 0x01f3_f3b3),
    ("and t4, t4, t5", 0x01ee_feb3),
    ("or t2, t2, t4", 0x01d3_e3b3),
    ("addi a0, t2, 0", 0x0003_8513),
    ("ret", 0x0000_8067),
];

#[test]
fn encodes_the_golden_program_with_the_instruction_set_field_layout() {
    let codelet = arithmetic_codelet(0x8a64, 1, 1);
    let program = compile_numeric_arithmetic(&codelet).expect("the golden codelet compiles");
    let pairs: Vec<(&str, u32)> = program
        .instructions
        .iter()
        .map(|instruction| (instruction.text.as_str(), instruction.word))
        .collect();

    assert_eq!(pairs, GOLDEN_INSTRUCTIONS.to_vec());
}

#[test]
fn compiles_the_golden_numeric_plus_into_a_register_program() {
    let codelet = arithmetic_codelet(0x8a64, 1, 1);
    let program = compile_numeric_arithmetic(&codelet).expect("the golden codelet compiles");

    let listing = program
        .instructions
        .iter()
        .map(|instruction| instruction.text.as_str())
        .collect::<Vec<_>>()
        .join("; ");
    assert_eq!(
        listing,
        "addi t0, zero, 1; addi t1, zero, 1; add t2, t0, t1; lui t4, 0x10; \
             addi t4, t4, -1; sltu t5, t4, t2; sub t5, zero, t5; xori t6, t5, -1; \
             and t2, t2, t6; and t4, t4, t5; or t2, t2, t4; addi a0, t2, 0; ret"
    );
    assert_eq!(execute(&program), Ok(2));
}

#[test]
fn agrees_with_the_interpreter_for_every_proven_numeric_operator() {
    let cases = [
        (0x8a64, 1, 1),
        (0xabd2, 9, 4),
        (0xec90, 3, 4),
        (0x8893, 8, 2),
        (0xa810, 7, 3),
        (0x8c29, 2, 10),
        (0xe82a, 9, 4),
    ];
    for (operator, ob, to) in cases {
        let codelet = arithmetic_codelet(operator, ob, to);
        let expected =
            execute_numeric_arithmetic_codelet(&codelet).expect("the interpreter runs it");
        let compiled =
            compile_and_execute_numeric_arithmetic(&codelet).expect("the backend runs it");
        assert_eq!(compiled, expected, "operator 0x{operator:04x} disagrees");
    }
}

#[test]
fn saturates_like_the_interpreter_at_the_numeric_limit() {
    let saturated = arithmetic_codelet(0x8a64, u16::MAX, 1);
    assert_eq!(
        compile_and_execute_numeric_arithmetic(&saturated),
        Ok(u64::from(u16::MAX))
    );

    let floored = arithmetic_codelet(0xabd2, 2, 9);
    assert_eq!(compile_and_execute_numeric_arithmetic(&floored), Ok(0));

    let power = arithmetic_codelet(0x8c29, 2, 20);
    assert_eq!(
        compile_and_execute_numeric_arithmetic(&power),
        Ok(u64::from(u16::MAX))
    );

    let exact_power = arithmetic_codelet(0x8c29, 3, 3);
    assert_eq!(compile_and_execute_numeric_arithmetic(&exact_power), Ok(27));
}

#[test]
fn multiplies_without_the_saturation_the_other_operators_use() {
    let codelet = arithmetic_codelet(0xec90, u16::MAX, u16::MAX);
    assert_eq!(
        compile_and_execute_numeric_arithmetic(&codelet),
        Ok(4_294_836_225)
    );
}

#[test]
fn refuses_the_operators_the_interpreter_also_refuses() {
    let divide_by_zero = arithmetic_codelet(0x8893, 8, 0);
    assert_eq!(
        compile_numeric_arithmetic(&divide_by_zero).expect_err("division by zero"),
        BackendError::DivisionByZero
    );

    let remainder_by_zero = arithmetic_codelet(0xe82a, 9, 0);
    assert_eq!(
        compile_numeric_arithmetic(&remainder_by_zero).expect_err("remainder by zero"),
        BackendError::DivisionByZero
    );
}

#[test]
fn compiles_the_legacy_invert_shape() {
    let codelet = invert_codelet(5, 0);
    let program = compile_numeric_invert(&codelet).expect("the invert codelet compiles");
    let result = execute(&program).expect("the program runs");
    assert_eq!(result as i64, -5);
    assert_eq!(execute_numeric_invert_codelet(&codelet), Ok(-5));
}

#[test]
fn keeps_the_semantic_decoding_shared_with_the_interpreter() {
    let codelet = arithmetic_codelet(0x8a64, 7, 3);
    let instruction = decode_numeric_arithmetic(&codelet).expect("the codelet decodes");
    assert_eq!(
        instruction,
        NumericArithmeticInstruction {
            operator: NumericOperator::Plus,
            ob: 7,
            operand: 3,
            operand_case: NumericOperandCase::To,
        }
    );
    assert_eq!(execute_numeric_arithmetic(&instruction), Ok(10));
    assert_eq!(
        compile_and_execute_numeric_arithmetic(&codelet).expect("the backend runs it"),
        execute_numeric_arithmetic(&instruction).expect("the interpreter runs it")
    );
}

#[test]
fn semantic_decode_retains_the_from_case_hnuc() {
    let codelet = arithmetic_codelet_with_case(0xabd2, 9, 4, SOURCE_CASE);
    let instruction = decode_numeric_arithmetic(&codelet).expect("from-case codelet decodes");
    assert_eq!(instruction.operand_case, NumericOperandCase::From);
    assert_eq!(execute_numeric_arithmetic(&instruction), Ok(5));
    assert_eq!(compile_and_execute_numeric_arithmetic(&codelet), Ok(5));
}
