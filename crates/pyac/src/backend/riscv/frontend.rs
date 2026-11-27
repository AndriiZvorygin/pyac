//! The compiler front end: a codelet in, a register program out.
//!
//! The compiled program must agree with the interpreter, so the numeric
//! semantics are the interpreter's, including the saturation to `0xffff` that
//! applies to addition, subtraction, and exponentiation.

use super::emitter::{Emitter, register_name};
use super::encoding::*;
use crate::WORDS_PER_CODELET;
use crate::backend::riscv::{BackendError, RiscvProgram, malformed};
use crate::semantic::{NumericOperator, decode_numeric_arithmetic, decode_numeric_invert};

/// Compile a numeric arithmetic codelet into a RISC-V program.
pub fn compile_numeric_arithmetic(
    words: &[u16; WORDS_PER_CODELET],
) -> Result<RiscvProgram, BackendError> {
    let instruction =
        decode_numeric_arithmetic(words).map_err(|error| malformed(error.to_string()))?;
    let mut emitter = Emitter::new();

    emitter.load_immediate(T0, u64::from(instruction.ob))?;
    emitter.load_immediate(T1, u64::from(instruction.operand))?;

    match instruction.operator {
        NumericOperator::Plus => emitter.saturating_add(T2, T0, T1),
        NumericOperator::Subtract => emitter.saturating_subtract(T2, T0, T1),
        NumericOperator::Multiply => emitter.emit(
            mul(T2, T0, T1),
            format!(
                "mul {}, {}, {}",
                register_name(T2),
                register_name(T0),
                register_name(T1)
            ),
        ),
        NumericOperator::Divide => {
            if instruction.operand == 0 {
                return Err(BackendError::DivisionByZero);
            }
            emitter.emit(
                divu(T2, T0, T1),
                format!(
                    "divu {}, {}, {}",
                    register_name(T2),
                    register_name(T0),
                    register_name(T1)
                ),
            );
        }
        NumericOperator::Xor => emitter.emit(
            xor(T2, T0, T1),
            format!(
                "xor {}, {}, {}",
                register_name(T2),
                register_name(T0),
                register_name(T1)
            ),
        ),
        NumericOperator::Exponential => emitter.saturating_power(T2, T0, T1),
        NumericOperator::Remains => {
            if instruction.operand == 0 {
                return Err(BackendError::DivisionByZero);
            }
            emitter.emit(
                remu(T2, T0, T1),
                format!(
                    "remu {}, {}, {}",
                    register_name(T2),
                    register_name(T0),
                    register_name(T1)
                ),
            );
        }
    }

    emitter.move_register(A0, T2);
    emitter.emit(ret(), "ret");
    Ok(RiscvProgram {
        instructions: emitter.instructions,
    })
}

/// Compile an invert codelet into a RISC-V program.
///
/// Pyash inverts a number by negating it, so the result is negative.
pub fn compile_numeric_invert(
    words: &[u16; WORDS_PER_CODELET],
) -> Result<RiscvProgram, BackendError> {
    let instruction = decode_numeric_invert(words).map_err(|error| malformed(error.to_string()))?;
    let mut emitter = Emitter::new();

    emitter.load_immediate(T0, u64::from(instruction.ob))?;
    emitter.negate(T2, T0);
    emitter.move_register(A0, T2);
    emitter.emit(ret(), "ret");
    Ok(RiscvProgram {
        instructions: emitter.instructions,
    })
}

/// Run a compiled program and return the result it leaves in `a0`.
///
/// The executor models the instruction subset the backend emits. A `ret` ends
/// the program, which is how the compiled body hands its result back.
pub fn execute(program: &RiscvProgram) -> Result<u64, BackendError> {
    let mut registers = [0u64; 32];
    let mut pc = 0usize;

    loop {
        let word = program
            .instructions
            .get(pc)
            .ok_or(BackendError::ProgramUnderrun)?
            .word;
        let opcode = word & 0x7f;
        let rd = ((word >> 7) & 0x1f) as usize;
        let funct3 = (word >> 12) & 0x7;
        let rs1 = ((word >> 15) & 0x1f) as usize;
        let rs2 = ((word >> 20) & 0x1f) as usize;
        let funct7 = (word >> 25) & 0x7f;
        let immediate = ((word as i32) >> 20) as i64;

        match opcode {
            OPCODE_LUI => {
                registers[rd] = (word & 0xffff_f000) as u32 as i32 as i64 as u64;
                pc += 1;
            }
            OPCODE_OP_IMM => {
                let value = match funct3 {
                    0b000 => registers[rs1].wrapping_add(immediate as u64),
                    0b001 => registers[rs1] << (immediate as u64 & 0x3f),
                    0b010 => u64::from((registers[rs1] as i64) < immediate),
                    0b011 => u64::from(registers[rs1] < immediate as u64),
                    0b101 => registers[rs1] >> (immediate as u64 & 0x3f),
                    0b100 => registers[rs1] ^ (immediate as u64),
                    0b110 => registers[rs1] | (immediate as u64),
                    0b111 => registers[rs1] & (immediate as u64),
                    _ => return Err(BackendError::UnknownInstruction { word }),
                };
                registers[rd] = value;
                pc += 1;
            }
            OPCODE_OP => {
                let value = match (funct7, funct3) {
                    (0x00, 0b000) => registers[rs1].wrapping_add(registers[rs2]),
                    (0x00, 0b001) => registers[rs1] << (registers[rs2] & 0x3f),
                    (0x00, 0b010) => u64::from((registers[rs1] as i64) < (registers[rs2] as i64)),
                    (0x00, 0b011) => u64::from(registers[rs1] < registers[rs2]),
                    (0x00, 0b100) => registers[rs1] ^ registers[rs2],
                    (0x00, 0b101) => registers[rs1] >> (registers[rs2] & 0x3f),
                    (0x00, 0b110) => registers[rs1] | registers[rs2],
                    (0x00, 0b111) => registers[rs1] & registers[rs2],
                    (0x20, 0b000) => registers[rs1].wrapping_sub(registers[rs2]),
                    (0x01, 0b000) => registers[rs1].wrapping_mul(registers[rs2]),
                    (0x01, 0b101) => {
                        if registers[rs2] == 0 {
                            u64::MAX
                        } else {
                            registers[rs1] / registers[rs2]
                        }
                    }
                    (0x01, 0b111) => {
                        if registers[rs2] == 0 {
                            registers[rs1]
                        } else {
                            registers[rs1] % registers[rs2]
                        }
                    }
                    _ => return Err(BackendError::UnknownInstruction { word }),
                };
                registers[rd] = value;
                pc += 1;
            }
            OPCODE_JALR => {
                if rd == ZERO as usize && immediate == 0 {
                    return Ok(registers[A0 as usize]);
                }
                return Err(BackendError::UnknownInstruction { word });
            }
            OPCODE_SYSTEM => {
                if word == ebreak() {
                    return Ok(registers[A0 as usize]);
                }
                return Err(BackendError::UnknownInstruction { word });
            }
            _ => return Err(BackendError::UnknownInstruction { word }),
        }
    }
}

/// Compile and run a numeric arithmetic codelet, and return its result.
pub fn compile_and_execute_numeric_arithmetic(
    words: &[u16; WORDS_PER_CODELET],
) -> Result<u64, BackendError> {
    let program = compile_numeric_arithmetic(words)?;
    execute(&program)
}
