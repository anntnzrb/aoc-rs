// approach: generate erosion levels in dependency order and sum the target rectangle's risk
// cost: O(width * height) time and memory, one input pass
// trade: retain the erosion grid so later region queries can reuse its dependencies

//! Cave erosion and risk calculation for Advent of Code 2018, day 22.

use aoc_core::Answers;

use super::{ParseError, Puzzle, parse};

const EROSION_MODULUS: u64 = 20_183;

/// Failure to parse or calculate a cave survey.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// The survey is malformed.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// A coordinate, erosion calculation, or answer exceeds its integer representation.
    #[error("cave calculation exceeds integer range")]
    Overflow,
    /// A required predecessor is missing from the erosion grid.
    #[error("missing erosion predecessor")]
    Grid,
    /// A coordinate cannot be represented in the erosion calculation.
    #[error("coordinate exceeds erosion integer range")]
    Coordinate(#[from] std::num::TryFromIntError),
}

struct Cave {
    puzzle: Puzzle,
    erosion: Vec<Vec<u64>>,
    width: usize,
}

impl Cave {
    const fn new(puzzle: Puzzle) -> Self {
        Self {
            puzzle,
            erosion: Vec::new(),
            width: 0,
        }
    }

    fn get(&self, x: usize, y: usize) -> Result<u64, SolveError> {
        self.erosion
            .get(y)
            .and_then(|row| row.get(x))
            .copied()
            .ok_or(SolveError::Grid)
    }

    fn extend(&mut self, x: usize, y: usize) -> Result<(), SolveError> {
        self.width = self
            .width
            .max(x.checked_add(1).ok_or(SolveError::Overflow)?);
        let height = self
            .erosion
            .len()
            .max(y.checked_add(1).ok_or(SolveError::Overflow)?);
        self.erosion.resize_with(height, Vec::new);
        for row in 0..height {
            let start = self.erosion.get(row).ok_or(SolveError::Grid)?.len();
            for column in start..self.width {
                let index = if (column, row) == (0, 0) || (column, row) == self.puzzle.target {
                    0
                } else if row == 0 {
                    u64::try_from(column)?
                        .checked_mul(16_807)
                        .ok_or(SolveError::Overflow)?
                } else if column == 0 {
                    u64::try_from(row)?
                        .checked_mul(48_271)
                        .ok_or(SolveError::Overflow)?
                } else {
                    let left = self.get(column.checked_sub(1).ok_or(SolveError::Grid)?, row)?;
                    let above = self.get(column, row.checked_sub(1).ok_or(SolveError::Grid)?)?;
                    left.checked_mul(above).ok_or(SolveError::Overflow)?
                };
                let erosion = index
                    .checked_add(self.puzzle.depth % EROSION_MODULUS)
                    .ok_or(SolveError::Overflow)?
                    % EROSION_MODULUS;
                self.erosion
                    .get_mut(row)
                    .ok_or(SolveError::Grid)?
                    .push(erosion);
            }
        }
        Ok(())
    }

    fn risk(&mut self) -> Result<u64, SolveError> {
        self.extend(self.puzzle.target.0, self.puzzle.target.1)?;
        self.erosion
            .iter()
            .flatten()
            .try_fold(0_u64, |total, erosion| {
                total.checked_add(erosion % 3).ok_or(SolveError::Overflow)
            })
    }
}

/// Computes the target rectangle's risk; unfinished part two renders as zero.
///
/// # Errors
/// Returns an error for invalid surveys or unrepresentable calculations.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let mut cave = Cave::new(parse(input)?);
    Ok(Answers::new(cave.risk()?, 0))
}

#[cfg(test)]
mod tests {
    use super::{Cave, parse, solve};

    #[test]
    fn mouth_and_target_override_axis_rules() {
        assert_eq!(
            Cave::new(parse("depth: 2\ntarget: 0,0").unwrap())
                .risk()
                .unwrap(),
            2,
            "single region"
        );
        assert_eq!(
            Cave::new(parse("depth: 0\ntarget: 1,0").unwrap())
                .risk()
                .unwrap(),
            0,
            "target axis override"
        );
        assert_eq!(
            Cave::new(parse("depth: 0\ntarget: 0,1").unwrap())
                .risk()
                .unwrap(),
            0,
            "target vertical override"
        );
    }

    #[test]
    fn extending_preserves_predecessors() {
        let mut cave = Cave::new(parse("depth: 0\ntarget: 2,2").unwrap());
        cave.extend(0, 2).unwrap();
        let predecessor = cave.get(0, 1).unwrap();
        cave.extend(2, 2).unwrap();
        assert_eq!(cave.get(2, 2).unwrap(), 0, "target erosion");
        assert_eq!(
            cave.get(0, 1).unwrap(),
            predecessor,
            "preserved existing column"
        );
        assert!(solve("bad").is_err(), "parser error is propagated");
    }
}
