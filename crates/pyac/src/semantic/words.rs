//! The HNUC words the semantic decoders compare against, and the golden codelet.

use crate::WORDS_PER_CODELET;

pub(super) const HEAD: u16 = 0x0149;
pub(super) const NUMBER_QUOTE: u16 = 0x881d;
pub(super) const ACCUSATIVE_CASE: u16 = 0x245e;
pub(super) const SOURCE_CASE: u16 = 0x313e;
pub(super) const DESTINATION_CASE: u16 = 0x243e;
pub(super) const COMITATIVE_CASE: u16 = 0x490f;
pub(super) const INSTRUMENTAL_CASE: u16 = 0x287e;
pub(super) const DATIVE_CASE: u16 = 0x207e;
pub(super) const DEONTIC_MOOD: u16 = 0x295e;
pub(super) const REALIS_MOOD: u16 = 0x217e;
pub(super) const CONDITIONAL_MOOD: u16 = 0x29be;
pub(super) const NUMBER_VERB: u16 = 0xea30;
pub(super) const EQUALLY: u16 = 0x4809;
pub(super) const PLUS: u16 = 0x8a64;
pub(super) const SUBTRACT: u16 = 0xabd2;
pub(super) const MULTIPLY: u16 = 0xec90;
pub(super) const DIVIDE: u16 = 0x8893;
pub(super) const XOR: u16 = 0xa810;
pub(super) const EXPONENTIAL: u16 = 0x8c29;
pub(super) const REMAINS: u16 = 0xe82a;
pub(super) const INVERT: u16 = 0xaad2;
/// The legacy golden codelet: one plus one.
///
/// The golden is the shared fixture for every implementation of the numeric
/// semantics, so a destination compiles the same bytes the interpreter runs.
pub const GOLDEN_NUMERIC_PLUS_CODELET: [u16; WORDS_PER_CODELET] = [
    0x0149, 0x881d, 0x0001, 0x245e, 0x881d, 0x0001, 0x243e, 0x8a64, 0x295e, 0, 0, 0, 0, 0, 0, 0,
];
