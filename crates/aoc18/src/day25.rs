//! Four-dimensional fixed points parsed from signed comma-separated coordinates.

pub mod primary;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Point([i32; 4]);

impl Point {
    fn is_near(self, other: Self) -> bool {
        self.0
            .into_iter()
            .zip(other.0)
            .try_fold(3_u32, |budget, (a, b)| budget.checked_sub(a.abs_diff(b)))
            .is_some()
    }
}

/// Nonempty observations of four-dimensional fixed points.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    points: Vec<Point>,
}

/// An invalid four-dimensional coordinate record.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PointError {
    /// A record has something other than four coordinates.
    #[error("expected four comma-separated coordinates")]
    Dimensions,
    /// A coordinate is not a signed 32-bit integer.
    #[error("invalid point coordinate")]
    Integer(#[from] std::num::ParseIntError),
}

/// Input failure retaining the record number and underlying cause.
pub type ParseError = aoc_core::RecordsError<PointError>;

/// Parses nonempty fixed-point observations, preserving their input order.
///
/// # Errors
/// Returns [`ParseError`] for empty input, incorrect dimensions or invalid integers.
pub fn parse(input: &str) -> Result<Puzzle, ParseError> {
    let points = aoc_core::parse_records(input, |line| {
        let mut fields = line.split(',');
        let mut coordinates = [0; 4];
        for coordinate in &mut coordinates {
            *coordinate = fields
                .next()
                .ok_or(PointError::Dimensions)?
                .trim()
                .parse()?;
        }
        if fields.next().is_some() {
            return Err(PointError::Dimensions);
        }
        Ok(Point(coordinates))
    })?;
    Ok(Puzzle { points })
}

#[cfg(test)]
mod tests {
    use super::{ParseError, Point, PointError, parse};

    #[test]
    fn empty_input_is_rejected() {
        assert!(matches!(parse(""), Err(ParseError::Empty)), "no points");
    }

    #[test]
    fn lines_keep_their_order() {
        let puzzle = parse("-1,2,-3,4\n5,-6,7,-8\n").unwrap();
        assert_eq!(
            puzzle.points,
            [Point([-1, 2, -3, 4]), Point([5, -6, 7, -8])],
            "coordinate and point order"
        );
    }

    #[test]
    fn lines_borrow_the_input_and_strip_crlf() {
        let puzzle = parse(" -1, 2, -3, 4\r\n").unwrap();
        assert_eq!(
            puzzle.points,
            [Point([-1, 2, -3, 4])],
            "whitespace and CRLF"
        );
    }

    #[test]
    fn malformed_points_retain_line_and_cause() {
        assert!(
            matches!(
                parse("1,2,3,4\n1,2,3"),
                Err(ParseError::Line {
                    line: 2,
                    source: PointError::Dimensions
                })
            ),
            "missing dimension"
        );
        assert!(
            matches!(
                parse("1,2,3,4,5"),
                Err(ParseError::Line {
                    source: PointError::Dimensions,
                    ..
                })
            ),
            "extra dimension"
        );
        for invalid in ["", "a,2,3,4", "1,,3,4", "2147483648,0,0,0"] {
            assert!(parse(invalid).is_err(), "invalid point {invalid:?}");
        }
    }
}
