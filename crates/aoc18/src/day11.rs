//! Fuel-grid serial number parsing for Advent of Code 2018, day 11.

pub mod primary;

/// A nonnegative grid serial number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Puzzle {
    serial: u32,
}

/// Failure to parse a grid serial number.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// The input contains no serial number.
    #[error("missing grid serial number")]
    Empty,
    /// The input is not one nonnegative integer.
    #[error("invalid grid serial number")]
    Serial(#[from] std::num::ParseIntError),
}

/// Parses a single nonnegative grid serial number.
///
/// # Errors
/// Returns [`ParseError`] for empty input or an invalid integer.
pub fn parse(input: &str) -> Result<Puzzle, ParseError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(ParseError::Empty);
    }
    Ok(Puzzle {
        serial: input.parse()?,
    })
}

#[cfg(test)]
mod tests {
    use super::{ParseError, parse};

    #[test]
    fn empty_input_is_rejected() {
        assert!(matches!(parse(""), Err(ParseError::Empty)), "no serial");
    }

    #[test]
    fn lines_keep_their_order() {
        assert!(
            matches!(parse("2\n1\n"), Err(ParseError::Serial(_))),
            "one serial only"
        );
    }

    #[test]
    fn lines_borrow_the_input_and_strip_crlf() {
        assert_eq!(parse("17\r\n").unwrap().serial, 17, "CRLF serial");
    }

    #[test]
    fn serial_boundaries_and_invalid_values() {
        assert_eq!(parse("0").unwrap().serial, 0, "zero serial");
        assert_eq!(
            parse("4294967295").unwrap().serial,
            u32::MAX,
            "largest serial"
        );
        for input in ["-1", "4294967296", "serial", "1 2"] {
            assert!(
                matches!(parse(input), Err(ParseError::Serial(_))),
                "reject {input}"
            );
        }
    }
}
