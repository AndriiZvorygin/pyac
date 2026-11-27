//! Register numbers and the RV32I instruction encoders.
//!
//! Each function returns one 32-bit instruction word for a specific register
//! form. They are the only place a bit layout is written down, so a field-order
//! mistake shows up as one wrong word rather than everywhere.

#![allow(dead_code)]

pub const ZERO: u8 = 0;
/// The return address register.
pub const RA: u8 = 1;
/// The first temporary register.
pub const T0: u8 = 5;
/// The second temporary register.
pub const T1: u8 = 6;
/// The third temporary register.
pub const T2: u8 = 7;
/// The fourth temporary register.
pub const T3: u8 = 28;
/// The fifth temporary register.
pub const T4: u8 = 29;
/// The sixth temporary register.
pub const T5: u8 = 30;
/// The seventh temporary register.
pub const T6: u8 = 31;
/// The first argument register, where the result leaves.
pub const A0: u8 = 10;
/// The first saved register available as scratch space.
pub const S2: u8 = 18;
/// The second saved register available as scratch space.
pub const S3: u8 = 19;
/// The third saved register available as scratch space.
pub const S4: u8 = 20;
/// The fourth saved register available as scratch space.
pub const S5: u8 = 21;

/// The largest value the interpreter's numeric semantics produce.
pub const NUMERIC_MAX: u64 = 0xffff;

pub(super) const OPCODE_OP: u32 = 0x33;
pub(super) const OPCODE_OP_IMM: u32 = 0x13;
pub(super) const OPCODE_LUI: u32 = 0x37;
pub(super) const OPCODE_JALR: u32 = 0x67;
pub(super) const OPCODE_SYSTEM: u32 = 0x73;

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

fn malformed(cause: impl Into<String>) -> BackendError {
    BackendError::Malformed {
        cause: cause.into(),
    }
}

fn r_type(funct7: u32, funct3: u32, rs1: u8, rs2: u8, rd: u8, opcode: u32) -> u32 {
    ((funct7 & 0x7f) << 25)
        | ((rs2 as u32 & 0x1f) << 20)
        | ((rs1 as u32 & 0x1f) << 15)
        | ((funct3 & 0x7) << 12)
        | ((rd as u32 & 0x1f) << 7)
        | (opcode & 0x7f)
}

fn i_type(imm: i32, rs1: u8, funct3: u32, rd: u8, opcode: u32) -> u32 {
    (((imm as u32) & 0xfff) << 20)
        | ((rs1 as u32 & 0x1f) << 15)
        | ((funct3 & 0x7) << 12)
        | ((rd as u32 & 0x1f) << 7)
        | (opcode & 0x7f)
}

fn u_type(imm: u32, rd: u8, opcode: u32) -> u32 {
    ((imm & 0xfffff) << 12) | ((rd as u32 & 0x1f) << 7) | (opcode & 0x7f)
}

/// `addi rd, rs1, imm`
pub fn addi(rd: u8, rs1: u8, imm: i32) -> u32 {
    i_type(imm, rs1, 0b000, rd, OPCODE_OP_IMM)
}

/// `sltiu rd, rs1, imm`
pub fn sltiu(rd: u8, rs1: u8, imm: i32) -> u32 {
    i_type(imm, rs1, 0b011, rd, OPCODE_OP_IMM)
}

/// `slli rd, rs1, imm`
pub fn slli(rd: u8, rs1: u8, imm: i32) -> u32 {
    i_type(imm, rs1, 0b001, rd, OPCODE_OP_IMM)
}

/// `srli rd, rs1, imm`
pub fn srli(rd: u8, rs1: u8, imm: i32) -> u32 {
    i_type(imm, rs1, 0b101, rd, OPCODE_OP_IMM)
}

/// `xori rd, rs1, imm`
pub fn xori(rd: u8, rs1: u8, imm: i32) -> u32 {
    i_type(imm, rs1, 0b100, rd, OPCODE_OP_IMM)
}

/// `andi rd, rs1, imm`
pub fn andi(rd: u8, rs1: u8, imm: i32) -> u32 {
    i_type(imm, rs1, 0b111, rd, OPCODE_OP_IMM)
}

/// `add rd, rs1, rs2`
pub fn add(rd: u8, rs1: u8, rs2: u8) -> u32 {
    r_type(0x00, 0b000, rs1, rs2, rd, OPCODE_OP)
}

/// `sub rd, rs1, rs2`
pub fn sub(rd: u8, rs1: u8, rs2: u8) -> u32 {
    r_type(0x20, 0b000, rs1, rs2, rd, OPCODE_OP)
}

/// `slt rd, rs1, rs2`
pub fn slt(rd: u8, rs1: u8, rs2: u8) -> u32 {
    r_type(0x00, 0b010, rs1, rs2, rd, OPCODE_OP)
}

/// `sltu rd, rs1, rs2`
pub fn sltu(rd: u8, rs1: u8, rs2: u8) -> u32 {
    r_type(0x00, 0b011, rs1, rs2, rd, OPCODE_OP)
}

/// `xor rd, rs1, rs2`
pub fn xor(rd: u8, rs1: u8, rs2: u8) -> u32 {
    r_type(0x00, 0b100, rs1, rs2, rd, OPCODE_OP)
}

/// `srl rd, rs1, rs2`
pub fn srl(rd: u8, rs1: u8, rs2: u8) -> u32 {
    r_type(0x00, 0b101, rs1, rs2, rd, OPCODE_OP)
}

/// `or rd, rs1, rs2`
pub fn or(rd: u8, rs1: u8, rs2: u8) -> u32 {
    r_type(0x00, 0b110, rs1, rs2, rd, OPCODE_OP)
}

/// `and rd, rs1, rs2`
pub fn and(rd: u8, rs1: u8, rs2: u8) -> u32 {
    r_type(0x00, 0b111, rs1, rs2, rd, OPCODE_OP)
}

/// `mul rd, rs1, rs2`
pub fn mul(rd: u8, rs1: u8, rs2: u8) -> u32 {
    r_type(0x01, 0b000, rs1, rs2, rd, OPCODE_OP)
}

/// `divu rd, rs1, rs2`
pub fn divu(rd: u8, rs1: u8, rs2: u8) -> u32 {
    r_type(0x01, 0b101, rs1, rs2, rd, OPCODE_OP)
}

/// `remu rd, rs1, rs2`
pub fn remu(rd: u8, rs1: u8, rs2: u8) -> u32 {
    r_type(0x01, 0b111, rs1, rs2, rd, OPCODE_OP)
}

/// `lui rd, imm20`
pub fn lui(rd: u8, imm: u32) -> u32 {
    u_type(imm, rd, OPCODE_LUI)
}

/// `jalr rd, rs1, imm`
pub fn jalr(rd: u8, rs1: u8, imm: i32) -> u32 {
    i_type(imm, rs1, 0b000, rd, OPCODE_JALR)
}

/// `ebreak`
pub fn ebreak() -> u32 {
    i_type(1, 0, 0b000, 0, OPCODE_SYSTEM)
}

/// The `ret` encoding, `jalr x0, ra, 0`.
pub fn ret() -> u32 {
    jalr(ZERO, RA, 0)
}
