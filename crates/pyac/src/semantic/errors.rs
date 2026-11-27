//! The semantic error and its display.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticError {
    InvalidShape {
        index: usize,
        expected: u16,
        actual: u16,
    },
    NonzeroTail {
        index: usize,
        actual: u16,
    },
    UnsupportedOperator {
        actual: u16,
    },
    UnsupportedCase {
        actual: u16,
    },
    UnsupportedBound {
        actual: u16,
    },
    UnsupportedVectorWidth {
        actual: usize,
        maximum: usize,
    },
    InvalidBranchShape {
        index: usize,
        expected: u16,
        actual: u16,
    },
    UnsupportedBranch,
    VectorWidthMismatch {
        left: usize,
        right: usize,
    },
    UnsupportedCodelet,
    DivisionByZero,
    NonFiniteFloat,
    Fixed16Domain,
    MalformedStorageSection,
    MalformedProgramlet,
    UnsupportedProgramlet,
    ProgramletFrameCapacity,
    ProgramletConstantWrite,
    ProgramletStepBudget,
    ProgramletProgramCapacity,
    ProgramletCallDepth,
}
impl SemanticError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidShape { .. } => "malformed-codelet",
            Self::NonzeroTail { .. } => "nonzero-tail",
            Self::UnsupportedOperator { .. } => "unsupported-operator",
            Self::UnsupportedCase { .. } => "unsupported-case",
            Self::UnsupportedBound { .. } => "unsupported-sentence",
            Self::UnsupportedVectorWidth { .. } => "unsupported-vector-width",
            Self::InvalidBranchShape { .. } => "malformed-branch",
            Self::UnsupportedBranch => "unsupported-branch",
            Self::VectorWidthMismatch { .. } => "vector-width-mismatch",
            Self::UnsupportedCodelet => "unsupported-codelet",
            Self::DivisionByZero => "division-by-zero",
            Self::NonFiniteFloat => "non-finite-float",
            Self::Fixed16Domain => "fixed16-domain",
            Self::MalformedStorageSection => "malformed-storage-section",
            Self::MalformedProgramlet => "malformed-programlet",
            Self::UnsupportedProgramlet => "unsupported-programlet",
            Self::ProgramletFrameCapacity => "programlet-frame-capacity",
            Self::ProgramletConstantWrite => "programlet-constant-write",
            Self::ProgramletStepBudget => "programlet-step-budget",
            Self::ProgramletProgramCapacity => "programlet-program-capacity",
            Self::ProgramletCallDepth => "programlet-call-depth",
        }
    }
}
impl std::fmt::Display for SemanticError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidShape {
                index,
                expected,
                actual,
            } => write!(
                formatter,
                "numeric-plus marker at word {index} expected 0x{expected:04x}, got 0x{actual:04x}"
            ),
            Self::NonzeroTail { index, actual } => {
                write!(
                    formatter,
                    "numeric-plus tail at word {index} is 0x{actual:04x}"
                )
            }
            Self::UnsupportedOperator { actual } => {
                write!(
                    formatter,
                    "numeric arithmetic operator 0x{actual:04x} is unsupported"
                )
            }
            Self::UnsupportedCase { actual } => write!(
                formatter,
                "numeric arithmetic has an unsupported operand case 0x{actual:04x}"
            ),
            Self::UnsupportedBound { actual } => {
                write!(
                    formatter,
                    "numeric invert only supports a zero bound, got {actual}"
                )
            }
            Self::UnsupportedVectorWidth { actual, maximum } => write!(
                formatter,
                "numeric vector arithmetic width {actual} is outside the supported range 2..={maximum}"
            ),
            Self::InvalidBranchShape {
                index,
                expected,
                actual,
            } => write!(
                formatter,
                "conditional branch word {index} expected 0x{expected:04x}, got 0x{actual:04x}"
            ),
            Self::UnsupportedBranch => {
                formatter.write_str("the conditional branch shape is not supported")
            }
            Self::VectorWidthMismatch { left, right } => write!(
                formatter,
                "numeric vector arithmetic requires equal operand widths, got {left} and {right}"
            ),
            Self::UnsupportedCodelet => formatter.write_str("the codelet family is unsupported"),
            Self::DivisionByZero => formatter.write_str("numeric division cannot divide by zero"),
            Self::NonFiniteFloat => {
                formatter.write_str("f32 values and results must remain finite")
            }
            Self::Fixed16Domain => {
                formatter.write_str("fixed16 result is outside the supported u16 range")
            }
            Self::MalformedStorageSection => {
                formatter.write_str("the PyAC storage section packet is malformed")
            }
            Self::MalformedProgramlet => {
                formatter.write_str("the canonical Pyash programlet is malformed")
            }
            Self::UnsupportedProgramlet => {
                formatter.write_str("the canonical Pyash programlet form is not supported")
            }
            Self::ProgramletFrameCapacity => {
                formatter.write_str("the programlet runtime frame capacity was exceeded")
            }
            Self::ProgramletConstantWrite => {
                formatter.write_str("a programlet attempted to write a constant quantity")
            }
            Self::ProgramletStepBudget => {
                formatter.write_str("the programlet execution step budget was exhausted")
            }
            Self::ProgramletProgramCapacity => {
                formatter.write_str("the programlet exceeded a routine or section resource limit")
            }
            Self::ProgramletCallDepth => {
                formatter.write_str("the programlet call depth exceeded the supported limit")
            }
        }
    }
}

impl std::error::Error for SemanticError {}
