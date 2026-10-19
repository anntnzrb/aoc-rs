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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Instruction {
    pub(crate) op: Op,
    pub(crate) args: [u64; 3],
}

impl Instruction {
    fn parse(line: &str) -> Result<Self, Error> {
        let mut fields = line.split_whitespace();
        let name = fields
            .next()
            .ok_or_else(|| Error::Syntax(line.to_owned()))?;
        let op = match name {
            "addr" => Op::Add(R),
            "addi" => Op::Add(I),
            "mulr" => Op::Mul(R),
            "muli" => Op::Mul(I),
            "banr" => Op::And(R),
            "bani" => Op::And(I),
            "borr" => Op::Or(R),
            "bori" => Op::Or(I),
            "setr" => Op::Set(R),
            "seti" => Op::Set(I),
            "gtir" => Op::Greater(I, R),
            "gtri" => Op::Greater(R, I),
            "gtrr" => Op::Greater(R, R),
            "eqir" => Op::Equal(I, R),
            "eqri" => Op::Equal(R, I),
            "eqrr" => Op::Equal(R, R),
            _ => return Err(Error::Syntax(line.to_owned())),
        };
        let mut number = || {
            fields
                .next()
                .ok_or_else(|| Error::Syntax(line.to_owned()))?
                .parse::<u64>()
                .map_err(Error::from)
        };
        let args = [number()?, number()?, number()?];
        if fields.next().is_some() {
            return Err(Error::Syntax(line.to_owned()));
        }
        op.execute(&mut [0; 6], args)?;
        Ok(Self { op, args })
    }
}

/// A validated six-register program with an instruction-pointer binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub(crate) binding: usize,
    pub(crate) instructions: Vec<Instruction>,
}

impl Program {
    /// Parses the binding declaration and named instructions.
    ///
    /// # Errors
    /// Returns an error for invalid syntax, register numbers or numeric tokens.
    pub fn parse(input: &str) -> Result<Self, Error> {
        let mut lines = input.lines();
        let declaration = lines
            .next()
            .ok_or_else(|| Error::Syntax(input.to_owned()))?;
        let register: u64 = declaration
            .strip_prefix("#ip ")
            .ok_or_else(|| Error::Syntax(declaration.to_owned()))?
            .parse()?;
        if register >= 6 {
            return Err(Error::Register(register));
        }
        let instructions = lines
            .map(Instruction::parse)
            .collect::<Result<Vec<_>, _>>()?;
        if instructions.is_empty() {
            return Err(Error::Syntax(input.to_owned()));
        }
        Ok(Self {
            binding: usize::try_from(register)?,
            instructions,
        })
    }

    /// Executes one instruction, returning false if the machine has halted.
    ///
    /// # Errors
    /// Returns an error on arithmetic overflow or an invalid machine state.
    pub fn step(&self, machine: &mut Machine) -> Result<bool, Error> {
        let Some(instruction) = self.instructions.get(usize::try_from(machine.ip)?) else {
            return Ok(false);
        };
        *machine
            .registers
            .get_mut(self.binding)
            .ok_or(Error::Register(u64::try_from(self.binding)?))? = machine.ip;
        instruction
            .op
            .execute(&mut machine.registers, instruction.args)?;
        machine.ip = machine
            .registers
            .get(self.binding)
            .copied()
            .ok_or(Error::Register(u64::try_from(self.binding)?))?
            .checked_add(1)
            .ok_or(Error::Overflow)?;
        Ok(true)
    }
}

/// Six registers and the separate next-instruction pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Machine {
    /// Registers, initially all zero.
    pub registers: [u64; 6],
    /// Next instruction, initially zero.
    pub ip: u64,
}

#[cfg(test)]
mod tests {
    use super::{Error, Machine, Program};

    #[test]
    fn binding_writes_before_execution_and_does_not_increment_register() {
        let program =
            Program::parse("#ip 1\nseti 2 999 1\nseti 7 0 0\nseti 8 0 0\nsetr 1 999 0").unwrap();
        let mut machine = Machine::default();
        assert!(
            program.step(&mut machine).unwrap(),
            "absolute jump executes"
        );
        assert_eq!(machine.ip, 3, "jump includes increment");
        assert!(program.step(&mut machine).unwrap(), "destination executes");
        assert_eq!(
            machine.registers,
            [3, 3, 0, 0, 0, 0],
            "bound register is written before reading"
        );
        assert!(!program.step(&mut machine).unwrap(), "out of program halts");
        assert_eq!(machine.registers[1], 3, "halting does not write pointer");
    }

    #[test]
    fn malformed_programs_are_rejected() {
        assert!(
            matches!(Program::parse("#ip 6\nseti 0 0 0"), Err(Error::Register(6))),
            "invalid binding"
        );
        assert!(
            matches!(Program::parse("#ip 0\naddr 6 0 0"), Err(Error::Register(6))),
            "invalid source"
        );
        assert!(
            matches!(Program::parse("#ip 0\nnoop 0 0 0"), Err(Error::Syntax(_))),
            "unknown opcode"
        );
    }
}
