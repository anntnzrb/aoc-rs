//! Validated ASCII polymer units and parsing for Advent of Code 2018 day 5.

pub mod primary;

/// A polymer consisting entirely of ASCII letters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle<'a> {
    units: &'a [u8],
}

impl Puzzle<'_> {
    /// Returns the units, retaining their case-dependent polarity.
    #[must_use]
    pub const fn units(&self) -> &[u8] {
        self.units
    }
}

/// Invalid polymer input.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// No polymer was supplied.
    #[error("empty polymer")]
    Empty,
    /// A unit is not an ASCII letter.
    #[error("polymer must contain only ASCII letters")]
    InvalidUnit,
}

/// Parses one polymer with an optional final line terminator.
///
/// # Errors
/// Returns [`ParseError`] for missing units, multiple lines or nonletters.
pub fn parse(input: &str) -> Result<Puzzle<'_>, ParseError> {
    let polymer = input.strip_suffix('\n').unwrap_or(input);
    let polymer = polymer.strip_suffix('\r').unwrap_or(polymer);
    if polymer.is_empty() {
        return Err(ParseError::Empty);
    }
    if !polymer.bytes().all(|unit| unit.is_ascii_alphabetic()) {
        return Err(ParseError::InvalidUnit);
    }
    Ok(Puzzle {
        units: polymer.as_bytes(),
    })
}

#[cfg(test)]
mod tests {
    use super::{ParseError, parse};

    #[test]
    fn empty_input_is_rejected() {
        assert_eq!(parse(""), Err(ParseError::Empty), "no units");
    }

    #[test]
    fn lines_keep_their_order() {
        let puzzle = parse("bAa\n").unwrap();
        assert_eq!(puzzle.units(), b"bAa", "unit order and polarity");
        assert_eq!(
            parse("b\na\n"),
            Err(ParseError::InvalidUnit),
            "one polymer only"
        );
        assert_eq!(parse("é"), Err(ParseError::InvalidUnit), "ASCII only");
    }

    #[test]
    fn lines_borrow_the_input_and_strip_crlf() {
        let input = String::from("bAa\r\n");
        let puzzle = parse(&input).unwrap();
        assert_eq!(puzzle.units(), b"bAa", "line terminator removed");
        assert_eq!(puzzle.units().as_ptr(), input.as_ptr(), "borrowed storage");
    }
}
