// approach: TODO
// cost: TODO
// trade: TODO

//! Advent of Code 2018, day 9: the primary solution.

use aoc_core::Answers;

use super::{ParseError, parse};

/// Solves both parts; an unfinished part renders as `0`.
///
/// # Errors
///
/// Returns [`ParseError`] when the input does not match the puzzle's grammar.
pub fn solve(input: &str) -> Result<Answers, ParseError> {
    let _puzzle = parse(input)?;
    Ok(Answers::new(0, 0))
}
