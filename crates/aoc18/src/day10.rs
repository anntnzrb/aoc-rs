//! Validated moving lights with integer positions and velocities.

pub mod primary;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Light {
    position: (i32, i32),
    velocity: (i32, i32),
}

/// Nonempty moving-light observations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    lights: Vec<Light>,
}

/// Malformed position or velocity pair.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LightError {
    /// Incorrect observation grammar.
    #[error("expected position=<x, y> velocity=<vx, vy>")]
    Syntax,
    /// Invalid signed coordinate.
    #[error("invalid light coordinate")]
    Integer(#[from] std::num::ParseIntError),
}

/// Input failure retaining observation line and cause.
pub type ParseError = aoc_core::RecordsError<LightError>;

fn pair(text: &str) -> Result<(i32, i32), LightError> {
    let (x, y) = text.split_once(',').ok_or(LightError::Syntax)?;
    Ok((x.trim().parse()?, y.trim().parse()?))
}

/// Parses position and velocity observations in input order.
///
/// # Errors
/// Returns [`ParseError`] for empty input or malformed observations.
pub fn parse(input: &str) -> Result<Puzzle, ParseError> {
    let lights = aoc_core::parse_records(input, |line| {
        let text = line
            .strip_prefix("position=<")
            .and_then(|text| text.strip_suffix('>'))
            .ok_or(LightError::Syntax)?;
        let (position, velocity) = text.split_once("> velocity=<").ok_or(LightError::Syntax)?;
        Ok(Light {
            position: pair(position)?,
            velocity: pair(velocity)?,
        })
    })?;
    Ok(Puzzle { lights })
}

#[cfg(test)]
mod tests {
    use super::{ParseError, parse};

    #[test]
    fn empty_input_is_rejected() {
        assert!(
            matches!(parse(""), Err(ParseError::Empty)),
            "missing observations"
        );
    }

    #[test]
    fn lines_keep_their_order() {
        let puzzle =
            parse("position=<-2, 3> velocity=< 4, -5>\nposition=<1, 2> velocity=<0, 0>\n").unwrap();
        assert_eq!(puzzle.lights[0].position, (-2, 3), "signed position");
        assert_eq!(puzzle.lights[0].velocity, (4, -5), "signed velocity");
        assert_eq!(puzzle.lights[1].position, (1, 2), "observation order");
        for invalid in [
            "position= velocity=",
            "position=<1, 2> velocity=<3>",
            "position=<1, 2> velocity=<3, 4> extra",
        ] {
            assert!(parse(invalid).is_err(), "invalid observation {invalid}");
        }
    }

    #[test]
    fn lines_borrow_the_input_and_strip_crlf() {
        assert_eq!(
            parse("position=<1, 2> velocity=<3, 4>\r\n").unwrap().lights[0].velocity,
            (3, 4),
            "CRLF terminator"
        );
    }
}
