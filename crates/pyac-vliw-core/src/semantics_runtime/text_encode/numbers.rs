//! Numeric literals: the profile, and how a digit run collects into a value.
//!
//! A number is a typed literal, not text: `dohyiktyut` is uint16(12) in decimal,
//! `tc6h` makes it signed, `fyih` float, and the width words `po`/`do`/`pu`/`tl6h`
//! select 8/16/32/64 bits.

use super::errors::*;
use super::glyph::*;
use super::words::*;
/// How a numeric literal's digit words combine into a value.
///
/// Decimal: `hyik tyut` is twelve, because the digit words are the ten decimal
/// digits and a positional run reads in written order. The digit table also holds
/// ten through fifteen (`htip`..`hpet`) for a hexadecimal profile, but those are
/// rejected under the decimal default where the value is not below the radix.
///
/// Big-endian is the default because it reads in the order it is written —
/// `do hyik tyut` is 1*10 + 2 — and because the leading terminator means the
/// digits arrive already in that order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NumberFormat {
    /// Radix. Digit words with a value at or above this are rejected.
    pub base: u32,
    /// Whether the first digit is the most significant.
    pub big_endian: bool,
}
impl NumberFormat {
    /// Decimal, most significant digit first: `do hyik tyut` is twelve.
    ///
    /// Big-endian because it reads in the order it is written and the leading
    /// terminator means the digits arrive already in that order.
    pub const DEFAULT: Self = Self {
        base: 10,
        big_endian: true,
    };

    /// Base 16, most significant digit first. A hexadecimal profile uses the
    /// digit words through `hpet` (15).
    pub const HEX: Self = Self {
        base: 16,
        big_endian: true,
    };

    /// The historical C encoding: base 16, least significant digit first.
    pub const LEGACY_HEX_LITTLE_ENDIAN: Self = Self {
        base: 16,
        big_endian: false,
    };
}
pub(super) fn number_digit(honuc: u32) -> Option<u32> {
    NUMBER_DIGITS
        .iter()
        .find(|(word, _)| *word == honuc)
        .map(|(_, digit)| *digit)
}
