//! Recipe-count input and validation for the chocolate scoreboard.

pub mod primary;

/// A validated number of recipes to skip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle<'a> {
    text: &'a str,
    count: usize,
}

/// Invalid recipe-count input.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// No recipe count was supplied.
    #[error("missing recipe count")]
    Empty,
    /// The input is not one decimal number.
    #[error("expected one nonnegative decimal recipe count")]
    Grammar,
    /// The number exceeds the platform's capacity.
    #[error("recipe count is too large")]
    Count(#[source] std::num::ParseIntError),
}

/// Parses one decimal recipe count, preserving any leading zeroes.
///
/// # Errors
/// Returns an error for missing, nondecimal, multiple, or oversized counts.
pub fn parse(input: &str) -> Result<Puzzle<'_>, ParseError> {
    let text = input.trim();
    if text.is_empty() {
        return Err(ParseError::Empty);
    }
    if !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ParseError::Grammar);
    }
    let count = text.parse().map_err(ParseError::Count)?;
    Ok(Puzzle { text, count })
}

#[cfg(test)]
mod tests {
    use super::{ParseError, parse};

    #[test]
    fn empty_input_is_rejected() {
        assert!(matches!(parse(""), Err(ParseError::Empty)), "no count");
    }

    #[test]
    fn lines_keep_their_order() {
        for input in ["1\n2", "-1", "+1", "1.0", "a"] {
            assert!(
                matches!(parse(input), Err(ParseError::Grammar)),
                "{input:?}"
            );
        }
    }

    #[test]
    fn lines_borrow_the_input_and_strip_crlf() {
        let input = String::from("0012\r\n");
        let puzzle = parse(&input).unwrap();
        assert_eq!(puzzle.count, 12, "numeric interpretation");
        assert_eq!(puzzle.text, "0012", "leading zeroes preserved");
        assert_eq!(puzzle.text.as_ptr(), input.as_ptr(), "borrowed token");
    }

    #[test]
    fn oversized_count_retains_its_cause() {
        assert!(
            matches!(
                parse("99999999999999999999999999999"),
                Err(ParseError::Count(_))
            ),
            "oversized count"
        );
    }
}
