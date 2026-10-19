//! The activation program exposes a single equality-controlled halt against register zero.

pub mod primary;

use crate::device::{DeviceError, Mode, Op, Program};

/// Invalid activation program or failed device execution.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// Device parsing or execution failed.
    #[error("device execution failed")]
    Device(#[from] DeviceError),
    /// Register zero must be isolated except for one final equality test.
    #[error("program lacks a single isolated register-zero halt comparison")]
    Structure,
    /// A register or instruction offset cannot fit the platform.
    #[error("index is too large")]
    Index(#[from] std::num::TryFromIntError),
    /// The program halts before reaching its comparison.
    #[error("program halted before its register-zero comparison")]
    Halted,
}

/// A validated program and the location and operand of its halt comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    pub(crate) program: Program,
    pub(crate) comparison: u64,
    pub(crate) candidate: usize,
}

/// Parses and validates the isolated equality-controlled halt structure.
///
/// # Errors
/// Returns an error for malformed instructions or unsupported register-zero dependencies.
pub fn parse(input: &str) -> Result<Puzzle, ParseError> {
    let program = Program::parse(input)?;
    if program.binding == 0 {
        return Err(ParseError::Structure);
    }
    let mut found = None;
    for (index, instruction) in program.instructions.iter().enumerate() {
        let [a, b, c] = instruction.args;
        if c == 0 {
            return Err(ParseError::Structure);
        }
        let (am, bm) = match instruction.op {
            Op::Add(mode) | Op::Mul(mode) | Op::And(mode) | Op::Or(mode) => (Mode::Register, mode),
            Op::Set(mode) => (mode, Mode::Immediate),
            Op::Greater(am, bm) | Op::Equal(am, bm) => (am, bm),
        };
        if (am == Mode::Register && a == 0) || (bm == Mode::Register && b == 0) {
            if instruction.op != Op::Equal(Mode::Register, Mode::Register)
                || a == b
                || found.is_some()
            {
                return Err(ParseError::Structure);
            }
            found = Some((index, if a == 0 { b } else { a }, c));
        }
    }
    let (index, candidate, flag) = found.ok_or(ParseError::Structure)?;
    let tail = program
        .instructions
        .get(index.saturating_add(1)..)
        .ok_or(ParseError::Structure)?;
    let [branch, restart] = tail else {
        return Err(ParseError::Structure);
    };
    let ip = u64::try_from(program.binding)?;
    if branch.op != Op::Add(Mode::Register)
        || branch.args != [flag, ip, ip]
        || restart.op != Op::Set(Mode::Immediate)
    {
        return Err(ParseError::Structure);
    }
    let [destination, _, output] = restart.args;
    if output != ip || destination >= u64::try_from(index)? {
        return Err(ParseError::Structure);
    }
    Ok(Puzzle {
        program,
        comparison: u64::try_from(index)?,
        candidate: usize::try_from(candidate)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{ParseError, parse};

    #[test]
    fn empty_input_is_rejected() {
        assert!(
            matches!(parse(""), Err(ParseError::Device(_))),
            "no declaration"
        );
    }

    #[test]
    fn lines_keep_their_order() {
        assert!(
            matches!(parse("#ip 5\nseti 1 0 0"), Err(ParseError::Structure)),
            "register zero must not be written"
        );
    }

    #[test]
    fn lines_borrow_the_input_and_strip_crlf() {
        let puzzle =
            parse("#ip 5\r\nseti 7 0 1\r\neqrr 1 0 2\r\naddr 2 5 5\r\nseti 0 0 5\r\n").unwrap();
        assert_eq!(puzzle.comparison, 1, "CRLF preserves order");
        assert_eq!(puzzle.candidate, 1, "comparison register");
    }
}
