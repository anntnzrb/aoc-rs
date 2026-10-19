// approach: rotate a deque to keep the current marble at its back
// cost: O(m+p) time, O(m+p) memory, one input pass
// trade: bounded rotations implement circular placement without pointers or indexed links

//! Circular marble simulation with constant-distance deque rotations.

use std::collections::VecDeque;
use std::num::NonZeroUsize;

use aoc_core::Answers;

use super::{ParseError, parse};

/// Parsing, allocation or score arithmetic failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// Invalid game parameters.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// Score arithmetic exceeds u64.
    #[error("marble score overflow")]
    Overflow,
    /// Storage cannot be reserved.
    #[error("cannot allocate marble game storage")]
    Allocation(#[from] std::collections::TryReserveError),
    /// A required marble or player is absent.
    #[error("invalid marble game state")]
    State,
}

fn high_score(players: NonZeroUsize, last: u64) -> Result<u64, SolveError> {
    let mut scores = Vec::new();
    scores.try_reserve_exact(players.get())?;
    scores.resize(players.get(), 0_u64);
    let mut circle = VecDeque::from([0_u64]);
    let mut player = 0_usize;
    for marble in 1..=last {
        if marble.is_multiple_of(23) {
            circle.rotate_right(7);
            let removed = circle.pop_back().ok_or(SolveError::State)?;
            let score = scores.get_mut(player).ok_or(SolveError::State)?;
            *score = score
                .checked_add(marble)
                .and_then(|value| value.checked_add(removed))
                .ok_or(SolveError::Overflow)?;
            circle.rotate_left(1);
        } else {
            circle.rotate_left(1);
            circle.try_reserve(1)?;
            circle.push_back(marble);
        }
        player = player.checked_add(1).ok_or(SolveError::Overflow)?;
        if player == players.get() {
            player = 0;
        }
    }
    Ok(scores.into_iter().max().unwrap_or(0))
}

/// Finds the highest score; unfinished part two renders `0`.
///
/// # Errors
/// Returns [`SolveError`] for invalid descriptions, storage exhaustion or overflow.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let game = parse(input)?;
    Ok(Answers::new(high_score(game.players, game.last_marble)?, 0))
}

#[cfg(test)]
mod tests {
    use super::high_score;
    use std::num::NonZeroUsize;

    #[test]
    fn score_begins_on_first_special_marble() {
        let players = NonZeroUsize::new(1).unwrap();
        assert_eq!(high_score(players, 0).unwrap(), 0, "initial marble only");
        assert_eq!(
            high_score(players, 22).unwrap(),
            0,
            "ordinary turns do not score"
        );
        assert_eq!(
            high_score(players, 23).unwrap(),
            32,
            "seven counterclockwise is removed"
        );
        assert_eq!(
            high_score(players, 46).unwrap(),
            95,
            "one player accumulates both scores"
        );
    }
}
