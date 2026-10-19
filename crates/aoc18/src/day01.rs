//! Signed frequency changes in the order they must be applied.

pub mod primary;

/// Validated changes, one signed integer per input line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    changes: Vec<i64>,
}

impl Puzzle {
    /// Returns the frequency changes in their original order.
    #[must_use]
    pub fn changes(&self) -> &[i64] {
        &self.changes
    }
}

/// Input that does not match the frequency-change grammar.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// The input holds no changes.
    #[error("input is empty")]
    Empty,
    /// A line lacks a sign followed by decimal digits.
    #[error("expected a sign followed by decimal digits")]
    InvalidChange,
    /// A change is outside the supported integer range.
    #[error("frequency change is outside the signed 64-bit range")]
    Integer(#[source] std::num::ParseIntError),
}

/// Parses signed, decimal frequency changes without surrounding whitespace.
///
/// # Errors
///
/// Returns [`ParseError`] for empty input, malformed lines or out-of-range changes.
pub fn parse(input: &str) -> Result<Puzzle, ParseError> {
    let changes = input
        .lines()
        .map(|line| {
            let mut bytes = line.bytes();
            if !matches!(bytes.next(), Some(b'+' | b'-'))
                || bytes.len() == 0
                || !bytes.all(|byte| byte.is_ascii_digit())
            {
                return Err(ParseError::InvalidChange);
            }
            line.parse().map_err(ParseError::Integer)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if changes.is_empty() {
        return Err(ParseError::Empty);
    }
    Ok(Puzzle { changes })
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
        let puzzle = parse("-8\n+4\n").unwrap();
        assert_eq!(puzzle.changes(), [-8, 4], "input order");
    }

    #[test]
    fn malformed_changes_are_rejected() {
        for input in ["7", "+", "-", "+ 3", " +3", "+3 ", "+3\n\n-2", "++2", "+²"] {
            assert_eq!(parse(input), Err(ParseError::InvalidChange), "{input:?}");
        }
        assert!(
            matches!(parse("+9223372036854775808"), Err(ParseError::Integer(_))),
            "positive overflow"
        );
        assert!(
            matches!(parse("-9223372036854775809"), Err(ParseError::Integer(_))),
            "negative overflow"
        );
    }

    #[test]
    fn signed_values_round_trip() {
        for value in -1024_i64..=1024 {
            let text = format!("{value:+}\r\n");
            assert_eq!(parse(&text).unwrap().changes(), [value], "{text:?}");
        }
        assert_eq!(
            parse("-9223372036854775808\n+9223372036854775807")
                .unwrap()
                .changes(),
            [i64::MIN, i64::MAX],
            "integer boundaries"
        );
    }
}
