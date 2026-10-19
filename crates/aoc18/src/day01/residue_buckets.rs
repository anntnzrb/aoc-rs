// approach: Sort cycle prefixes into drift-residue buckets; adjacent distinct values meet first.
// cost: O(n log n) time, O(n) memory; one parse and one buffered accumulation pass.
// trade: 1.93 M work instructions vs primary 105.32 M; 4.70 vs 12.30 ms, 2168 vs 2208 KiB.

//! Find the earliest repeated frequency using sorted residue buckets.

use aoc_core::Answers;

use super::{ParseError, parse};

/// Invalid changes or arithmetic outside the supported frequency and timing ranges.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// The input contains an invalid frequency change.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// Accumulation or repeat timing exceeds its integer range.
    #[error("frequency or repeat timing is outside the supported integer range")]
    Overflow,
    /// An input position cannot be represented by the timing arithmetic.
    #[error("input position is outside the signed 128-bit range")]
    Position(#[from] std::num::TryFromIntError),
}

/// Solves both parts, omitting part two when no frequency can repeat.
///
/// # Errors
///
/// Returns [`SolveError`] for invalid changes or frequency or timing overflow.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let puzzle = parse(input)?;
    let length = i128::try_from(puzzle.changes().len())?;
    let mut prefixes = Vec::with_capacity(puzzle.changes().len());
    let mut frequency = 0_i64;
    for (position, &change) in puzzle.changes().iter().enumerate() {
        prefixes.push((
            0,
            i128::from(frequency),
            i128::try_from(position)?,
            frequency,
        ));
        frequency = frequency.checked_add(change).ok_or(SolveError::Overflow)?;
    }
    let repeated = first_repeat(prefixes, frequency, length)?;
    Ok(repeated.map_or_else(
        || Answers::part1_only(frequency),
        |repeat| Answers::new(frequency, repeat),
    ))
}

fn first_repeat(
    mut buckets: Vec<(i128, i128, i128, i64)>,
    drift: i64,
    length: i128,
) -> Result<Option<i64>, SolveError> {
    let magnitude = i128::from(drift)
        .checked_abs()
        .ok_or(SolveError::Overflow)?;
    for (residue, value, _, _) in &mut buckets {
        // Reflect negative drift so every later cycle moves toward larger values.
        if drift < 0 {
            *value = value.checked_neg().ok_or(SolveError::Overflow)?;
        }
        *residue = if drift == 0 {
            *value
        } else {
            value
                .checked_rem_euclid(magnitude)
                .ok_or(SolveError::Overflow)?
        };
    }
    buckets.sort_unstable();
    // Canonical prefixes exclude the endpoint: it is cycle one's initial zero.
    let mut first = (drift == 0).then_some((length, 0));
    let mut previous: Option<(i128, i128, i128)> = None;
    for (residue, value, position, frequency) in buckets {
        if let Some((old_residue, old_value, old_position)) = previous
            && residue == old_residue
        {
            let time = if value == old_value {
                // Equal values are ordered by position; keep the earliest as representative.
                position
            } else {
                // A prefix at v reaches the next larger w after (w-v)/|drift| cycles.
                // Its second visit is k*n + index(v); k >= 1 makes it later than index(w).
                // Any nonadjacent w has a larger k for this v, so cannot win globally.
                value
                    .checked_sub(old_value)
                    .and_then(|gap| gap.checked_div(magnitude))
                    .and_then(|cycles| cycles.checked_mul(length))
                    .and_then(|time| time.checked_add(old_position))
                    .ok_or(SolveError::Overflow)?
            };
            if first.is_none_or(|(best, _)| time < best) {
                first = Some((time, frequency));
            }
            if value == old_value {
                continue;
            }
        }
        previous = Some((residue, value, position));
    }
    Ok(first.map(|(_, frequency)| frequency))
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;

    use aoc_core::Answers;

    use super::{SolveError, solve};

    #[test]
    fn short_sequences_agree_with_the_primary() {
        for length in 1..=6 {
            for mut encoded in 0_u32..5_u32.pow(length) {
                let mut input = String::new();
                for _ in 0..length {
                    let change = i64::from(encoded % 5) - 2;
                    writeln!(input, "{change:+}").unwrap();
                    encoded /= 5;
                }
                assert_eq!(
                    solve(&input).unwrap(),
                    crate::day01::primary::solve(&input).unwrap(),
                    "{input}"
                );
            }
        }
    }

    #[test]
    fn negative_drift_reflects_the_bucket_order() {
        assert_eq!(
            solve("-9\n+5\n+2").unwrap(),
            Answers::new(-2, -4),
            "the lower frequency is reached again in a later negative-drift cycle"
        );
    }

    #[test]
    fn zero_drift_uses_exact_values_and_the_cycle_boundary() {
        assert_eq!(
            solve("+6\n-6").unwrap(),
            Answers::new(0, 0),
            "cycle boundary"
        );
        assert_eq!(
            solve("+6\n+2\n-2\n-6").unwrap(),
            Answers::new(0, 6),
            "an exact duplicate precedes the cycle boundary"
        );
        assert_eq!(solve("+0").unwrap(), Answers::new(0, 0), "single zero");
    }

    #[test]
    fn isolated_residues_have_no_second_answer() {
        for (input, total) in [("+3\n+5", 8), ("-3\n-5", -8), ("+7", 7)] {
            assert_eq!(solve(input).unwrap(), Answers::part1_only(total), "{input}");
        }
    }

    #[test]
    fn first_cycle_repeat_beats_future_bucket_collisions() {
        assert_eq!(
            solve("+5\n-5\n+2").unwrap(),
            Answers::new(2, 0),
            "initial zero repeats before any shifted prefix"
        );
        assert_eq!(
            solve("+5\n+3\n-3\n+2").unwrap(),
            Answers::new(7, 5),
            "a nonzero duplicate also keeps its earlier representative"
        );
    }

    #[test]
    fn several_values_in_one_residue_choose_time_not_frequency_order() {
        assert_eq!(
            solve("+12\n-8\n+4\n-6").unwrap(),
            Answers::new(2, 4),
            "the nearest gap and the lower prefix's position determine the second visit"
        );
        assert_eq!(
            solve("+12\n-6\n-2\n-2").unwrap(),
            Answers::new(2, 6),
            "a later adjacent pair can beat the first pair in sorted order"
        );
    }

    #[test]
    fn wide_differences_and_minimum_drift_are_supported() {
        assert_eq!(
            solve("-9223372036854775808").unwrap(),
            Answers::part1_only(i64::MIN),
            "drift magnitude is computed in i128"
        );
        assert_eq!(
            solve("+9223372036854775807\n-9223372036854775807\n-1").unwrap(),
            Answers::new(-1, 0),
            "large residue gaps do not overflow i64 arithmetic"
        );
        for input in ["+9223372036854775807\n+1", "-9223372036854775808\n-1"] {
            assert!(matches!(solve(input), Err(SolveError::Overflow)), "{input}");
        }
    }
}
