//! Numeric instruction shapes: what a decoded codelet means.

use super::words::*;
use crate::VectorElements;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericOperator {
    Plus,
    Subtract,
    Multiply,
    Divide,
    Xor,
    Exponential,
    Remains,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericOperandCase {
    From,
    To,
    With,
    By,
}
impl NumericOperandCase {
    pub(super) fn from_word(word: u16) -> Option<Self> {
        match word {
            SOURCE_CASE => Some(Self::From),
            DESTINATION_CASE => Some(Self::To),
            COMITATIVE_CASE => Some(Self::With),
            INSTRUMENTAL_CASE => Some(Self::By),
            _ => None,
        }
    }
}
impl NumericOperator {
    pub(super) fn from_word(word: u16) -> Option<Self> {
        match word {
            PLUS => Some(Self::Plus),
            SUBTRACT => Some(Self::Subtract),
            MULTIPLY => Some(Self::Multiply),
            DIVIDE => Some(Self::Divide),
            XOR => Some(Self::Xor),
            EXPONENTIAL => Some(Self::Exponential),
            REMAINS => Some(Self::Remains),
            _ => None,
        }
    }

    pub(super) fn word(self) -> u16 {
        match self {
            Self::Plus => PLUS,
            Self::Subtract => SUBTRACT,
            Self::Multiply => MULTIPLY,
            Self::Divide => DIVIDE,
            Self::Xor => XOR,
            Self::Exponential => EXPONENTIAL,
            Self::Remains => REMAINS,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumericArithmeticInstruction {
    pub operator: NumericOperator,
    pub ob: u16,
    pub operand: u16,
    pub operand_case: NumericOperandCase,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumericVectorArithmeticInstruction {
    pub operator: NumericOperator,
    pub elements: Vec<u16>,
    pub operand: NumericVectorOperand,
    pub operand_case: NumericOperandCase,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumericVectorOperand {
    Scalar(u16),
    Vector(Vec<u16>),
}
/// The legacy invert codelet carries the value in the accusative phrase and
/// the bound it is inverted about in the dative phrase. Pyash inverts a
/// number by negating it, so the bound is fixed at zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumericInvertInstruction {
    pub ob: u16,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumericPlusInstruction {
    pub ob: u16,
    pub to: u16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionalNumberInstruction {
    pub left: u16,
    pub right: u16,
    pub consequence: u16,
    pub alternative: u16,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumericCodeletValue {
    Unsigned(u64),
    Signed(i64),
    Vector(VectorElements),
}
