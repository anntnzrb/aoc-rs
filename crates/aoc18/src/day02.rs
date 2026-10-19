//! Validated lowercase box identifiers in their scanned order.

pub mod primary;

/// The scanned identifiers, borrowed from the puzzle input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle<'a> {
    lines: Vec<&'a str>,
}

impl Puzzle<'_> {
    /// Returns the box identifiers in their original order.
    #[must_use]
    pub fn lines(&self) -> &[&str] {
        &self.lines
    }
}

/// Failure to parse the input's nonempty sequence of box identifiers.
pub type ParseError = aoc_core::RecordsError<IdError>;

/// A line that does not match the box identifier grammar.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum IdError {
    /// An identifier is empty or contains a non-lowercase ASCII letter.
    #[error("expected a nonempty lowercase box identifier")]
    InvalidId,
}

/// Parses one nonempty lowercase ASCII box identifier per line.
///
/// # Errors
///
/// Returns [`ParseError`] for empty input or malformed identifiers.
pub fn parse(input: &str) -> Result<Puzzle<'_>, ParseError> {
    let lines = aoc_core::parse_records(input, |line| {
        if line.is_empty() || !line.bytes().all(|byte| byte.is_ascii_lowercase()) {
            return Err(IdError::InvalidId);
        }
        Ok(line)
    })?;
    Ok(Puzzle { lines })
}

#[cfg(test)]
mod tests {
    use super::{IdError, ParseError, parse};

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
    fn malformed_identifiers_are_rejected() {
        for input in ["\n", "a\n\nb", "a b", "Ab", "a1", "é", "a "] {
            assert!(
                matches!(
                    parse(input),
                    Err(ParseError::Line {
                        source: IdError::InvalidId,
                        ..
                    })
                ),
                "{input:?}"
            );
        }
    }

    #[test]
    fn lowercase_identifiers_round_trip() {
        for letter in b'a'..=b'z' {
            let id = char::from(letter).to_string().repeat(32);
            let text = format!("{id}\r\nz\r\n");
            assert_eq!(
                parse(&text).unwrap().lines(),
                [id.as_str(), "z"],
                "{text:?}"
            );
        }
    }
}
