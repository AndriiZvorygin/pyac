//! The RISC-V destination for the Pyash VLIW codelet format.
//!
//! Compiling a codelet for RISC-V means emitting a short register program:
//! the codelet's quoted operands become immediates, its operator word becomes
//! the instruction that computes the result, and the result leaves in `a0`.
//!
//! The compiled program must agree with the interpreter, so the numeric
//! semantics are the interpreter's semantics, including the saturation to
//! `0xffff` that the JavaScript reference backend applies to addition,
//! subtraction, and exponentiation.
//!
//! The backend emits no branches. That is a deliberate limit, not an
//! oversight: a branchless program needs no label resolution, so the emitted
//! words are the whole program, and the executor below can check them
//! without a control-flow model.

use crate::WORDS_PER_CODELET;
use crate::semantic::{NumericOperator, decode_numeric_arithmetic, decode_numeric_invert};

/// The zero register.
pub mod emitter;
pub mod encoding;
pub mod frontend;

pub(crate) use emitter::register_name;
pub use encoding::*;
pub use frontend::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    /// The 32-bit encoding.
    pub word: u32,
    /// The assembly the encoding stands for, for listings and reviews.
    pub text: String,
}

/// A compiled RISC-V program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiscvProgram {
    pub instructions: Vec<Instruction>,
}

impl RiscvProgram {
    /// The machine words of the program, in order.
    pub fn words(&self) -> Vec<u32> {
        self.instructions
            .iter()
            .map(|instruction| instruction.word)
            .collect()
    }

    /// The program as an assembly listing, for a build step that places it in
    /// an executable image.
    pub fn to_assembly(&self, symbol: &str) -> String {
        let mut listing = String::new();
        listing.push_str(".section .text.pyac_body, \"ax\", @progbits\n");
        listing.push_str(&format!(".global {symbol}\n"));
        listing.push_str(&format!(".type {symbol}, @function\n"));
        listing.push_str(".balign 4\n");
        // The compressed forms are off, so every instruction in the image is
        // the four-byte encoding the compiler produced.
        listing.push_str(".option norvc\n");
        listing.push_str(&format!("{symbol}:\n"));
        for instruction in &self.instructions {
            listing.push_str(&format!("    {}\n", instruction.text));
        }
        listing.push_str(&format!(".size {symbol}, .-{symbol}\n"));
        listing
    }
}

/// The errors the backend reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendError {
    /// The codelet is not a shape this destination compiles.
    Malformed { cause: String },
    /// The operator word has no proven machine code.
    UnsupportedOperator { actual: u16 },
    /// The operator cannot run with this operand.
    DivisionByZero,
    /// The immediate does not fit the load-immediate sequence.
    ImmediateOutOfRange { value: u64 },
    /// The program contains an instruction the executor does not model.
    UnknownInstruction { word: u32 },
    /// The program ran past its last instruction.
    ProgramUnderrun,
}
impl BackendError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Malformed { .. } => "malformed-codelet",
            Self::UnsupportedOperator { .. } => "unsupported-operator",
            Self::DivisionByZero => "division-by-zero",
            Self::ImmediateOutOfRange { .. } => "immediate-out-of-range",
            Self::UnknownInstruction { .. } => "unknown-instruction",
            Self::ProgramUnderrun => "program-underrun",
        }
    }
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed { cause } => write!(formatter, "the codelet is malformed: {cause}"),
            Self::UnsupportedOperator { actual } => {
                write!(
                    formatter,
                    "operator word 0x{actual:04x} has no RISC-V lowering"
                )
            }
            Self::DivisionByZero => formatter.write_str("numeric division cannot divide by zero"),
            Self::ImmediateOutOfRange { value } => {
                write!(
                    formatter,
                    "immediate {value} does not fit the load-immediate sequence"
                )
            }
            Self::UnknownInstruction { word } => {
                write!(formatter, "instruction 0x{word:08x} is not modelled")
            }
            Self::ProgramUnderrun => {
                formatter.write_str("the program ran past its last instruction")
            }
        }
    }
}

impl std::error::Error for BackendError {}

pub(super) fn malformed(cause: impl Into<String>) -> BackendError {
    BackendError::Malformed {
        cause: cause.into(),
    }
}

#[cfg(test)]
mod tests;
