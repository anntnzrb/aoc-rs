//! Validated marble game parameters.

use std::num::NonZeroUsize;

pub mod primary;

/// Number of players and final marble value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Puzzle {
    players: NonZeroUsize,
    last_marble: u64,
}

/// Malformed marble game description.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// No game description.
    #[error("empty game description")]
    Empty,
    /// Wrong description grammar or zero players.
    #[error("expected positive players and a last marble value")]
    Syntax,
    /// Invalid unsigned integer.
    #[error("invalid game parameter")]
    Integer(#[from] std::num::ParseIntError),
}

/// Parses one game using the statement's description grammar.
///
/// # Errors
/// Returns [`ParseError`] for malformed input or zero players.
pub fn parse(input: &str) -> Result<Puzzle, ParseError> {
    let text = input.trim_end();
    if text.is_empty() {
        return Err(ParseError::Empty);
    }
    let text = text.strip_suffix(" points").ok_or(ParseError::Syntax)?;
    let (players, last) = text
        .split_once(" players; last marble is worth ")
        .ok_or(ParseError::Syntax)?;
    Ok(Puzzle {
        players: NonZeroUsize::new(players.parse()?).ok_or(ParseError::Syntax)?,
        last_marble: last.parse()?,
    })
}

#[cfg(test)]
mod tests {
    use super::{ParseError, parse};

    #[test]
    fn empty_input_is_rejected() {
        assert!(matches!(parse(""), Err(ParseError::Empty)), "missing game");
    }

    #[test]
    fn lines_keep_their_order() {
        let game = parse("2 players; last marble is worth 0 points").unwrap();
        assert_eq!(
            (game.players.get(), game.last_marble),
            (2, 0),
            "parameters retain their roles"
        );
        for invalid in [
            "0 players; last marble is worth 1 points",
            "2 players; last marble is worth -1 points",
            "2 3",
            "2 players; last marble is worth 1 points extra",
        ] {
            assert!(parse(invalid).is_err(), "invalid game {invalid}");
        }
    }

    #[test]
    fn lines_borrow_the_input_and_strip_crlf() {
        assert_eq!(
            parse("1 players; last marble is worth 4 points\r\n")
                .unwrap()
                .last_marble,
            4,
            "CRLF terminator"
        );
    }
}
