// approach: Mark claimed square inches and count cells visited by at least two rectangles.
// cost: O(n + A) expected time, O(n + U) memory; one parse and one buffered claim pass.
// trade: Sparse coverage avoids a fixed fabric bound; hashing costs more than a dense grid.

//! Count square inches covered by multiple validated rectangular fabric claims.

use std::collections::HashMap;

use aoc_core::Answers;

use super::{ParseError, parse};

/// Solves part one, rendering the unfinished second part as `0`.
///
/// # Errors
///
/// Returns [`ParseError`] when the input does not match the claim grammar.
pub fn solve(input: &str) -> Result<Answers, ParseError> {
    let puzzle = parse(input)?;
    let mut coverage = HashMap::new();
    for claim in &puzzle.claims {
        for y in claim.y.clone() {
            for x in claim.x.clone() {
                coverage
                    .entry((x, y))
                    .and_modify(|overlap| *overlap = true)
                    .or_insert(false);
            }
        }
    }
    let overlapping = coverage.values().filter(|&&overlap| overlap).count();
    Ok(Answers::new(overlapping, 0))
}

#[cfg(test)]
mod tests {
    use aoc_core::Answers;

    use super::solve;

    #[test]
    fn repeated_coverage_counts_each_cell_once() {
        assert_eq!(
            solve("#8 @ 0,0: 3x2\n#2 @ 1,0: 2x2\n#7 @ 2,1: 1x1").unwrap(),
            Answers::new(4, 0),
            "triple coverage is not counted twice"
        );
    }

    #[test]
    fn touching_edges_are_not_overlap() {
        assert_eq!(
            solve("#4 @ 0,0: 2x3\n#5 @ 2,0: 1x3\n#6 @ 0,3: 2x1").unwrap(),
            Answers::new(0, 0),
            "half-open rectangles exclude touching edges"
        );
    }

    #[test]
    fn single_claim_and_large_offsets() {
        assert_eq!(
            solve("#9 @ 4294967294,10000: 1x2").unwrap(),
            Answers::new(0, 0),
            "no fixed fabric bound"
        );
    }
}
