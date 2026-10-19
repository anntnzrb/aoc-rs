// approach: evolve five-pot neighborhoods and fast-forward exact translated shape cycles.
// cost: O(g w log n + g n log g) time, O(g n) memory, one input pass before a cycle.
// trade: complete shape equality proves skips; no assumed stabilization or fixed horizon.

//! Finite plant propagation with simultaneous generation updates.

use std::collections::{BTreeMap, BTreeSet};

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

fn sum_after(puzzle: &Puzzle, generations: i64) -> Result<i64, SolveError> {
    let mut plants = puzzle.plants.clone();
    let mut seen = BTreeMap::new();
    let mut generation = 0_i64;
    while generation < generations {
        let origin = plants.first().copied().unwrap_or(0);
        let shape = plants
            .iter()
            .map(|position| position.checked_sub(origin).ok_or(SolveError::Overflow))
            .collect::<Result<Vec<_>, _>>()?;
        if let Some((previous_generation, previous_origin)) =
            seen.insert(shape, (generation, origin))
        {
            let period = generation
                .checked_sub(previous_generation)
                .ok_or(SolveError::Overflow)?;
            let remaining = generations
                .checked_sub(generation)
                .ok_or(SolveError::Overflow)?;
            let cycles = remaining.checked_div(period).ok_or(SolveError::Overflow)?;
            let shift = origin
                .checked_sub(previous_origin)
                .and_then(|delta| delta.checked_mul(cycles))
                .ok_or(SolveError::Overflow)?;
            if cycles > 0 {
                plants = plants
                    .into_iter()
                    .map(|position| position.checked_add(shift).ok_or(SolveError::Overflow))
                    .collect::<Result<BTreeSet<_>, _>>()?;
                generation = period
                    .checked_mul(cycles)
                    .and_then(|elapsed| generation.checked_add(elapsed))
                    .ok_or(SolveError::Overflow)?;
                seen.clear();
                continue;
            }
        }
        plants = step(&plants, &puzzle.growing)?;
        generation = generation.checked_add(1).ok_or(SolveError::Overflow)?;
    }
    plants.iter().try_fold(0_i64, |sum, position| {
        sum.checked_add(*position).ok_or(SolveError::Overflow)
    })
}

/// Computes sums after twenty and fifty billion generations.
///
/// # Errors
/// Returns an error for invalid input or coordinate and sum overflow.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let puzzle = parse(input)?;
    Ok(Answers::new(
        sum_after(&puzzle, 20)?,
        sum_after(&puzzle, 50_000_000_000)?,
    ))
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
        assert_eq!(
            sum_after(&puzzle, 50_000_000_000).unwrap(),
            -49_999_999_998,
            "exact translated cycle skip"
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
        assert_eq!(
            sum_after(&puzzle, 50_000_000_000).unwrap(),
            0,
            "large symmetric evolution"
        );
    }

    #[test]
    fn skips_period_two_with_a_remainder() {
        let puzzle = parse("initial state: ..#\n..#.. => #\n.#... => #\n..##. => #").unwrap();
        assert_eq!(sum_after(&puzzle, 1).unwrap(), 5, "one plant becomes two");
        assert_eq!(sum_after(&puzzle, 2).unwrap(), 2, "two plants become one");
        assert_eq!(
            sum_after(&puzzle, 50_000_000_001).unwrap(),
            5,
            "skip leaves an odd remainder"
        );
    }
}
