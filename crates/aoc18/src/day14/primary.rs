// approach: Append sum digits to a scoreboard and move the two recipe cursors cyclically.
// cost: O(n) time and memory for n requested recipes; one input pass.
// trade: A byte per score keeps random cursor access simple while retaining the whole board.

//! Chocolate scoreboard simulation.

use aoc_core::Answers;

use super::{ParseError, parse};

/// Failure to parse input or represent a scoreboard operation.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// The recipe count is invalid.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// A size or cursor calculation overflowed.
    #[error("scoreboard size exceeds the platform limit")]
    Overflow,
    /// A cursor does not refer to an existing recipe.
    #[error("scoreboard cursor invariant failed")]
    Cursor,
}

struct Scoreboard {
    scores: Vec<u8>,
    elves: [usize; 2],
}

impl Scoreboard {
    fn new() -> Self {
        Self {
            scores: vec![3, 7],
            elves: [0, 1],
        }
    }

    fn step(&mut self) -> Result<(), SolveError> {
        let [first, second] = self.elves;
        let first_score = *self.scores.get(first).ok_or(SolveError::Cursor)?;
        let second_score = *self.scores.get(second).ok_or(SolveError::Cursor)?;
        let sum = first_score
            .checked_add(second_score)
            .ok_or(SolveError::Overflow)?;
        if sum >= 10 {
            self.scores.push(1);
        }
        self.scores
            .push(sum.checked_rem(10).ok_or(SolveError::Overflow)?);
        for (elf, score) in self.elves.iter_mut().zip([first_score, second_score]) {
            *elf = elf
                .checked_add(usize::from(score))
                .and_then(|position| position.checked_add(1))
                .and_then(|position| position.checked_rem(self.scores.len()))
                .ok_or(SolveError::Overflow)?;
        }
        Ok(())
    }

    fn following(&self, count: usize, end: usize) -> Result<String, SolveError> {
        self.scores
            .get(count..end)
            .ok_or(SolveError::Cursor)?
            .iter()
            .map(|score| {
                b'0'.checked_add(*score)
                    .map(char::from)
                    .ok_or(SolveError::Overflow)
            })
            .collect()
    }
}

/// Returns ten scores after the requested recipe count; part two is unfinished.
///
/// # Errors
/// Returns an error for invalid input, arithmetic overflow, or an invalid cursor.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let puzzle = parse(input)?;
    let end = puzzle.count.checked_add(10).ok_or(SolveError::Overflow)?;
    let mut board = Scoreboard::new();
    while board.scores.len() < end {
        board.step()?;
    }
    Ok(Answers::new(board.following(puzzle.count, end)?, 0))
}

#[cfg(test)]
mod tests {
    use super::{SolveError, solve};

    #[test]
    fn zero_keeps_the_initial_scores() {
        assert_eq!(solve("0").unwrap().part1(), "3710101245", "initial board");
    }
    #[test]
    fn rejects_unrepresentable_end() {
        assert!(
            matches!(solve(&usize::MAX.to_string()), Err(SolveError::Overflow)),
            "end overflow"
        );
    }
}
