//! Advent of Code 2018, day 10: the puzzle's domain model and parser.

pub mod primary;

/// The puzzle input, one entry per line in input order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle<'a> {
    lines: Vec<&'a str>,
}

impl Puzzle<'_> {
    /// Returns the input lines in their original order.
    #[must_use]
    pub fn lines(&self) -> &[&str] {
        &self.lines
    }
}

/// Failure to parse a nonempty sequence of input lines.
pub type ParseError = aoc_core::RecordsError<std::convert::Infallible>;

/// Parses the puzzle input into its domain model.
///
/// # Errors
///
/// Returns [`ParseError::Empty`] when the input holds no lines.
pub fn parse(input: &str) -> Result<Puzzle<'_>, ParseError> {
    let lines = aoc_core::parse_records(input, Ok)?;
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

    #[test]
    fn lines_borrow_the_input_and_strip_crlf() {
        let input = String::from("b\r\n\r\na\r\n");
        let puzzle = parse(&input).unwrap();
        assert_eq!(
            puzzle.lines(),
            ["b", "", "a"],
            "line terminators and empty records"
        );
        assert_eq!(
            puzzle.lines()[0].as_ptr(),
            input.as_ptr(),
            "borrowed storage"
        );
    }
}
