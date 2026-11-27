//! Numeric and conditional semantics: what a codelet computes.
//!
//! A decoder reads a codelet's words into an instruction; an executor runs that
//! instruction. Keeping decode separate from execute is what would let another
//! destination share the decoding and differ only in how the arithmetic is
//! performed — that is why the two are distinct steps even though the CPU
//! interpreter is currently the only destination.
//!
//! - `words`        — the HNUC words the decoders compare against
//! - `types`        — the decoded instruction shapes
//! - `errors`       — `SemanticError`
//! - `arithmetic`   — scalar decode and execute
//! - `vectors`      — elementwise vector arithmetic
//! - `conditionals` — comparison and arm selection

#[path = "semantic/arithmetic.rs"]
mod arithmetic;
#[path = "semantic/conditionals.rs"]
mod conditionals;
#[path = "semantic/errors.rs"]
mod errors;
#[path = "semantic/types.rs"]
mod types;
#[path = "semantic/vectors.rs"]
mod vectors;
#[path = "semantic/words.rs"]
mod words;

pub use arithmetic::{
    decode_numeric_arithmetic, decode_numeric_invert, decode_numeric_plus,
    execute_numeric_arithmetic, execute_numeric_arithmetic_codelet, execute_numeric_codelet,
    execute_numeric_invert, execute_numeric_invert_codelet, execute_numeric_plus,
    execute_numeric_plus_codelet,
};
pub use conditionals::{
    decode_conditional_number, execute_conditional_number, execute_conditional_number_codelets,
};
pub use errors::SemanticError;
pub use types::{
    ConditionalNumberInstruction, NumericArithmeticInstruction, NumericCodeletValue,
    NumericInvertInstruction, NumericOperandCase, NumericOperator, NumericPlusInstruction,
    NumericVectorArithmeticInstruction, NumericVectorOperand,
};
pub use vectors::{decode_numeric_vector_arithmetic, execute_numeric_vector_arithmetic};
pub use words::GOLDEN_NUMERIC_PLUS_CODELET;

#[cfg(test)]
#[path = "semantic/tests.rs"]
mod tests;
