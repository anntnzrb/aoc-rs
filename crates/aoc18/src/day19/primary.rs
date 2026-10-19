// approach: interpret the program with its instruction pointer bound to the declared register.
// cost: O(executed instructions) time, O(program length) memory, one input pass.
// trade: direct simulation preserves all control flow without assumptions about the program.

//! Executes the background process to completion with initially zeroed registers.

use super::parse;
use crate::device::{DeviceError, Machine};
use aoc_core::Answers;

/// Returns register zero at termination; part two is not yet implemented.
///
/// # Errors
/// Returns an error for malformed programs or arithmetic overflow.
pub fn solve(input: &str) -> Result<Answers, DeviceError> {
    let program = parse(input)?;
    let mut machine = Machine::default();
    while program.step(&mut machine)? {}
    let [answer, ..] = machine.registers;
    Ok(Answers::new(answer, 0))
}
