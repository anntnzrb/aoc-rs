// approach: Flood-fill components using edges between points at Manhattan distance at most three.
// cost: O(n^2) time, O(n) memory, one input pass followed by component traversal.
// trade: Scanning remaining points avoids explicit edges and disjoint-set indexing.

//! Counts constellations by exhausting each transitive chain of nearby fixed points.

use aoc_core::Answers;

use super::{ParseError, parse};

/// Failure to parse observations or represent their constellation count.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// Invalid fixed-point observations.
    #[error("invalid fixed points")]
    Parse(#[from] ParseError),
    /// The constellation count exceeds the platform's integer capacity.
    #[error("constellation count overflow")]
    CountOverflow,
}

/// Counts constellations; the unfinished second part renders as `0`.
///
/// # Errors
/// Returns [`SolveError`] for invalid observations or an unrepresentable count.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let mut remaining = parse(input)?.points;
    let mut pending = Vec::with_capacity(remaining.len());
    let mut count = 0_usize;
    while let Some(seed) = remaining.pop() {
        count = count.checked_add(1).ok_or(SolveError::CountOverflow)?;
        pending.push(seed);
        while let Some(point) = pending.pop() {
            pending.extend(remaining.extract_if(.., |candidate| point.is_near(*candidate)));
        }
    }
    Ok(Answers::new(count, 0))
}

#[cfg(test)]
mod tests {
    use super::solve;

    #[test]
    fn bridges_join_components_regardless_of_input_order() {
        for input in ["0,0,0,0\n6,0,0,0\n3,0,0,0\n", "3,0,0,0\n0,0,0,0\n6,0,0,0\n"] {
            assert_eq!(
                solve(input).unwrap().part1(),
                "1",
                "transitive bridge {input}"
            );
        }
    }

    #[test]
    fn distance_budget_is_shared_by_all_dimensions() {
        assert_eq!(
            solve("0,0,0,0\n1,1,1,0\n-1,-1,-1,-1\n").unwrap().part1(),
            "2",
            "distance three joins, distance four separates"
        );
    }

    #[test]
    fn duplicates_join_and_extreme_coordinates_do_not_overflow() {
        assert_eq!(
            solve("-2147483648,0,0,0\n2147483647,0,0,0\n-2147483648,0,0,0\n")
                .unwrap()
                .part1(),
            "2",
            "duplicates and full signed range"
        );
        assert_eq!(
            solve("7,-8,9,-10\n").unwrap().part1(),
            "1",
            "singleton constellation"
        );
    }
}
