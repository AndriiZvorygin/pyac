//! The register-program emitter: instruction sequences for one operator.
//!
//! The backend emits no branches. That is a deliberate limit, not an oversight:
//! a branchless program needs no label resolution, so the emitted words are the
//! whole program and the executor checks them without a control-flow model.

use super::encoding::*;
use crate::backend::riscv::{BackendError, Instruction, T3, T4, T5, T6};

/// The register names the listings use.
pub(crate) fn register_name(register: u8) -> &'static str {
    match register {
        0 => "zero",
        1 => "ra",
        2 => "sp",
        5 => "t0",
        6 => "t1",
        7 => "t2",
        8 => "s0",
        9 => "s1",
        10 => "a0",
        11 => "a1",
        12 => "a2",
        13 => "a3",
        14 => "a4",
        15 => "a5",
        16 => "a6",
        17 => "a7",
        18 => "s2",
        19 => "s3",
        20 => "s4",
        21 => "s5",
        22 => "s6",
        23 => "s7",
        24 => "s8",
        25 => "s9",
        26 => "s10",
        27 => "s11",
        28 => "t3",
        29 => "t4",
        30 => "t5",
        31 => "t6",
        _ => "unknown",
    }
}

/// The builder that accumulates a program.
///
/// The register allocation is fixed. `t0` and `t1` hold the operands, `t2`
/// holds the result, `t3` holds the running factor of a power, and `t4` to
/// `t6` are the scratch space the saturation sequences use. `a0` receives the
/// result, and `s2` to `s5` are the scratch space the power sequence uses.
pub(super) struct Emitter {
    pub(super) instructions: Vec<Instruction>,
}

impl Emitter {
    pub(super) fn new() -> Self {
        Self {
            instructions: Vec::new(),
        }
    }

    pub(super) fn push(&mut self, word: u32, text: String) {
        self.instructions.push(Instruction { word, text });
    }

    pub(super) fn emit(&mut self, word: u32, text: impl Into<String>) {
        self.push(word, text.into());
    }

    /// `mv rd, rs`
    pub(super) fn move_register(&mut self, rd: u8, rs: u8) {
        self.emit(
            addi(rd, rs, 0),
            format!("addi {}, {}, 0", register_name(rd), register_name(rs)),
        );
    }

    /// `neg rd, rs`
    pub(super) fn negate(&mut self, rd: u8, rs: u8) {
        self.emit(
            sub(rd, ZERO, rs),
            format!("sub {}, zero, {}", register_name(rd), register_name(rs)),
        );
    }

    /// `not rd, rs`
    pub(super) fn bitwise_not(&mut self, rd: u8, rs: u8) {
        self.emit(
            xori(rd, rs, -1),
            format!("xori {}, {}, -1", register_name(rd), register_name(rs)),
        );
    }

    /// `li rd, value`, using the two-instruction sequence when the value does
    /// not fit a signed 12-bit immediate.
    pub(super) fn load_immediate(&mut self, rd: u8, value: u64) -> Result<(), BackendError> {
        if value > 0x7fff_efff {
            return Err(BackendError::ImmediateOutOfRange { value });
        }
        let low = ((value & 0xfff) as i64) << 52 >> 52;
        let high = ((value as i64).wrapping_sub(low) >> 12) & 0xfffff;
        if high == 0 && low >= -2048 && low <= 2047 {
            self.emit(
                addi(rd, ZERO, low as i32),
                format!("addi {}, zero, {}", register_name(rd), low),
            );
            return Ok(());
        }
        self.emit(
            lui(rd, high as u32),
            format!("lui {}, 0x{:x}", register_name(rd), high),
        );
        if low != 0 {
            self.emit(
                addi(rd, rd, low as i32),
                format!("addi {}, {}, {}", register_name(rd), register_name(rd), low),
            );
        }
        Ok(())
    }

    /// The numeric limit, `0xffff`, which needs the two-instruction sequence.
    pub(super) fn load_numeric_max(&mut self, rd: u8) {
        self.emit(lui(rd, 0x10), format!("lui {}, 0x10", register_name(rd)));
        self.emit(
            addi(rd, rd, -1),
            format!("addi {}, {}, -1", register_name(rd), register_name(rd)),
        );
    }

    /// The saturating product of two 16-bit values, which is the saturation
    /// the interpreter applies to addition, subtraction, and exponentiation.
    ///
    /// The product of two values below `0xffff` fits in 32 bits, so the high
    /// half of the product says whether the result crossed the limit.
    pub(super) fn saturating_multiply(&mut self, rd: u8, left: u8, right: u8) {
        self.emit(
            mul(T4, left, right),
            format!(
                "mul {}, {}, {}",
                register_name(T4),
                register_name(left),
                register_name(right)
            ),
        );
        self.emit(
            srli(T5, T4, 16),
            format!("srli {}, {}, 16", register_name(T5), register_name(T4)),
        );
        self.emit(
            sltu(T5, ZERO, T5),
            format!("sltu {}, zero, {}", register_name(T5), register_name(T5)),
        );
        self.negate(T5, T5);
        self.bitwise_not(T6, T5);
        self.emit(
            and(T4, T4, T6),
            format!(
                "and {}, {}, {}",
                register_name(T4),
                register_name(T4),
                register_name(T6)
            ),
        );
        self.load_numeric_max(T6);
        self.emit(
            and(T6, T6, T5),
            format!(
                "and {}, {}, {}",
                register_name(T6),
                register_name(T6),
                register_name(T5)
            ),
        );
        self.emit(
            or(T4, T4, T6),
            format!(
                "or {}, {}, {}",
                register_name(T4),
                register_name(T4),
                register_name(T6)
            ),
        );
        self.move_register(rd, T4);
    }

    /// The saturating sum of two 16-bit values.
    pub(super) fn saturating_add(&mut self, rd: u8, left: u8, right: u8) {
        self.emit(
            add(rd, left, right),
            format!(
                "add {}, {}, {}",
                register_name(rd),
                register_name(left),
                register_name(right)
            ),
        );
        self.load_numeric_max(T4);
        self.emit(
            sltu(T5, T4, rd),
            format!(
                "sltu {}, {}, {}",
                register_name(T5),
                register_name(T4),
                register_name(rd)
            ),
        );
        self.negate(T5, T5);
        self.bitwise_not(T6, T5);
        self.emit(
            and(rd, rd, T6),
            format!(
                "and {}, {}, {}",
                register_name(rd),
                register_name(rd),
                register_name(T6)
            ),
        );
        self.emit(
            and(T4, T4, T5),
            format!(
                "and {}, {}, {}",
                register_name(T4),
                register_name(T4),
                register_name(T5)
            ),
        );
        self.emit(
            or(rd, rd, T4),
            format!(
                "or {}, {}, {}",
                register_name(rd),
                register_name(rd),
                register_name(T4)
            ),
        );
    }

    /// The saturating difference of two 16-bit values, which stops at zero.
    pub(super) fn saturating_subtract(&mut self, rd: u8, left: u8, right: u8) {
        self.emit(
            sub(rd, left, right),
            format!(
                "sub {}, {}, {}",
                register_name(rd),
                register_name(left),
                register_name(right)
            ),
        );
        self.emit(
            slt(T4, rd, ZERO),
            format!("slt {}, {}, zero", register_name(T4), register_name(rd)),
        );
        self.negate(T4, T4);
        self.bitwise_not(T5, T4);
        self.emit(
            and(rd, rd, T5),
            format!(
                "and {}, {}, {}",
                register_name(rd),
                register_name(rd),
                register_name(T5)
            ),
        );
    }

    /// The saturating power of two 16-bit values.
    ///
    /// The exponent is at most `0xffff`, so sixteen rounds of binary
    /// exponentiation cover it. Every round is branchless: the exponent bit
    /// selects the factor with a mask, and each product saturates.
    pub(super) fn saturating_power(&mut self, rd: u8, base: u8, exponent: u8) {
        self.load_immediate(rd, 1)
            .expect("one fits a load-immediate sequence");
        self.move_register(T3, base);
        for _ in 0..16 {
            // The factor enters the product only when the exponent bit is set,
            // and a clear bit multiplies by one instead of by zero.
            self.emit(
                andi(S3, exponent, 1),
                format!("andi {}, {}, 1", register_name(S3), register_name(exponent)),
            );
            self.negate(S3, S3);
            self.emit(
                and(S2, T3, S3),
                format!(
                    "and {}, {}, {}",
                    register_name(S2),
                    register_name(T3),
                    register_name(S3)
                ),
            );
            self.bitwise_not(S4, S3);
            self.emit(
                andi(S5, S4, 1),
                format!("andi {}, {}, 1", register_name(S5), register_name(S4)),
            );
            self.emit(
                or(S2, S2, S5),
                format!(
                    "or {}, {}, {}",
                    register_name(S2),
                    register_name(S2),
                    register_name(S5)
                ),
            );
            self.emit(
                srli(exponent, exponent, 1),
                format!(
                    "srli {}, {}, 1",
                    register_name(exponent),
                    register_name(exponent)
                ),
            );
            self.saturating_multiply(rd, rd, S2);
            self.saturating_multiply(T3, T3, T3);
        }
    }
}
