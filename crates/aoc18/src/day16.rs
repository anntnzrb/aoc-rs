//! Chronal samples constrain the unknown numeric names of sixteen register operations.

pub mod primary;

/// Invalid sample syntax or device execution.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// No samples were supplied.
    #[error("input contains no samples")]
    Empty,
    /// A record does not have the required structure.
    #[error("invalid sample or instruction: {0}")]
    Syntax(String),
    /// A numeric token is invalid.
    #[error("invalid number")]
    Number(#[from] std::num::ParseIntError),
    /// A register operand is outside the machine.
    #[error("invalid register {0}")]
    Register(u64),
    /// A register number cannot fit the platform index type.
    #[error("register index is too large")]
    Index(#[from] std::num::TryFromIntError),
    /// Device arithmetic overflowed.
    #[error("device arithmetic overflow")]
    Overflow,
}

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

fn operand(registers: &[u64], value: u64, mode: Mode) -> Result<u64, ParseError> {
    match mode {
        I => Ok(value),
        R => registers
            .get(usize::try_from(value)?)
            .copied()
            .ok_or(ParseError::Register(value)),
    }
}

impl Op {
    pub(crate) fn execute(
        self,
        registers: &mut [u64],
        [a, b, c]: [u64; 3],
    ) -> Result<(), ParseError> {
        let value = match self {
            Self::Add(mode) => operand(registers, a, R)?
                .checked_add(operand(registers, b, mode)?)
                .ok_or(ParseError::Overflow)?,
            Self::Mul(mode) => operand(registers, a, R)?
                .checked_mul(operand(registers, b, mode)?)
                .ok_or(ParseError::Overflow)?,
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
        *registers.get_mut(index).ok_or(ParseError::Register(c))? = value;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Sample {
    pub(crate) before: [u64; 4],
    pub(crate) instruction: [u64; 4],
    pub(crate) after: [u64; 4],
}

/// Parsed observations and the numeric test program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    pub(crate) samples: Vec<Sample>,
    pub(crate) program: Vec<[u64; 4]>,
}

fn numbers(line: &str, separator: char) -> Result<[u64; 4], ParseError> {
    let values = line
        .split(separator)
        .filter(|s| !s.is_empty())
        .map(|s| s.trim().parse::<u64>())
        .collect::<Result<Vec<_>, _>>()?;
    match values.try_into() {
        Ok(array) => Ok(array),
        Err(values) => Err(ParseError::Syntax(format!(
            "expected four numbers, got {}: {line}",
            values.len()
        ))),
    }
}

fn registers(line: &str, prefix: &str) -> Result<[u64; 4], ParseError> {
    let inner = line
        .strip_prefix(prefix)
        .and_then(|s| s.strip_suffix(']'))
        .ok_or_else(|| ParseError::Syntax(line.to_owned()))?;
    numbers(inner, ',')
}

fn instruction(line: &str) -> Result<[u64; 4], ParseError> {
    let values @ [opcode, _, _, c] = numbers(line, ' ')?;
    if opcode >= 16 || c >= 4 {
        return Err(ParseError::Syntax(line.to_owned()));
    }
    Ok(values)
}

/// Parses complete before/instruction/after observations followed by a program.
///
/// # Errors
/// Returns an error for missing observations, malformed records or invalid numbers.
pub fn parse(input: &str) -> Result<Puzzle, ParseError> {
    let mut lines = input
        .lines()
        .filter(|line| !line.trim().is_empty())
        .peekable();
    let mut samples = Vec::new();
    while lines.peek().is_some_and(|line| line.starts_with("Before:")) {
        let before = registers(lines.next().ok_or(ParseError::Empty)?, "Before: [")?;
        let command = instruction(lines.next().ok_or(ParseError::Empty)?)?;
        let after = registers(lines.next().ok_or(ParseError::Empty)?, "After:  [")?;
        samples.push(Sample {
            before,
            instruction: command,
            after,
        });
    }
    if samples.is_empty() {
        return Err(ParseError::Empty);
    }
    let program = lines.map(instruction).collect::<Result<_, _>>()?;
    Ok(Puzzle { samples, program })
}

#[cfg(test)]
mod tests {
    use super::{Mode, Op, ParseError, parse};

    #[test]
    fn empty_input_is_rejected() {
        assert!(
            matches!(parse(""), Err(ParseError::Empty)),
            "no observations"
        );
    }

    #[test]
    fn lines_keep_their_order() {
        let puzzle =
            parse("Before: [0, 1, 2, 3]\n1 0 1 0\nAfter:  [1, 1, 2, 3]\n2 1 0 3\n3 2 1 0\n")
                .unwrap();
        assert_eq!(
            puzzle.program,
            [[2, 1, 0, 3], [3, 2, 1, 0]],
            "instruction order"
        );
        assert!(
            matches!(
                parse("Before: [0, 0]\n0 0 0 0\nAfter:  [0, 0, 0, 0]"),
                Err(ParseError::Syntax(_))
            ),
            "register count"
        );
    }

    #[test]
    fn lines_borrow_the_input_and_strip_crlf() {
        let input = "Before: [0, 1, 2, 3]\r\n\r\n1 0 1 0\r\nAfter:  [1, 1, 2, 3]\r\n";
        let puzzle = parse(input).unwrap();
        assert_eq!(
            puzzle.samples[0].before,
            [0, 1, 2, 3],
            "CRLF and blank records"
        );
        assert_eq!(
            puzzle.samples[0].after,
            [1, 1, 2, 3],
            "parsed state owns its values"
        );
    }

    #[test]
    fn execution_checks_bounds_and_overflow() {
        let mut r = [u64::MAX, 2, 0, 0];
        assert!(
            matches!(
                Op::Add(Mode::Immediate).execute(&mut r, [0, 1, 3]),
                Err(ParseError::Overflow)
            ),
            "checked addition"
        );
        assert!(
            matches!(
                Op::Set(Mode::Register).execute(&mut r, [4, 0, 0]),
                Err(ParseError::Register(4))
            ),
            "source bound"
        );
        assert!(
            matches!(
                Op::Set(Mode::Immediate).execute(&mut r, [7, 99, 4]),
                Err(ParseError::Register(4))
            ),
            "destination bound"
        );
        Op::Set(Mode::Immediate)
            .execute(&mut r, [7, 99, 1])
            .unwrap();
        assert_eq!(
            r,
            [u64::MAX, 7, 0, 0],
            "ignored operand and isolated destination"
        );
    }
}
