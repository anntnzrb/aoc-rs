//! Validated rectangular lumber collection areas and their three possible acre states.

pub mod primary;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Acre {
    Open,
    Trees,
    Lumberyard,
}

/// A nonempty rectangular area of acres.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    acres: Vec<Vec<Acre>>,
}

/// Invalid area geometry or acre symbols.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// No acres were supplied.
    #[error("area is empty")]
    Empty,
    /// Rows have different widths.
    #[error("area rows have different widths")]
    Ragged,
    /// Only the three acre symbols are accepted.
    #[error("invalid acre symbol {0:?}")]
    Symbol(char),
}

/// Parses a nonempty rectangular area, accepting LF and CRLF line endings.
///
/// # Errors
/// Returns [`ParseError`] for an empty area, uneven rows, or unknown symbols.
pub fn parse(input: &str) -> Result<Puzzle, ParseError> {
    let mut acres = Vec::new();
    let mut width = None;
    for line in input.lines() {
        if line.is_empty() {
            return Err(ParseError::Empty);
        }
        if width.is_some_and(|expected| expected != line.len()) {
            return Err(ParseError::Ragged);
        }
        width = Some(line.len());
        let row = line
            .chars()
            .map(|symbol| match symbol {
                '.' => Ok(Acre::Open),
                '|' => Ok(Acre::Trees),
                '#' => Ok(Acre::Lumberyard),
                other => Err(ParseError::Symbol(other)),
            })
            .collect::<Result<Vec<_>, _>>()?;
        acres.push(row);
    }
    if acres.is_empty() {
        return Err(ParseError::Empty);
    }
    Ok(Puzzle { acres })
}

#[cfg(test)]
mod tests {
    use super::{Acre, ParseError, parse};

    #[test]
    fn empty_input_is_rejected() {
        assert_eq!(parse(""), Err(ParseError::Empty), "no lines");
    }

    #[test]
    fn lines_keep_their_order() {
        let puzzle = parse(".|\n#.\n").unwrap();
        assert_eq!(
            puzzle.acres,
            [[Acre::Open, Acre::Trees], [Acre::Lumberyard, Acre::Open]],
            "row and column order"
        );
    }

    #[test]
    fn lines_borrow_the_input_and_strip_crlf() {
        assert_eq!(parse(".|\r\n#.\r\n"), parse(".|\n#.\n"), "CRLF");
        assert_eq!(parse(".\n\n.\n"), Err(ParseError::Empty), "empty row");
    }

    #[test]
    fn rejects_invalid_geometry_and_symbols() {
        assert_eq!(parse("..\n.\n"), Err(ParseError::Ragged), "ragged");
        assert_eq!(parse("x"), Err(ParseError::Symbol('x')), "symbol");
    }
}
