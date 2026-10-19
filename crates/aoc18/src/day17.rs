//! Clay veins parsed into validated horizontal and vertical scan segments.

pub mod primary;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    X,
    Y,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Vein {
    axis: Axis,
    fixed: u32,
    start: u32,
    end: u32,
}

/// Nonempty clay scan with ordered inclusive vein endpoints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    veins: Vec<Vein>,
}

/// A malformed clay vein.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum VeinError {
    /// Missing axes, separators, or range endpoints.
    #[error("expected x=n, y=a..b or y=n, x=a..b")]
    Syntax,
    /// Coordinate outside the unsigned 32-bit scan domain.
    #[error("invalid scan coordinate")]
    Integer(#[from] std::num::ParseIntError),
    /// Inclusive endpoints are reversed.
    #[error("vein range is reversed")]
    Reversed,
}

/// Input failure retaining the offending line and its cause.
pub type ParseError = aoc_core::RecordsError<VeinError>;

fn vein(line: &str) -> Result<Vein, VeinError> {
    let (fixed, range) = line.split_once(", ").ok_or(VeinError::Syntax)?;
    let (axis, fixed, range) = if let Some(fixed) = fixed.strip_prefix("x=") {
        (Axis::X, fixed, range.strip_prefix("y="))
    } else if let Some(fixed) = fixed.strip_prefix("y=") {
        (Axis::Y, fixed, range.strip_prefix("x="))
    } else {
        return Err(VeinError::Syntax);
    };
    let (start, end) = range
        .ok_or(VeinError::Syntax)?
        .split_once("..")
        .ok_or(VeinError::Syntax)?;
    let result = Vein {
        axis,
        fixed: fixed.parse()?,
        start: start.parse()?,
        end: end.parse()?,
    };
    if result.start > result.end {
        return Err(VeinError::Reversed);
    }
    Ok(result)
}

/// Parses the scan's nonempty sequence of clay veins.
///
/// # Errors
/// Returns [`ParseError`] for missing records, malformed veins, or reversed ranges.
pub fn parse(input: &str) -> Result<Puzzle, ParseError> {
    Ok(Puzzle {
        veins: aoc_core::parse_records(input, vein)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{Axis, ParseError, parse};

    #[test]
    fn empty_input_is_rejected() {
        assert!(matches!(parse(""), Err(ParseError::Empty)), "empty scan");
    }

    #[test]
    fn lines_keep_their_order() {
        let puzzle = parse("x=0, y=1..2\ny=3, x=4..4\n").unwrap();
        assert_eq!(puzzle.veins[0].axis, Axis::X, "vertical first");
        assert_eq!(puzzle.veins[1].axis, Axis::Y, "horizontal second");
        assert_eq!(puzzle.veins[1].start, 4, "singleton range");
        for invalid in [
            "x=2, x=1..3",
            "y=1, x=3..2",
            "x=-1, y=0..2",
            "x=4294967296, y=0..2",
            "x=1, y=1..2 trailing",
            "x=1, y=1",
            "",
        ] {
            assert!(parse(invalid).is_err(), "invalid vein {invalid}");
        }
    }

    #[test]
    fn lines_borrow_the_input_and_strip_crlf() {
        let puzzle = parse("y=0, x=0..4294967295\r\n").unwrap();
        assert_eq!(puzzle.veins[0].end, u32::MAX, "CRLF and endpoint");
        assert!(parse("x=1, y=1..2\n\ny=3, x=0..1").is_err(), "blank vein");
    }
}
