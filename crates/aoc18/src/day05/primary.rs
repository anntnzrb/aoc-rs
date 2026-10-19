// approach: keep a stack of unreacted units, cancelling opposite-polarity neighbors
// cost: O(n) time, O(n) memory, two input passes including validation
// trade: a byte stack exposes newly adjacent pairs without rescanning the polymer

//! Stack reduction of a polymer's adjacent, opposite-polarity units.

use aoc_core::Answers;

use super::{ParseError, parse};

fn reacted_length(units: &[u8]) -> usize {
    let mut stack = Vec::with_capacity(units.len());
    for &unit in units {
        if stack
            .last()
            .is_some_and(|&previous: &u8| previous != unit && previous.eq_ignore_ascii_case(&unit))
        {
            stack.truncate(stack.len().saturating_sub(1));
        } else {
            stack.push(unit);
        }
    }
    stack.len()
}

/// Solves part one; the unfinished second part renders as `0`.
///
/// # Errors
/// Returns [`ParseError`] when the input is not an ASCII polymer.
pub fn solve(input: &str) -> Result<Answers, ParseError> {
    let puzzle = parse(input)?;
    Ok(Answers::new(reacted_length(puzzle.units()), 0))
}

#[cfg(test)]
mod tests {
    use super::reacted_length;

    #[test]
    fn reactions_reveal_earlier_units() {
        assert_eq!(reacted_length(b"xYyzZX"), 0, "cascading cancellations");
        assert_eq!(reacted_length(b"aabb"), 4, "same polarity remains");
        assert_eq!(reacted_length(b"aBc"), 3, "distinct types remain");
        assert_eq!(reacted_length(b""), 0, "empty remainder");
    }
}
