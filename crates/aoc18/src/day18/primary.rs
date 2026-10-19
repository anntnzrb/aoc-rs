// approach: Simulate simultaneous acre rules and skip only exactly repeated full states.
// cost: O((transient + period) * acres) expected time and memory, one input pass.
// trade: Exact state keys avoid false cycles; search stops at 100000 states or 64 MiB of acres.

//! Simultaneous landscape evolution and resource valuation.

use aoc_core::Answers;
use std::collections::HashMap;

use super::{Acre, ParseError, Puzzle, parse};

/// Parsing or resource-value computation failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// The initial area is invalid.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// The resource value does not fit the machine word.
    #[error("resource value overflow")]
    Overflow,
    /// No exact repeated landscape was found within the resource budget.
    #[error("no repeated landscape within {0} stored states")]
    NoCycle(usize),
}

fn step(area: &mut Puzzle, next: &mut Puzzle) {
    let acres = &area.acres;
    for (y, (row, next_row)) in area.acres.iter().zip(&mut next.acres).enumerate() {
        for (x, (acre, output)) in row.iter().zip(next_row).enumerate() {
            let neighbors = (-1_isize..=1).flat_map(|dy| {
                (-1_isize..=1).filter_map(move |dx| {
                    if dx == 0 && dy == 0 {
                        return None;
                    }
                    let ny = y.checked_add_signed(dy)?;
                    let nx = x.checked_add_signed(dx)?;
                    acres.get(ny)?.get(nx)
                })
            });
            let (trees, yards) = neighbors.fold((0_u8, 0_u8), |(trees, yards), neighbor| {
                // At most eight neighbors; saturation cannot change the rule thresholds.
                match neighbor {
                    Acre::Open => (trees, yards),
                    Acre::Trees => (trees.saturating_add(1), yards),
                    Acre::Lumberyard => (trees, yards.saturating_add(1)),
                }
            });
            *output = match acre {
                Acre::Open if trees >= 3 => Acre::Trees,
                Acre::Trees if yards >= 3 => Acre::Lumberyard,
                Acre::Lumberyard if trees == 0 || yards == 0 => Acre::Open,
                Acre::Open | Acre::Trees | Acre::Lumberyard => *acre,
            };
        }
    }
    std::mem::swap(area, next);
}

fn resource(area: &Puzzle) -> Result<usize, SolveError> {
    let trees = area
        .acres
        .iter()
        .flatten()
        .filter(|&&a| a == Acre::Trees)
        .count();
    let yards = area
        .acres
        .iter()
        .flatten()
        .filter(|&&a| a == Acre::Lumberyard)
        .count();
    trees.checked_mul(yards).ok_or(SolveError::Overflow)
}

fn evolve(area: &mut Puzzle, minutes: usize, bound: usize) -> Result<(), SolveError> {
    // Full states must be independently owned while the working grid changes.
    let mut seen = HashMap::new();
    let mut next = area.clone();
    for minute in 0..=minutes {
        if minute == minutes {
            return Ok(());
        }
        if let Some(previous) = seen.get(&area.acres) {
            let period = minute.checked_sub(*previous).ok_or(SolveError::Overflow)?;
            let remaining = minutes.checked_sub(minute).ok_or(SolveError::Overflow)?;
            let remainder = remaining.checked_rem(period).ok_or(SolveError::Overflow)?;
            for _ in 0..remainder {
                step(area, &mut next);
            }
            return Ok(());
        }
        if seen.len() >= bound {
            return Err(SolveError::NoCycle(bound));
        }
        seen.insert(area.acres.clone(), minute);
        step(area, &mut next);
    }
    Err(SolveError::NoCycle(bound))
}

/// Computes resource values after ten and one billion minutes.
///
/// # Errors
/// Returns [`SolveError`] for malformed input, overflow, or cycle-budget exhaustion.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let mut area = parse(input)?;
    // The scratch grid needs independent storage for simultaneous updates.
    let mut next = area.clone();
    for _ in 0..10 {
        step(&mut area, &mut next);
    }
    let part1 = resource(&area)?;
    let cells = area.acres.iter().try_fold(0_usize, |sum, row| {
        sum.checked_add(row.len()).ok_or(SolveError::Overflow)
    })?;
    // This is a practical resource limit, not a claim that every area cycles quickly:
    // cap stored acre payloads at 64 MiB and evolution at 100000 states. Longer
    // transients or periods are explicitly rejected rather than extrapolated.
    let bound = 67_108_864_usize
        .checked_div(cells)
        .ok_or(SolveError::Overflow)?
        .min(100_000);
    evolve(&mut area, 999_999_990, bound)?;
    Ok(Answers::new(part1, resource(&area)?))
}

#[cfg(test)]
mod tests {
    use super::{SolveError, evolve, parse, resource, solve, step};

    #[test]
    fn isolated_acres_obey_boundary_rules() {
        for (input, expected) in [(".", "."), ("|", "|"), ("#", ".")] {
            let mut area = parse(input).unwrap();
            let mut next = area.clone();
            step(&mut area, &mut next);
            assert_eq!(area, parse(expected).unwrap(), "isolated {input}");
        }
    }

    #[test]
    fn updates_are_simultaneous_and_diagonal() {
        let mut area = parse("|||\n|.#\n###").unwrap();
        let mut next = area.clone();
        step(&mut area, &mut next);
        assert_eq!(
            area,
            parse("|||\n||#\n##.").unwrap(),
            "one simultaneous step"
        );
        assert_eq!(resource(&area).unwrap(), 15, "resource product");
    }

    #[test]
    fn stable_area_has_zero_value() {
        assert_eq!(
            solve(".").unwrap(),
            aoc_core::Answers::new(0, 0),
            "open area"
        );
    }

    #[test]
    fn cycle_skipping_matches_direct_evolution() {
        let initial = parse("|||\n|.#\n###").unwrap();
        let mut direct = initial.clone();
        let mut next = direct.clone();
        for minutes in 0..80 {
            let mut skipped = initial.clone();
            evolve(&mut skipped, minutes, 100).unwrap();
            assert_eq!(skipped, direct, "minute {minutes}");
            step(&mut direct, &mut next);
        }
    }

    #[test]
    fn cycle_budget_is_enforced_without_guessing() {
        let mut area = parse("#").unwrap();
        assert!(
            matches!(evolve(&mut area, 100, 1), Err(SolveError::NoCycle(1))),
            "transient exceeds budget"
        );
        evolve(&mut area, 1_000_000_000, 1).unwrap();
        assert_eq!(area, parse(".").unwrap(), "fixed point skips safely");
    }
}
