//! Cave survey parameters and validated parsing for Advent of Code 2018, day 22.

use std::num::ParseIntError;

pub mod primary;

/// A cave's depth and the target's nonnegative coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Puzzle {
    depth: u64,
    target: (usize, usize),
}

/// A malformed cave survey.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// The two labeled records or coordinate pair are missing or malformed.
    #[error("expected depth and target records")]
    Format,
    /// A parameter is not an unsigned integer.
    #[error("invalid survey integer")]
    Integer(#[from] ParseIntError),
}

/// Parses the two labeled cave survey records.
///
/// # Errors
/// Returns an error for missing, extra, or malformed records and invalid integers.
pub fn parse(input: &str) -> Result<Puzzle, ParseError> {
    let mut lines = input.lines();
    let depth = lines
        .next()
        .and_then(|line| line.strip_prefix("depth: "))
        .ok_or(ParseError::Format)?
        .parse()?;
    let (x, y) = lines
        .next()
        .and_then(|line| line.strip_prefix("target: "))
        .and_then(|line| line.split_once(','))
        .ok_or(ParseError::Format)?;
    let target = (x.parse()?, y.parse()?);
    if lines.next().is_some() {
        return Err(ParseError::Format);
    }
    Ok(Puzzle { depth, target })
}

#[cfg(test)]
mod tests {
    use super::{ParseError, Puzzle, parse};

    #[test]
    fn empty_input_is_rejected() {
        assert!(
            matches!(parse(""), Err(ParseError::Format)),
            "missing survey"
        );
    }

    #[test]
    fn lines_keep_their_order() {
        assert_eq!(
            parse("depth: 0\ntarget: 0,0\n").unwrap(),
            Puzzle {
                depth: 0,
                target: (0, 0)
            },
            "labeled survey"
        );
        assert!(
            matches!(parse("target: 0,0\ndepth: 0\n"), Err(ParseError::Format)),
            "record order"
        );
    }

    #[test]
    fn lines_borrow_the_input_and_strip_crlf() {
        assert_eq!(
            parse("depth: 7\r\ntarget: 3,4\r\n").unwrap(),
            Puzzle {
                depth: 7,
                target: (3, 4)
            },
            "CRLF survey"
        );
    }

    #[test]
    fn invalid_surveys_are_rejected() {
        for input in [
            "depth: 1\ntarget: 1\n",
            "depth: 1\ntarget: 1,2\nextra",
            "depth: 1\n",
        ] {
            assert!(
                matches!(parse(input), Err(ParseError::Format)),
                "bad structure: {input}"
            );
        }
        for input in [
            "depth: -1\ntarget: 1,2",
            "depth: 1\ntarget: 1,-2",
            "depth: 1\ntarget: a,2",
        ] {
            assert!(
                matches!(parse(input), Err(ParseError::Integer(_))),
                "bad integer: {input}"
            );
        }
    }
}
