// approach: Count square-inch coverage in the claims' bounding box using a flat row-major grid.
// cost: O(n + A + B) time, O(n + B) space; B bounding-box bytes, A summed area; parse + 3 claim passes.
// trade: Measured 14.1 M work / 3.1 ms / 3108 KiB vs primary 156.0 M / 15.9 ms / 12412 KiB; sparse boxes waste B bytes.

//! Count overlaps with dense coverage counters without assuming a fixed fabric size.

use std::{collections::TryReserveError, num::TryFromIntError};

use aoc_core::Answers;

use super::{Claim, ParseError, Puzzle, parse};

/// A claim parse failure or a dense grid that cannot be represented or allocated.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// A claim does not match the puzzle grammar.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// A coordinate does not fit the platform's address space.
    #[error("grid coordinate exceeds the platform address space")]
    Coordinate(#[from] TryFromIntError),
    /// The bounding-box size or row-major offset is not representable.
    #[error("grid size or offset exceeds the platform address space")]
    Overflow,
    /// The allocator cannot reserve the bounding box.
    #[error("cannot allocate the bounding-box grid")]
    Allocation(#[from] TryReserveError),
    /// Validated claims unexpectedly fall outside their bounding box.
    #[error("claim lies outside its computed bounding box")]
    Bounds,
}

struct Grid {
    left: u32,
    top: u32,
    width: usize,
    cells: Vec<u8>,
    overlapping: usize,
}

impl Grid {
    fn new(puzzle: &Puzzle) -> Result<Self, SolveError> {
        let first = puzzle.claims.first().ok_or(SolveError::Bounds)?;
        let (mut left, mut top, mut right, mut bottom) =
            (first.x.start, first.y.start, first.x.end, first.y.end);
        for claim in &puzzle.claims {
            left = left.min(claim.x.start);
            top = top.min(claim.y.start);
            right = right.max(claim.x.end);
            bottom = bottom.max(claim.y.end);
        }
        let width = usize::try_from(right.checked_sub(left).ok_or(SolveError::Bounds)?)?;
        let height = usize::try_from(bottom.checked_sub(top).ok_or(SolveError::Bounds)?)?;
        let area = width.checked_mul(height).ok_or(SolveError::Overflow)?;
        // The statement gives no maximum fabric size, so an arbitrary area cap
        // would reject valid claims. Reserve fallibly; sparse boxes still cost B bytes.
        let mut cells = Vec::new();
        cells.try_reserve_exact(area)?;
        cells.resize(area, 0);
        Ok(Self {
            left,
            top,
            width,
            cells,
            overlapping: 0,
        })
    }

    fn offset(&self, x: u32, y: u32) -> Result<usize, SolveError> {
        let x = usize::try_from(x.checked_sub(self.left).ok_or(SolveError::Bounds)?)?;
        let y = usize::try_from(y.checked_sub(self.top).ok_or(SolveError::Bounds)?)?;
        y.checked_mul(self.width)
            .and_then(|row| row.checked_add(x))
            .ok_or(SolveError::Overflow)
    }

    fn mark(&mut self, claim: &Claim) -> Result<(), SolveError> {
        for y in claim.y.clone() {
            for x in claim.x.clone() {
                let offset = self.offset(x, y)?;
                let count = self.cells.get_mut(offset).ok_or(SolveError::Bounds)?;
                if *count == 1 {
                    self.overlapping = self
                        .overlapping
                        .checked_add(1)
                        .ok_or(SolveError::Overflow)?;
                }
                // Only zero, one and multiple coverage matter, even for triple overlaps.
                *count = count.saturating_add(1).min(2);
            }
        }
        Ok(())
    }

    fn intact(&self, claim: &Claim) -> Result<bool, SolveError> {
        for y in claim.y.clone() {
            for x in claim.x.clone() {
                let offset = self.offset(x, y)?;
                if *self.cells.get(offset).ok_or(SolveError::Bounds)? != 1 {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}

/// Solves both parts, omitting part two unless exactly one intact claim exists.
///
/// # Errors
///
/// Returns [`SolveError`] for malformed claims, unrepresentable grid dimensions,
/// allocation failure or an inconsistent bounding-box offset.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let puzzle = parse(input)?;
    let mut grid = Grid::new(&puzzle)?;
    for claim in &puzzle.claims {
        grid.mark(claim)?;
    }
    let overlapping = grid.overlapping;
    let mut intact = None;
    for claim in &puzzle.claims {
        if grid.intact(claim)? {
            if intact.is_some() {
                return Ok(Answers::part1_only(overlapping));
            }
            intact = Some(claim.id);
        }
    }
    Ok(intact.map_or_else(
        || Answers::part1_only(overlapping),
        |id| Answers::new(overlapping, id),
    ))
}

#[cfg(test)]
mod tests {
    use aoc_core::Answers;

    use super::solve;

    #[test]
    fn bounding_box_has_nonzero_origin() {
        assert_eq!(
            solve("#42 @ 2000,3000: 2x2\n#7 @ 2001,3001: 1x1\n#9 @ 2003,3002: 1x1").unwrap(),
            Answers::new(1, 9),
            "row-major offsets subtract both bounding-box origins"
        );
    }

    #[test]
    fn touching_edges_do_not_overlap() {
        assert_eq!(
            solve("#4 @ 12,13: 2x3\n#5 @ 14,13: 1x3\n#6 @ 12,16: 2x1").unwrap(),
            Answers::part1_only(0),
            "half-open rectangles exclude shared edges on both axes"
        );
    }

    #[test]
    fn triple_overlap_counts_once() {
        assert_eq!(
            solve("#8 @ 20,30: 3x2\n#2 @ 21,30: 2x2\n#7 @ 22,31: 1x1").unwrap(),
            Answers::part1_only(4),
            "saturating counters count multiply covered cells, not pairs"
        );
    }

    #[test]
    fn single_claim_near_coordinate_limit() {
        assert_eq!(
            solve("#9 @ 4294967294,4294967293: 1x2").unwrap(),
            Answers::new(0, 9),
            "large coordinates need only the tight two-byte bounding box"
        );
    }

    #[test]
    fn unrepresentable_allocation_returns_a_typed_error() {
        assert!(
            matches!(
                solve("#1 @ 0,0: 1x1\n#2 @ 4294967294,4294967294: 1x1"),
                Err(super::SolveError::Allocation(_) | super::SolveError::Overflow)
            ),
            "a sparse box larger than the address space must not allocate or panic"
        );
    }
}
