// approach: validate an isolated register-zero halt equality and observe its first candidate.
// cost: O(instructions before first comparison) time, O(program length) memory, one input pass.
// trade: device simulation stays authoritative; unsupported dependencies return a typed error.

//! Finds the first value tested against an otherwise isolated register zero.

use super::{ParseError, parse};
use crate::device::Machine;
use aoc_core::Answers;

/// Returns the earliest halting register-zero value; part two renders zero.
///
/// # Errors
/// Returns an error for unsupported control flow, malformed programs or overflow.
pub fn solve(input: &str) -> Result<Answers, ParseError> {
    let puzzle = parse(input)?;
    let mut machine = Machine::default();
    while machine.ip != puzzle.comparison {
        if !puzzle.program.step(&mut machine)? {
            return Err(ParseError::Halted);
        }
    }
    let answer = machine
        .registers
        .get(puzzle.candidate)
        .copied()
        .ok_or(ParseError::Structure)?;
    Ok(Answers::new(answer, 0))
}
