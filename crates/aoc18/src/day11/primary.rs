// approach: summed-area table gives constant-time square totals; enumerate every size and origin
// cost: O(N³) time and O(N²) memory for N=300, one input pass
// trade: storing prefix sums avoids repeatedly adding the same cells for overlapping squares

//! Maximum-power fuel squares for Advent of Code 2018, day 11.

use aoc_core::Answers;

use super::{ParseError, parse};

const GRID_SIZE: u32 = 300;

/// Failures while parsing the serial number or calculating grid power.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// The serial number is invalid.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// An intermediate value exceeds its representation.
    #[error("grid power arithmetic overflow")]
    Overflow,
    /// A table coordinate is not representable on this platform.
    #[error("grid coordinate conversion failed")]
    Coordinate(#[from] std::num::TryFromIntError),
    /// A coordinate falls outside the summed-area table.
    #[error("grid coordinate outside table")]
    Bounds,
}

fn power(serial: u32, x: u32, y: u32) -> Result<i64, SolveError> {
    let rack = i64::from(x).checked_add(10).ok_or(SolveError::Overflow)?;
    let value = rack
        .checked_mul(i64::from(y))
        .and_then(|value| value.checked_add(i64::from(serial)))
        .and_then(|value| value.checked_mul(rack))
        .ok_or(SolveError::Overflow)?;
    let digit = value
        .checked_div(100)
        .and_then(|value| value.checked_rem(10))
        .ok_or(SolveError::Overflow)?;
    digit.checked_sub(5).ok_or(SolveError::Overflow)
}

struct Grid {
    stride: usize,
    prefix: Vec<i64>,
}

impl Grid {
    fn new(serial: u32, size: u32) -> Result<Self, SolveError> {
        let stride = usize::try_from(size)?
            .checked_add(1)
            .ok_or(SolveError::Overflow)?;
        let len = stride.checked_mul(stride).ok_or(SolveError::Overflow)?;
        let mut grid = Self {
            stride,
            prefix: vec![0; len],
        };
        for y in 1..=size {
            let mut row = 0_i64;
            for x in 1..=size {
                row = row
                    .checked_add(power(serial, x, y)?)
                    .ok_or(SolveError::Overflow)?;
                let above = grid.at(x, y.checked_sub(1).ok_or(SolveError::Overflow)?)?;
                let index = grid.index(x, y)?;
                *grid.prefix.get_mut(index).ok_or(SolveError::Bounds)? =
                    row.checked_add(above).ok_or(SolveError::Overflow)?;
            }
        }
        Ok(grid)
    }

    fn index(&self, x: u32, y: u32) -> Result<usize, SolveError> {
        let x = usize::try_from(x)?;
        let y = usize::try_from(y)?;
        if x >= self.stride || y >= self.stride {
            return Err(SolveError::Bounds);
        }
        y.checked_mul(self.stride)
            .and_then(|offset| offset.checked_add(x))
            .ok_or(SolveError::Overflow)
    }

    fn at(&self, x: u32, y: u32) -> Result<i64, SolveError> {
        self.prefix
            .get(self.index(x, y)?)
            .copied()
            .ok_or(SolveError::Bounds)
    }

    fn square(&self, x: u32, y: u32, size: u32) -> Result<i64, SolveError> {
        let right = x.checked_add(size).ok_or(SolveError::Overflow)?;
        let bottom = y.checked_add(size).ok_or(SolveError::Overflow)?;
        let bottom_right = self.at(right, bottom)?;
        let bottom_left = self.at(x, bottom)?;
        let top_right = self.at(right, y)?;
        let top_left = self.at(x, y)?;
        bottom_right
            .checked_sub(bottom_left)
            .and_then(|total| total.checked_sub(top_right))
            .and_then(|total| total.checked_add(top_left))
            .ok_or(SolveError::Overflow)
    }
}

/// Finds the strongest three-by-three square and the strongest square of any size.
///
/// # Errors
/// Returns [`SolveError`] for invalid input, arithmetic overflow or invalid table access.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let puzzle = parse(input)?;
    let grid = Grid::new(puzzle.serial, GRID_SIZE)?;
    let mut best = (i64::MIN, 1, 1, 1);
    let mut best_three = (i64::MIN, 1, 1);
    for size in 1..=GRID_SIZE {
        let last = GRID_SIZE.checked_sub(size).ok_or(SolveError::Overflow)?;
        for y in 0..=last {
            for x in 0..=last {
                let total = grid.square(x, y, size)?;
                if total > best.0 {
                    best = (total, x, y, size);
                }
                if size == 3 && total > best_three.0 {
                    best_three = (total, x, y);
                }
            }
        }
    }
    Ok(Answers::new(
        format!(
            "{},{}",
            best_three.1.checked_add(1).ok_or(SolveError::Overflow)?,
            best_three.2.checked_add(1).ok_or(SolveError::Overflow)?
        ),
        format!(
            "{},{},{}",
            best.1.checked_add(1).ok_or(SolveError::Overflow)?,
            best.2.checked_add(1).ok_or(SolveError::Overflow)?,
            best.3
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::{Grid, SolveError, power};

    #[test]
    fn cell_power_handles_zero_hundreds_and_large_serials() {
        assert_eq!(power(0, 1, 1).unwrap(), -4, "121 has hundreds digit one");
        assert_eq!(power(0, 0, 0).unwrap(), -5, "no hundreds digit");
        assert_eq!(power(u32::MAX, 300, 300).unwrap(), -1, "wide intermediate");
    }

    #[test]
    fn square_totals_include_boundaries_and_negative_cells() {
        let grid = Grid::new(0, 2).unwrap();
        assert_eq!(grid.square(0, 0, 2).unwrap(), -14, "whole grid");
        assert_eq!(grid.square(1, 1, 1).unwrap(), -3, "bottom-right cell");
        assert!(
            matches!(grid.square(2, 2, 1), Err(SolveError::Bounds)),
            "outside grid"
        );
        assert!(
            matches!(grid.square(u32::MAX, 0, 1), Err(SolveError::Overflow)),
            "coordinate overflow"
        );
    }
}
