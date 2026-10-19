//! Validated integer nanobot positions and nonnegative signal radii.

pub mod primary;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Nanobot {
    position: [i128; 3],
    radius: u128,
}

/// A nonempty collection of nanobot observations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    bots: Vec<Nanobot>,
}

/// A malformed nanobot observation.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BotError {
    /// The observation does not follow the expected grammar.
    #[error("expected pos=<x,y,z>, r=radius")]
    Syntax,
    /// A coordinate or radius is not a supported integer.
    #[error("invalid nanobot integer")]
    Integer(#[from] std::num::ParseIntError),
}

/// Input error retaining the observation line and underlying cause.
pub type ParseError = aoc_core::RecordsError<BotError>;

/// Parses signed 64-bit coordinates and unsigned 64-bit radii.
///
/// # Errors
/// Returns an error for empty input, malformed syntax or invalid integers.
pub fn parse(input: &str) -> Result<Puzzle, ParseError> {
    let bots = aoc_core::parse_records(input, |line| {
        let (position_text, radius) = line
            .strip_prefix("pos=<")
            .and_then(|text| text.split_once(">, r="))
            .ok_or(BotError::Syntax)?;
        let mut coordinates = position_text.split(',');
        let mut next_coordinate = || -> Result<i128, BotError> {
            Ok(i128::from(
                coordinates.next().ok_or(BotError::Syntax)?.parse::<i64>()?,
            ))
        };
        let position = [next_coordinate()?, next_coordinate()?, next_coordinate()?];
        if coordinates.next().is_some() {
            return Err(BotError::Syntax);
        }
        Ok(Nanobot {
            position,
            radius: u128::from(radius.parse::<u64>()?),
        })
    })?;
    Ok(Puzzle { bots })
}

#[cfg(test)]
mod tests {
    use super::{ParseError, parse};

    #[test]
    fn empty_input_is_rejected() {
        assert!(matches!(parse(""), Err(ParseError::Empty)), "no bots");
    }

    #[test]
    fn lines_keep_their_order() {
        let puzzle = parse("pos=<-2,3,4>, r=0\npos=<1,2,3>, r=5\n").unwrap();
        assert_eq!(puzzle.bots[0].position, [-2, 3, 4], "signed coordinates");
        assert_eq!(puzzle.bots[1].radius, 5, "observation order");
        for invalid in [
            "pos=<1,2>, r=3",
            "pos=<1,2,3,4>, r=3",
            "pos=<1,2,3>, r=-1",
            "pos=<1,2,3>, r=3 trailing",
            "pos=<9223372036854775808,0,0>, r=0",
            "pos=<a,2,3>, r=3",
        ] {
            assert!(parse(invalid).is_err(), "invalid observation {invalid}");
        }
    }

    #[test]
    fn lines_borrow_the_input_and_strip_crlf() {
        let puzzle = parse("pos=<1,2,3>, r=0\r\n").unwrap();
        assert_eq!(puzzle.bots[0].position, [1, 2, 3], "CRLF terminator");
        assert!(parse("pos=<1,2,3>, r=0\n\n").is_err(), "blank record");
    }
}
