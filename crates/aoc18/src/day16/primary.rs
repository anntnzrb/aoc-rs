// approach: intersect observed opcode meanings, solve the bijection, then execute the program.
// cost: O(samples * 16 + search + program) time, O(samples + program) memory, one input pass.
// trade: bounded backtracking handles constraints beyond singleton elimination; worst case 16!.

//! Counts observations compatible with at least three possible instruction meanings.

use aoc_core::Answers;

use super::{ParseError, Puzzle, Sample, parse};
use crate::device::{DeviceError as Error, OPS, Op};

fn compatible(sample: &Sample, op: Op) -> bool {
    let [_, a, b, c] = sample.instruction;
    let mut registers = sample.before;
    match op.execute(&mut registers, [a, b, c]) {
        Ok(()) => registers == sample.after,
        Err(
            Error::Register(_)
            | Error::Index(_)
            | Error::Overflow
            | Error::Syntax(_)
            | Error::Number(_),
        ) => false,
    }
}

fn matches(sample: &Sample) -> usize {
    OPS.iter().filter(|&&op| compatible(sample, op)).count()
}

fn search(
    candidates: &[Vec<Op>],
    assigned: &mut [Option<Op>; 16],
    solutions: &mut Vec<[Option<Op>; 16]>,
) {
    if solutions.len() >= 2 {
        return;
    }
    let next = candidates
        .iter()
        .enumerate()
        .filter(|(i, _)| assigned.get(*i) == Some(&None))
        .min_by_key(|(_, ops)| {
            ops.iter()
                .filter(|op| !assigned.contains(&Some(**op)))
                .count()
        });
    let Some((index, options)) = next else {
        solutions.push(*assigned);
        return;
    };
    for &op in options {
        if assigned.contains(&Some(op)) {
            continue;
        }
        if let Some(slot) = assigned.get_mut(index) {
            *slot = Some(op);
        }
        search(candidates, assigned, solutions);
        if let Some(slot) = assigned.get_mut(index) {
            *slot = None;
        }
        if solutions.len() >= 2 {
            return;
        }
    }
}

fn execute(puzzle: &Puzzle) -> Result<u64, ParseError> {
    let mut candidates = vec![OPS.to_vec(); 16];
    for sample in &puzzle.samples {
        let [number, _, _, _] = sample.instruction;
        candidates
            .get_mut(usize::try_from(number)?)
            .ok_or(ParseError::Mapping)?
            .retain(|&op| compatible(sample, op));
    }
    let mut solutions = Vec::new();
    search(&candidates, &mut [None; 16], &mut solutions);
    let [assignment] = solutions.as_slice() else {
        return Err(ParseError::Mapping);
    };
    let mut registers = [0; 4];
    for &[number, a, b, c] in &puzzle.program {
        assignment
            .get(usize::try_from(number)?)
            .copied()
            .flatten()
            .ok_or(ParseError::Mapping)?
            .execute(&mut registers, [a, b, c])?;
    }
    let [answer, _, _, _] = registers;
    Ok(answer)
}

/// Counts ambiguous samples and runs the uniquely decoded program when present.
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
    if puzzle.program.is_empty() {
        return Ok(Answers::part1_only(part1));
    }
    Ok(Answers::new(part1, execute(&puzzle)?))
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

    #[test]
    fn bijection_search_detects_unique_ambiguous_and_impossible() {
        let candidates: Vec<_> = crate::device::OPS.iter().map(|&op| vec![op]).collect();
        let mut solutions = Vec::new();
        super::search(&candidates, &mut [None; 16], &mut solutions);
        assert_eq!(solutions.len(), 1, "one permitted bijection");
        let mut ambiguous = candidates.clone();
        ambiguous[0].push(crate::device::OPS[1]);
        ambiguous[1].push(crate::device::OPS[0]);
        solutions.clear();
        super::search(&ambiguous, &mut [None; 16], &mut solutions);
        assert_eq!(solutions.len(), 2, "two interchangeable meanings");
        let mut impossible = candidates;
        impossible[0].clear();
        solutions.clear();
        super::search(&impossible, &mut [None; 16], &mut solutions);
        assert!(solutions.is_empty(), "no permitted assignment");
    }
}
