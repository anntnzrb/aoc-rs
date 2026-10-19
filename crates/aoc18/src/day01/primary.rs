// approach: Compare cycle prefixes modulo drift; shift earlier occurrences into cycle zero.
// cost: O(n²) time, O(n) memory; one parse, one accumulation, then buffered prefix pairs.
// trade: Finite drift divisibility proves termination; zero drift repeats within one cycle.

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
    /// An input position cannot be represented by the timing arithmetic.
    #[error("input position is outside the signed 128-bit range")]
    Position(#[from] std::num::TryFromIntError),
}

/// Solves both parts, omitting part two when no frequency can repeat.
///
/// # Errors
///
/// Returns [`SolveError`] for invalid changes or accumulated frequency overflow.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let puzzle = parse(input)?;
    let mut prefixes = Vec::with_capacity(puzzle.changes().len().saturating_add(1));
    let mut frequency = 0_i64;
    prefixes.push(frequency);
    for change in puzzle.changes() {
        frequency = frequency.checked_add(*change).ok_or(SolveError::Overflow)?;
        prefixes.push(frequency);
    }
    let repeated = first_repeat(&prefixes, frequency, puzzle.changes().len())?;
    // Some part-one examples never repeat, so they have no second answer to render.
    Ok(repeated.map_or_else(
        || Answers::part1_only(frequency),
        |repeat| Answers::new(frequency, repeat),
    ))
}

fn first_repeat(prefixes: &[i64], drift: i64, length: usize) -> Result<Option<i64>, SolveError> {
    let length = i128::try_from(length)?;
    let mut first: Option<(i128, i64)> = None;
    for (earlier, &frequency) in prefixes.iter().enumerate() {
        let earlier = i128::try_from(earlier)?;
        for (later, &other) in prefixes.iter().enumerate() {
            let later = i128::try_from(later)?;
            let difference = i128::from(frequency)
                .checked_sub(i128::from(other))
                .ok_or(SolveError::Overflow)?;
            let cycles = if drift == 0 {
                if difference != 0 {
                    continue;
                }
                0
            } else {
                let drift = i128::from(drift);
                if difference.checked_rem(drift) != Some(0) {
                    continue;
                }
                difference.checked_div(drift).ok_or(SolveError::Overflow)?
            };
            if cycles < 0 {
                continue;
            }
            let time = cycles
                .checked_mul(length)
                .and_then(|time| time.checked_add(later))
                .ok_or(SolveError::Overflow)?;
            if time > earlier && first.is_none_or(|(best, _)| time < best) {
                first = Some((time, frequency));
            }
        }
    }
    Ok(first.map(|(_, frequency)| frequency))
}

#[cfg(test)]
mod tests {
    use aoc_core::Answers;

    use super::{SolveError, solve};

    #[test]
    fn accumulates_in_order_from_zero() {
        assert_eq!(
            solve("-8\n+4\n+7").unwrap(),
            Answers::part1_only(3),
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

    #[test]
    fn no_repeat_has_only_the_resulting_frequency() {
        assert_eq!(
            solve("+2\n+4").unwrap(),
            Answers::part1_only(6),
            "positive drift without a matching residue"
        );
        assert_eq!(
            solve("-2\n-4").unwrap(),
            Answers::part1_only(-6),
            "negative drift without a matching residue"
        );
    }

    #[test]
    fn detects_initial_zero_and_in_cycle_repeats() {
        assert_eq!(
            solve("+4\n-4").unwrap(),
            Answers::new(0, 0),
            "initial zero was visited"
        );
        assert_eq!(
            solve("+4\n-4\n+2").unwrap(),
            Answers::new(2, 0),
            "stop before cycle end"
        );
        assert_eq!(
            solve("-4\n+2\n+1").unwrap(),
            Answers::new(-1, -2),
            "negative cycle drift"
        );
    }
}
