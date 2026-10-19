// approach: Apply the simultaneous acre rules using separate current and next grids.
// cost: O(10 * acres) time, O(acres) memory, one input pass.
// trade: Explicit three-state acres and boundary-aware neighbors keep edge rules local.

//! Simultaneous landscape evolution and resource valuation.

use aoc_core::Answers;

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

/// Computes the resource value at ten minutes; unfinished part two renders zero.
///
/// # Errors
/// Returns [`SolveError`] for malformed input or resource-value overflow.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let mut area = parse(input)?;
    // The scratch grid needs independent storage for simultaneous updates.
    let mut next = area.clone();
    for _ in 0..10 {
        step(&mut area, &mut next);
    }
    Ok(Answers::new(resource(&area)?, 0))
}

#[cfg(test)]
mod tests {
    use super::{parse, resource, solve, step};

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
}
