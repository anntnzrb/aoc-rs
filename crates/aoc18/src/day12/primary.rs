// approach: evolve the finite set of plant positions using five-pot neighborhoods.
// cost: O(g w log n) time, O(n) memory, one input pass; g generations, width w, n plants.
// trade: ordered sparse positions support negative pots and avoid artificial boundaries.

//! Finite plant propagation with simultaneous generation updates.

use std::collections::BTreeSet;

use aoc_core::Answers;

use super::{ParseError, Puzzle, parse};

/// Parsing or coordinate arithmetic failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// The input was malformed.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// A coordinate or answer exceeded its representation.
    #[error("plant coordinate or sum overflow")]
    Overflow,
}

fn step(plants: &BTreeSet<i64>, growing: &BTreeSet<u8>) -> Result<BTreeSet<i64>, SolveError> {
    let mut next = BTreeSet::new();
    if let (Some(first), Some(last)) = (plants.first(), plants.last()) {
        let start = first.checked_sub(2).ok_or(SolveError::Overflow)?;
        let end = last.checked_add(2).ok_or(SolveError::Overflow)?;
        for center in start..=end {
            let mut mask = 0_u8;
            for offset in -2..=2 {
                let position = center.checked_add(offset).ok_or(SolveError::Overflow)?;
                mask = (mask << 1) | u8::from(plants.contains(&position));
            }
            if growing.contains(&mask) {
                next.insert(center);
            }
        }
    }
    Ok(next)
}

fn sum_after(puzzle: &Puzzle, generations: u64) -> Result<i64, SolveError> {
    let mut plants = puzzle.plants.clone();
    for _ in 0..generations {
        plants = step(&plants, &puzzle.growing)?;
    }
    plants.iter().try_fold(0_i64, |sum, position| {
        sum.checked_add(*position).ok_or(SolveError::Overflow)
    })
}

/// Computes the sum after twenty generations; unfinished part two renders zero.
///
/// # Errors
/// Returns an error for invalid input or coordinate and sum overflow.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let puzzle = parse(input)?;
    Ok(Answers::new(sum_after(&puzzle, 20)?, 0))
}

#[cfg(test)]
mod tests {
    use super::{parse, sum_after};

    #[test]
    fn isolated_plant_moves_left() {
        let puzzle = parse("initial state: ..#\n...#. => #").unwrap();
        assert_eq!(sum_after(&puzzle, 0).unwrap(), 2, "initial coordinate");
        assert_eq!(
            sum_after(&puzzle, 20).unwrap(),
            -18,
            "negative positions remain meaningful"
        );
    }

    #[test]
    fn missing_rules_and_empty_states_die_out() {
        for input in ["initial state: #", "initial state: ...\n..#.. => #"] {
            let puzzle = parse(input).unwrap();
            assert_eq!(
                sum_after(&puzzle, 20).unwrap(),
                0,
                "empty population stays empty"
            );
        }
    }

    #[test]
    fn updates_are_simultaneous() {
        let puzzle = parse("initial state: #\n..#.. => #\n.#... => #\n...#. => #").unwrap();
        assert_eq!(
            sum_after(&puzzle, 1).unwrap(),
            0,
            "symmetric first generation"
        );
        assert_eq!(
            sum_after(&puzzle, 2).unwrap(),
            0,
            "symmetry survives simultaneous updates"
        );
    }
}
