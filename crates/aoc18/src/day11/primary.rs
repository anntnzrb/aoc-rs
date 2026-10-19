// approach: evaluate every three-by-three square using the stated cell-power formula
// cost: O(N²) time for N=300, O(1) auxiliary memory, one input pass
// trade: direct summation keeps the fixed-size search simple without storing the grid

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

/// Finds the strongest three-by-three square; part two is not yet implemented.
///
/// # Errors
/// Returns [`SolveError`] for invalid input or unrepresentable arithmetic.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let puzzle = parse(input)?;
    let mut best = (i64::MIN, 1, 1);
    let last = GRID_SIZE.checked_sub(2).ok_or(SolveError::Overflow)?;
    for y in 1..=last {
        for x in 1..=last {
            let mut total = 0_i64;
            for dy in 0..3 {
                for dx in 0..3 {
                    let cell = power(
                        puzzle.serial,
                        x.checked_add(dx).ok_or(SolveError::Overflow)?,
                        y.checked_add(dy).ok_or(SolveError::Overflow)?,
                    )?;
                    total = total.checked_add(cell).ok_or(SolveError::Overflow)?;
                }
            }
            if total > best.0 {
                best = (total, x, y);
            }
        }
    }
    Ok(Answers::new(format!("{},{}", best.1, best.2), 0))
}

#[cfg(test)]
mod tests {
    use super::power;

    #[test]
    fn cell_power_handles_zero_hundreds_and_large_serials() {
        assert_eq!(power(0, 1, 1).unwrap(), -4, "121 has hundreds digit one");
        assert_eq!(power(0, 0, 0).unwrap(), -5, "no hundreds digit");
        assert_eq!(power(u32::MAX, 300, 300).unwrap(), -1, "wide intermediate");
    }
}
