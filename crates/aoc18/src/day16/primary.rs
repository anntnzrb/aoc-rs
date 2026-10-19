// approach: execute every named operation against each observed state transition.
// cost: O(samples * 16 + program) time, O(samples + program) memory, one input pass.
// trade: explicit instruction semantics keep inference independent of numeric opcode names.

//! Counts observations compatible with at least three possible instruction meanings.

use aoc_core::Answers;

use super::{OPS, ParseError, Sample, parse};

fn matches(sample: &Sample) -> usize {
    let [_, a, b, c] = sample.instruction;
    OPS.iter()
        .filter(|op| {
            let mut registers = sample.before;
            match op.execute(&mut registers, [a, b, c]) {
                Ok(()) => registers == sample.after,
                Err(
                    ParseError::Register(_)
                    | ParseError::Index(_)
                    | ParseError::Overflow
                    | ParseError::Empty
                    | ParseError::Syntax(_)
                    | ParseError::Number(_),
                ) => false,
            }
        })
        .count()
}

/// Counts ambiguous observations; the unfinished second part renders zero.
///
/// # Errors
/// Returns an error if observations or instructions are malformed.
pub fn solve(input: &str) -> Result<Answers, ParseError> {
    let puzzle = parse(input)?;
    let part1 = puzzle
        .samples
        .iter()
        .filter(|sample| matches(sample) >= 3)
        .count();
    Ok(Answers::new(part1, 0))
}

#[cfg(test)]
mod tests {
    use super::matches;
    use crate::day16::Sample;

    #[test]
    fn unchanged_other_registers_are_required() {
        let sample = Sample {
            before: [0, 0, 0, 0],
            instruction: [0, 0, 0, 0],
            after: [0, 1, 0, 0],
        };
        assert_eq!(
            matches(&sample),
            0,
            "an instruction writes exactly one register"
        );
    }
}
