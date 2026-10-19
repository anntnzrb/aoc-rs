//! Advent of Code 2018, day 2: the puzzle's domain model and parser.

pub mod primary;

/// The puzzle input, one entry per line in input order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    lines: Vec<String>,
}

impl Puzzle {
    /// Returns the input lines in their original order.
    #[must_use]
    pub fn lines(&self) -> &[String] {
        &self.lines
    }
}

/// Input that does not match the puzzle's grammar.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// The input holds no lines.
    #[error("input is empty")]
    Empty,
}

/// Parses the puzzle input into its domain model.
///
/// # Errors
///
/// Returns [`ParseError::Empty`] when the input holds no lines.
pub fn parse(input: &str) -> Result<Puzzle, ParseError> {
    let lines: Vec<String> = input.lines().map(str::to_owned).collect();
    if lines.is_empty() {
        return Err(ParseError::Empty);
    }
    Ok(Puzzle { lines })
}

#[cfg(test)]
mod tests {
    use super::{ParseError, parse};

    #[test]
    fn empty_input_is_rejected() {
        assert_eq!(parse(""), Err(ParseError::Empty), "no lines");
    }

    #[test]
    fn lines_keep_their_order() {
        let puzzle = parse("b\na\n").unwrap();
        assert_eq!(puzzle.lines(), ["b", "a"], "input order");
    }
}
