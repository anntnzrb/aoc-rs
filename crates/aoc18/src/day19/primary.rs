// approach: interpret both initial states; replace validated nested divisor loops by divisor sums.
// cost: O(prefix + sqrt(target) + suffix) time with a recognized loop, O(program) memory, one pass.
// trade: accelerates only a fully checked 15-instruction loop with six distinct register roles.

//! Runs bound-register programs, recognizing an exact divisor-enumeration loop without fixed constants.

use super::parse;
use crate::device::{DeviceError, Machine, Mode, Op, Program};
use Mode::{Immediate as I, Register as R};
use aoc_core::Answers;

/// Failures in parsing, simulation or accelerated loop arithmetic.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// Device parsing or execution failed.
    #[error("device execution failed")]
    Device(#[from] DeviceError),
    /// The loop's final registers or divisor sum cannot be represented.
    #[error("divisor loop arithmetic overflow")]
    Overflow,
    /// A validated register role cannot be indexed on this platform.
    #[error("register index is too large")]
    Index(#[from] std::num::TryFromIntError),
}

#[derive(Debug, Clone, Copy)]
struct DivisorLoop {
    start: u64,
    outer: u64,
    inner: u64,
    temporary: u64,
    target: u64,
}

fn recognize(program: &Program, start: usize) -> Option<DivisorLoop> {
    let instructions = program.instructions.get(start..)?.get(..15)?;
    let [first, second, product, comparison, ..] = instructions else {
        return None;
    };
    let [1, _, outer] = first.args else {
        return None;
    };
    let [1, _, inner] = second.args else {
        return None;
    };
    let [_, _, temporary] = product.args;
    let [_, target, _] = comparison.args;
    let Ok(ip) = u64::try_from(program.binding) else {
        return None;
    };
    let Ok(start) = u64::try_from(start) else {
        return None;
    };
    let roles = [0, outer, inner, temporary, target, ip];
    if roles.iter().any(|&r| r >= 6)
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
        (Op::Set(I), [1, 0, outer]),
        (Op::Set(I), [1, 0, inner]),
        (Op::Mul(R), [outer, inner, temporary]),
        (Op::Equal(R, R), [temporary, target, temporary]),
        (Op::Add(R), [temporary, ip, ip]),
        (Op::Add(I), [ip, 1, ip]),
        (Op::Add(R), [outer, 0, 0]),
        (Op::Add(I), [inner, 1, inner]),
        (Op::Greater(R, R), [inner, target, temporary]),
        (Op::Add(R), [ip, temporary, ip]),
        (Op::Set(I), [start.checked_add(1)?, 0, ip]),
        (Op::Add(I), [outer, 1, outer]),
        (Op::Greater(R, R), [outer, target, temporary]),
        (Op::Add(R), [temporary, ip, ip]),
        (Op::Set(I), [start, 0, ip]),
    ];
    let valid = instructions
        .iter()
        .zip(expected)
        .all(|(actual, (op, args))| {
            actual.op == op
                && match op {
                    Op::Set(I) => {
                        let [a, _, c] = actual.args;
                        let [ea, _, ec] = args;
                        a == ea && c == ec
                    }
                    Op::Add(_)
                    | Op::Mul(_)
                    | Op::And(_)
                    | Op::Or(_)
                    | Op::Set(R)
                    | Op::Greater(_, _)
                    | Op::Equal(_, _) => actual.args == args,
                }
        });
    valid.then_some(DivisorLoop {
        start,
        outer,
        inner,
        temporary,
        target,
    })
}

fn sum_divisors(target: u64) -> Result<u64, SolveError> {
    let mut sum = 0_u64;
    let mut divisor = 1_u64;
    while divisor <= target.checked_div(divisor).ok_or(SolveError::Overflow)? {
        if target.is_multiple_of(divisor) {
            let quotient = target.checked_div(divisor).ok_or(SolveError::Overflow)?;
            sum = sum.checked_add(divisor).ok_or(SolveError::Overflow)?;
            if quotient != divisor {
                sum = sum.checked_add(quotient).ok_or(SolveError::Overflow)?;
            }
        }
        divisor = divisor.checked_add(1).ok_or(SolveError::Overflow)?;
    }
    Ok(sum)
}

impl DivisorLoop {
    fn apply(self, machine: &mut Machine, binding: usize) -> Result<(), SolveError> {
        let target = machine
            .registers
            .get(usize::try_from(self.target)?)
            .copied()
            .ok_or(DeviceError::Register(self.target))?;
        // With target zero the original loop still tests the pair (1,1), adding nothing.
        let end_counter = target.max(1).checked_add(1).ok_or(SolveError::Overflow)?;
        let [accumulator, ..] = machine.registers;
        for (register, value) in [
            (
                0,
                accumulator
                    .checked_add(sum_divisors(target)?)
                    .ok_or(SolveError::Overflow)?,
            ),
            (self.outer, end_counter),
            (self.inner, end_counter),
            (self.temporary, 1),
        ] {
            *machine
                .registers
                .get_mut(usize::try_from(register)?)
                .ok_or(DeviceError::Register(register))? = value;
        }
        let final_ip = self.start.checked_add(14).ok_or(SolveError::Overflow)?;
        *machine
            .registers
            .get_mut(binding)
            .ok_or(SolveError::Overflow)? = final_ip;
        machine.ip = final_ip.checked_add(1).ok_or(SolveError::Overflow)?;
        Ok(())
    }
}

fn run(program: &Program, initial: u64) -> Result<u64, SolveError> {
    let loops: Vec<_> = (0..program.instructions.len())
        .filter_map(|i| recognize(program, i))
        .collect();
    let mut machine = Machine {
        registers: [initial, 0, 0, 0, 0, 0],
        ip: 0,
    };
    loop {
        if let Some(loop_) = loops.iter().find(|loop_| loop_.start == machine.ip) {
            loop_.apply(&mut machine, program.binding)?;
        } else if !program.step(&mut machine)? {
            break;
        }
    }
    let [answer, ..] = machine.registers;
    Ok(answer)
}

/// Returns register zero at termination for initial values zero and one.
///
/// # Errors
/// Returns an error for malformed programs or arithmetic overflow.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let program = parse(input)?;
    Ok(Answers::new(run(&program, 0)?, run(&program, 1)?))
}

#[cfg(test)]
mod tests {
    use super::{recognize, run, sum_divisors};
    use crate::device::{Machine, Program};

    #[test]
    fn divisor_pairs_do_not_double_count_squares() {
        assert_eq!(sum_divisors(0).unwrap(), 0, "zero has no loop matches");
        assert_eq!(sum_divisors(1).unwrap(), 1, "unit square");
        assert_eq!(sum_divisors(36).unwrap(), 91, "square counted once");
        assert_eq!(sum_divisors(13).unwrap(), 14, "prime");
    }

    #[test]
    fn accelerated_loop_matches_device_states() {
        for target in 0..12 {
            let source = format!(
                "#ip 5\nseti {target} 0 2\nseti 1 0 1\nseti 1 0 4\nmulr 1 4 3\neqrr 3 2 3\naddr 3 5 5\naddi 5 1 5\naddr 1 0 0\naddi 4 1 4\ngtrr 4 2 3\naddr 5 3 5\nseti 2 0 5\naddi 1 1 1\ngtrr 1 2 3\naddr 3 5 5\nseti 1 0 5"
            );
            let program = Program::parse(&source).unwrap();
            let loop_ = recognize(&program, 1).unwrap();
            let mut slow = Machine::default();
            assert!(program.step(&mut slow).unwrap(), "initialization");
            let mut fast = slow;
            loop_.apply(&mut fast, program.binding).unwrap();
            while program.step(&mut slow).unwrap() {}
            assert_eq!(fast, slow, "all registers and pointer agree for {target}");
            assert_eq!(
                run(&program, 0).unwrap(),
                slow.registers[0],
                "accelerated run"
            );
        }
    }
}
