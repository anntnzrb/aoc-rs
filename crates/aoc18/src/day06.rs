//! Validated integer sites for Manhattan-distance regions.

pub mod primary;

/// An integer coordinate site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    x: i32,
    y: i32,
}

/// Nonempty coordinate list in input order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    points: Vec<Point>,
}

/// Malformed coordinate record.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PointError {
    /// Missing coordinate separator.
    #[error("expected x, y")]
    Syntax,
    /// An integer is invalid or outside the coordinate range.
    #[error("invalid coordinate")]
    Integer(#[from] std::num::ParseIntError),
}

/// Input parsing failure, retaining line and cause.
pub type ParseError = aoc_core::RecordsError<PointError>;

/// Parses a nonempty list of signed 32-bit coordinate pairs.
///
/// # Errors
/// Returns [`ParseError`] for empty input or invalid coordinate records.
pub fn parse(input: &str) -> Result<Puzzle, ParseError> {
    let points = aoc_core::parse_records(input, |line| {
        let (x, y) = line.split_once(", ").ok_or(PointError::Syntax)?;
        Ok(Point {
            x: x.parse()?,
            y: y.parse()?,
        })
    })?;
    Ok(Puzzle { points })
}

#[cfg(test)]
mod tests {
    use super::{ParseError, Point, parse};

    #[test]
    fn empty_input_is_rejected() {
        assert!(matches!(parse(""), Err(ParseError::Empty)), "empty input");
    }

    #[test]
    fn lines_keep_their_order() {
        assert_eq!(
            parse("-2, 3\n4, -5\n").unwrap().points,
            [Point { x: -2, y: 3 }, Point { x: 4, y: -5 }],
            "site order"
        );
        for invalid in ["1 2", "1, a", "1, 2, 3", "2147483648, 0"] {
            assert!(
                matches!(parse(invalid), Err(ParseError::Line { .. })),
                "invalid site {invalid}"
            );
        }
    }

    #[test]
    fn lines_borrow_the_input_and_strip_crlf() {
        assert_eq!(
            parse("2, 3\r\n").unwrap().points,
            [Point { x: 2, y: 3 }],
            "CRLF terminator"
        );
    }
}
