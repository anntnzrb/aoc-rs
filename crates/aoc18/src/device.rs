//! Checked opcode execution and six-register machines with a bound instruction pointer.
//!
//! Operations execute against register slices; programs bind the instruction pointer before
//! execution and read it back before incrementing. Parsing validates all register operands.

/// Invalid device input or execution.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DeviceError {
    /// A register operand lies outside the machine.
    #[error("invalid register {0}")]
    Register(u64),
    /// A register number cannot fit an index.
    #[error("register index is too large")]
    Index(#[from] std::num::TryFromIntError),
    /// Arithmetic exceeds the register width.
    #[error("device arithmetic overflow")]
    Overflow,
    /// Program syntax is malformed.
    #[error("invalid device instruction: {0}")]
    Syntax(String),
    /// A numeric token is invalid.
    #[error("invalid device number")]
    Number(#[from] std::num::ParseIntError),
}

use DeviceError as Error;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Register,
    Immediate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Op {
    Add(Mode),
    Mul(Mode),
    And(Mode),
    Or(Mode),
    Set(Mode),
    Greater(Mode, Mode),
    Equal(Mode, Mode),
}

use Mode::{Immediate as I, Register as R};

pub(crate) const OPS: [Op; 16] = [
    Op::Add(R),
    Op::Add(I),
    Op::Mul(R),
    Op::Mul(I),
    Op::And(R),
    Op::And(I),
    Op::Or(R),
    Op::Or(I),
    Op::Set(R),
    Op::Set(I),
    Op::Greater(I, R),
    Op::Greater(R, I),
    Op::Greater(R, R),
    Op::Equal(I, R),
    Op::Equal(R, I),
    Op::Equal(R, R),
];

fn operand(registers: &[u64], value: u64, mode: Mode) -> Result<u64, Error> {
    match mode {
        I => Ok(value),
        R => registers
            .get(usize::try_from(value)?)
            .copied()
            .ok_or(Error::Register(value)),
    }
}

impl Op {
    pub(crate) fn execute(self, registers: &mut [u64], [a, b, c]: [u64; 3]) -> Result<(), Error> {
        let value = match self {
            Self::Add(mode) => operand(registers, a, R)?
                .checked_add(operand(registers, b, mode)?)
                .ok_or(Error::Overflow)?,
            Self::Mul(mode) => operand(registers, a, R)?
                .checked_mul(operand(registers, b, mode)?)
                .ok_or(Error::Overflow)?,
            Self::And(mode) => operand(registers, a, R)? & operand(registers, b, mode)?,
            Self::Or(mode) => operand(registers, a, R)? | operand(registers, b, mode)?,
            Self::Set(mode) => operand(registers, a, mode)?,
            Self::Greater(am, bm) => {
                u64::from(operand(registers, a, am)? > operand(registers, b, bm)?)
            }
            Self::Equal(am, bm) => {
                u64::from(operand(registers, a, am)? == operand(registers, b, bm)?)
            }
        };
        let index = usize::try_from(c)?;
        *registers.get_mut(index).ok_or(Error::Register(c))? = value;
        Ok(())
    }
}

