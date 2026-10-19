// approach: count Manhattan distances from the largest-radius nanobot.
// cost: O(n) time and memory, two input passes including parsing.
// trade: widened coordinates retain exact distances at signed 64-bit boundaries.

//! Strongest-transmitter coverage of the observed nanobot positions.

use aoc_core::Answers;

use super::{ParseError, parse};

/// Failure to parse observations or represent a geometric calculation.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// Invalid input observations.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// An intermediate geometric value exceeded its representation.
    #[error("nanobot geometry overflow")]
    Overflow,
    /// The parsed observations unexpectedly contained no nanobots.
    #[error("missing nanobots")]
    Empty,
}

fn distance(left: [i128; 3], right: [i128; 3]) -> Result<u128, SolveError> {
    left.into_iter().zip(right).try_fold(0_u128, |sum, (a, b)| {
        sum.checked_add(a.abs_diff(b)).ok_or(SolveError::Overflow)
    })
}

/// Counts positions covered by the strongest transmitter; part two is unfinished.
///
/// # Errors
/// Returns an error for malformed input or unrepresentable geometry.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let puzzle = parse(input)?;
    let strongest = puzzle
        .bots
        .iter()
        .max_by_key(|bot| bot.radius)
        .ok_or(SolveError::Empty)?;
    let mut count = 0_usize;
    for bot in &puzzle.bots {
        if distance(bot.position, strongest.position)? <= strongest.radius {
            count = count.checked_add(1).ok_or(SolveError::Overflow)?;
        }
    }
    Ok(Answers::new(count, 0))
}

#[cfg(test)]
mod tests {
    use super::solve;

    #[test]
    fn includes_self_and_signal_boundary() {
        assert_eq!(
            solve("pos=<-5,0,0>, r=2\npos=<-3,0,0>, r=0\npos=<-2,0,0>, r=0\n")
                .unwrap()
                .to_string(),
            "2\n0\n",
            "inclusive radius and negative coordinates"
        );
    }

    #[test]
    fn wide_coordinate_distances_are_exact() {
        assert_eq!(
            solve("pos=<-9223372036854775808,0,0>, r=18446744073709551615\npos=<9223372036854775807,0,0>, r=0\n")
                .unwrap()
                .to_string(),
            "2\n0\n",
            "distance across the signed coordinate domain"
        );
    }
}
