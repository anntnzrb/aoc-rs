// approach: keep the current marble at the back of a deque, moving single marbles between its ends
// cost: O(m+p) time, O(m+p) memory (4 bytes per marble), one input pass
// trade: end-to-end pops and pushes avoid generic rotation copies; marbles must fit u32

//! Circular marble simulation moving single marbles between the ends of a deque.

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
    /// The last marble does not fit the circle's 32-bit storage.
    #[error("last marble exceeds the supported range")]
    Marbles(#[from] std::num::TryFromIntError),
    /// A required marble or player is absent.
    #[error("invalid marble game state")]
    State,
}

/// Moves the front marble to the back, advancing the current marble clockwise.
fn clockwise(circle: &mut VecDeque<u32>) -> Result<(), SolveError> {
    let front = circle.pop_front().ok_or(SolveError::State)?;
    circle.push_back(front);
    Ok(())
}

fn high_score(players: NonZeroUsize, last: u64) -> Result<u64, SolveError> {
    let last = u32::try_from(last)?;
    let mut scores = Vec::new();
    scores.try_reserve_exact(players.get())?;
    scores.resize(players.get(), 0_u64);
    // One up-front reservation replaces repeated growth copies of the circle.
    let capacity = usize::try_from(last)?
        .checked_add(1)
        .ok_or(SolveError::Overflow)?;
    let mut circle = VecDeque::new();
    circle.try_reserve_exact(capacity)?;
    circle.push_back(0_u32);
    let mut player = 0_usize;
    for marble in 1..=last {
        if marble.is_multiple_of(23) {
            for _ in 0..7 {
                let back = circle.pop_back().ok_or(SolveError::State)?;
                circle.push_front(back);
            }
            let removed = circle.pop_back().ok_or(SolveError::State)?;
            let score = scores.get_mut(player).ok_or(SolveError::State)?;
            *score = score
                .checked_add(u64::from(marble))
                .and_then(|value| value.checked_add(u64::from(removed)))
                .ok_or(SolveError::Overflow)?;
            clockwise(&mut circle)?;
        } else {
            clockwise(&mut circle)?;
            circle.push_back(marble);
        }
        player = player.checked_add(1).ok_or(SolveError::Overflow)?;
        if player == players.get() {
            player = 0;
        }
    }
    Ok(scores.into_iter().max().unwrap_or(0))
}

/// Finds the highest scores for the specified and hundredfold larger games.
///
/// # Errors
/// Returns [`SolveError`] for invalid descriptions, storage exhaustion or overflow.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let game = parse(input)?;
    let extended = game
        .last_marble
        .checked_mul(100)
        .ok_or(SolveError::Overflow)?;
    Ok(Answers::new(
        high_score(game.players, game.last_marble)?,
        high_score(game.players, extended)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::{SolveError, high_score};
    use std::num::NonZeroUsize;

    #[test]
    fn marbles_beyond_32_bits_are_rejected() {
        let players = NonZeroUsize::new(1).unwrap();
        assert!(
            matches!(
                high_score(players, u64::from(u32::MAX) + 1),
                Err(SolveError::Marbles(_))
            ),
            "the circle stores u32 marbles"
        );
    }

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
