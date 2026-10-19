// approach: Mark claimed square inches, then find the sole rectangle with no shared cell.
// cost: Expected O(n + A) time, O(n + U) space (A summed area, U union); parse + 2 claim passes.
// trade: Sparse coverage avoids a fixed fabric bound; hashing costs more than a dense grid.

//! Count multiply claimed square inches and identify the sole intact fabric claim.

use std::collections::HashMap;

use aoc_core::Answers;

use super::{ParseError, parse};

/// Solves both parts, omitting part two unless exactly one intact claim exists.
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
    let mut intact = puzzle.claims.iter().filter(|claim| {
        claim.y.clone().all(|y| {
            claim
                .x
                .clone()
                .all(|x| coverage.get(&(x, y)) == Some(&false))
        })
    });
    let first = intact.next();
    Ok(match (first, intact.next()) {
        (Some(claim), None) => Answers::new(overlapping, claim.id),
        _ => Answers::part1_only(overlapping),
    })
}

#[cfg(test)]
mod tests {
    use aoc_core::Answers;

    use super::solve;

    #[test]
    fn repeated_coverage_counts_each_cell_once() {
        assert_eq!(
            solve("#8 @ 0,0: 3x2\n#2 @ 1,0: 2x2\n#7 @ 2,1: 1x1").unwrap(),
            Answers::part1_only(4),
            "triple coverage is not counted twice"
        );
    }

    #[test]
    fn touching_edges_are_not_overlap() {
        assert_eq!(
            solve("#4 @ 0,0: 2x3\n#5 @ 2,0: 1x3\n#6 @ 0,3: 2x1").unwrap(),
            Answers::part1_only(0),
            "half-open rectangles exclude touching edges"
        );
    }

    #[test]
    fn single_claim_and_large_offsets() {
        assert_eq!(
            solve("#9 @ 4294967294,10000: 1x2").unwrap(),
            Answers::new(0, 9),
            "no fixed fabric bound"
        );
    }

    #[test]
    fn intact_claim_is_identified_by_id_not_position() {
        assert_eq!(
            solve("#42 @ 10,10: 2x3\n#7 @ 0,0: 2x2\n#8 @ 1,1: 1x1").unwrap(),
            Answers::new(1, 42),
            "a contained claim disqualifies both rectangles, leaving the first intact"
        );
    }
}
