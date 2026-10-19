// approach: keep a stack of unreacted units, cancelling opposite-polarity neighbors
// cost: O(26n) time, O(n) memory, 28 input passes including validation
// trade: a byte stack exposes newly adjacent pairs without rescanning the polymer

//! Stack reduction of a polymer's adjacent, opposite-polarity units.

use aoc_core::Answers;

use super::{ParseError, parse};

fn reacted_length(units: &[u8], removed: Option<u8>, stack: &mut Vec<u8>) -> usize {
    stack.clear();
    for &unit in units {
        if removed.is_some_and(|kind| unit.eq_ignore_ascii_case(&kind)) {
            continue;
        }
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

/// Reduces the polymer, then tries removing each possible unit type.
///
/// # Errors
/// Returns [`ParseError`] when the input is not an ASCII polymer.
pub fn solve(input: &str) -> Result<Answers, ParseError> {
    let puzzle = parse(input)?;
    let mut stack = Vec::with_capacity(puzzle.units().len());
    let part1 = reacted_length(puzzle.units(), None, &mut stack);
    let part2 = (b'a'..=b'z')
        .map(|kind| reacted_length(puzzle.units(), Some(kind), &mut stack))
        .min()
        .unwrap_or(0);
    Ok(Answers::new(part1, part2))
}

#[cfg(test)]
mod tests {
    use super::reacted_length;

    #[test]
    fn reactions_reveal_earlier_units() {
        let mut stack = Vec::new();
        assert_eq!(
            reacted_length(b"xYyzZX", None, &mut stack),
            0,
            "cascading cancellations"
        );
        assert_eq!(
            reacted_length(b"aabb", None, &mut stack),
            4,
            "same polarity remains"
        );
        assert_eq!(
            reacted_length(b"aBc", None, &mut stack),
            3,
            "distinct types remain"
        );
        assert_eq!(reacted_length(b"", None, &mut stack), 0, "empty remainder");
        assert_eq!(
            reacted_length(b"abA", Some(b'b'), &mut stack),
            0,
            "removal exposes a reaction"
        );
        assert_eq!(
            reacted_length(b"aAa", Some(b'a'), &mut stack),
            0,
            "both polarities removed"
        );
    }
}
