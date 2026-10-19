// approach: Apply each signed change to an initially zero frequency.
// cost: O(n) time and memory; one parse pass and one buffered change traversal.
// trade: Store validated integers to keep parsing separate from frequency arithmetic.

//! Calibrate the device by accumulating its frequency changes.

use aoc_core::Answers;

use super::{ParseError, parse};

/// A malformed change or an unrepresentable accumulated frequency.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// The input contains an invalid frequency change.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// Applying a change exceeds the supported frequency range.
    #[error("accumulated frequency is outside the signed 64-bit range")]
    Overflow,
}

/// Solves part one; the unfinished second part renders as `0`.
///
/// # Errors
///
/// Returns [`SolveError`] for invalid changes or accumulated frequency overflow.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let puzzle = parse(input)?;
    let frequency = puzzle
        .changes()
        .iter()
        .try_fold(0_i64, |frequency, change| {
            frequency.checked_add(*change).ok_or(SolveError::Overflow)
        })?;
    Ok(Answers::new(frequency, 0))
}

#[cfg(test)]
mod tests {
    use aoc_core::Answers;

    use super::{SolveError, solve};

    #[test]
    fn accumulates_in_order_from_zero() {
        assert_eq!(
            solve("-8\n+4\n+7").unwrap(),
            Answers::new(3, 0),
            "signed accumulation"
        );
        assert_eq!(solve("+0").unwrap(), Answers::new(0, 0), "zero change");
    }

    #[test]
    fn overflow_is_an_error() {
        for input in ["+9223372036854775807\n+1", "-9223372036854775808\n-1"] {
            assert!(matches!(solve(input), Err(SolveError::Overflow)), "{input}");
        }
    }
}
