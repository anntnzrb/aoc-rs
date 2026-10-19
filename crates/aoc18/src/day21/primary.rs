// approach: observe isolated halt comparisons until full machine state repeats; accelerate checked division loops.
// cost: O(executed non-division instructions) time, O(program + comparison states) memory, one pass.
// trade: exact loop matching preserves device semantics; full-state cycles avoid assuming candidate-only recurrence.

//! Selects first and last new halting candidates from a deterministic register-machine cycle.

use super::{ParseError, Puzzle, parse};
use crate::device::{DeviceError, Machine, Mode, Op, Program};
use Mode::{Immediate as I, Register as R};
use aoc_core::Answers;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy)]
struct Division {
    start: u64,
    quotient: usize,
    temporary: usize,
    source: usize,
    divisor: u64,
}

fn division(program: &Program, start: usize) -> Option<Division> {
    let block = program.instructions.get(start..)?.get(..9)?;
    let [initialize, increment, multiply, compare, ..] = block else {
        return None;
    };
    let [0, _, quotient] = initialize.args else {
        return None;
    };
    let [_, 1, temporary] = increment.args else {
        return None;
    };
    let [_, divisor, _] = multiply.args;
    let [_, source, _] = compare.args;
    let Ok(ip) = u64::try_from(program.binding) else {
        return None;
    };
    let Ok(start) = u64::try_from(start) else {
        return None;
    };
    let roles = [quotient, temporary, source, ip];
    if divisor < 2
        || roles.contains(&0)
        || roles.iter().any(|&r| r >= 6)
        || roles.iter().enumerate().any(|(i, r)| {
            roles
                .iter()
                .skip(i.saturating_add(1))
                .any(|other| r == other)
        })
    {
        return None;
    }
    let expected = [
        (Op::Set(I), [0, 0, quotient]),
        (Op::Add(I), [quotient, 1, temporary]),
        (Op::Mul(I), [temporary, divisor, temporary]),
        (Op::Greater(R, R), [temporary, source, temporary]),
        (Op::Add(R), [temporary, ip, ip]),
        (Op::Add(I), [ip, 1, ip]),
        (Op::Set(I), [start.checked_add(8)?, 0, ip]),
        (Op::Add(I), [quotient, 1, quotient]),
        (Op::Set(I), [start, 0, ip]),
    ];
    let valid = block.iter().zip(expected).all(|(actual, (op, args))| {
        let [a, b, c] = actual.args;
        let [ea, eb, ec] = args;
        actual.op == op && a == ea && c == ec && (matches!(op, Op::Set(I)) || b == eb)
    });
    if !valid {
        return None;
    }
    let (Ok(quotient), Ok(temporary), Ok(source)) = (
        usize::try_from(quotient),
        usize::try_from(temporary),
        usize::try_from(source),
    ) else {
        return None;
    };
    Some(Division {
        start,
        quotient,
        temporary,
        source,
        divisor,
    })
}

impl Division {
    fn apply(self, machine: &mut Machine, binding: usize) -> Result<(), ParseError> {
        let source = machine
            .registers
            .get(self.source)
            .copied()
            .ok_or(ParseError::Structure)?;
        let quotient = source
            .checked_div(self.divisor)
            .ok_or(DeviceError::Overflow)?;
        // The original loop also computes the first product strictly greater than the source.
        quotient
            .checked_add(1)
            .and_then(|q| q.checked_mul(self.divisor))
            .ok_or(DeviceError::Overflow)?;
        *machine
            .registers
            .get_mut(self.quotient)
            .ok_or(ParseError::Structure)? = quotient;
        *machine
            .registers
            .get_mut(self.temporary)
            .ok_or(ParseError::Structure)? = 1;
        let bound = self.start.checked_add(8).ok_or(DeviceError::Overflow)?;
        *machine
            .registers
            .get_mut(binding)
            .ok_or(ParseError::Structure)? = bound;
        machine.ip = bound.checked_add(1).ok_or(DeviceError::Overflow)?;
        Ok(())
    }
}

fn candidates(puzzle: &Puzzle) -> Result<(u64, u64), ParseError> {
    let divisions: Vec<_> = (0..puzzle.program.instructions.len())
        .filter_map(|i| division(&puzzle.program, i))
        .collect();
    let mut machine = Machine::default();
    let mut states = HashSet::new();
    let mut values = HashSet::new();
    let mut first = None;
    let mut last = None;
    loop {
        if machine.ip == puzzle.comparison {
            if !states.insert(machine.registers) {
                break;
            }
            let value = machine
                .registers
                .get(puzzle.candidate)
                .copied()
                .ok_or(ParseError::Structure)?;
            if values.insert(value) {
                first.get_or_insert(value);
                last = Some(value);
            }
            let instruction = puzzle
                .program
                .instructions
                .get(usize::try_from(puzzle.comparison)?)
                .ok_or(ParseError::Structure)?;
            let [_, _, flag] = instruction.args;
            if !puzzle.program.step(&mut machine)? {
                return Err(ParseError::Halted);
            }
            *machine
                .registers
                .get_mut(usize::try_from(flag)?)
                .ok_or(ParseError::Structure)? = 0;
        } else if let Some(divide) = divisions.iter().find(|d| d.start == machine.ip) {
            divide.apply(&mut machine, puzzle.program.binding)?;
        } else if !puzzle.program.step(&mut machine)? {
            return Err(ParseError::Halted);
        }
    }
    Ok((
        first.ok_or(ParseError::Halted)?,
        last.ok_or(ParseError::Halted)?,
    ))
}

/// Returns the earliest and latest first-occurring halting register-zero values.
///
/// # Errors
/// Returns an error for unsupported control flow, malformed programs or overflow.
pub fn solve(input: &str) -> Result<Answers, ParseError> {
    let puzzle = parse(input)?;
    let (first, last) = candidates(&puzzle)?;
    Ok(Answers::new(first, last))
}

#[cfg(test)]
mod tests {
    use super::{division, solve};
    use crate::device::{Machine, Program};

    #[test]
    fn cycle_uses_first_occurrence_not_last_duplicate() {
        let answer =
            solve("#ip 5\naddi 1 1 1\nbani 1 3 1\neqrr 1 0 2\naddr 2 5 5\nseti 0 0 5").unwrap();
        assert_eq!(
            answer,
            aoc_core::Answers::new(1, 1),
            "restart preserves candidate and repeats the same halt value"
        );
    }

    #[test]
    fn division_acceleration_matches_all_device_registers() {
        let program = Program::parse("#ip 4\nseti 0 0 1\naddi 1 1 5\nmuli 5 7 5\ngtrr 5 2 5\naddr 5 4 4\naddi 4 1 4\nseti 8 0 4\naddi 1 1 1\nseti 0 0 4").unwrap();
        let divide = division(&program, 0).unwrap();
        for source in 0..40 {
            let mut slow = Machine {
                registers: [0, 0, source, 0, 0, 0],
                ip: 0,
            };
            let mut fast = slow;
            divide.apply(&mut fast, program.binding).unwrap();
            while program.step(&mut slow).unwrap() {}
            assert_eq!(fast, slow, "source {source}");
        }
    }
}
