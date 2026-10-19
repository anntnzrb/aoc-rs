//! Track geometry, cart headings, and whitespace-preserving map parsing.

use std::collections::BTreeMap;

pub mod primary;

type Position = (usize, usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Track {
    Horizontal,
    Vertical,
    Slash,
    Backslash,
    Intersection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Direction {
    Up,
    Right,
    Down,
    Left,
}

impl Direction {
    const fn left(self) -> Self {
        match self {
            Self::Up => Self::Left,
            Self::Right => Self::Up,
            Self::Down => Self::Right,
            Self::Left => Self::Down,
        }
    }

    const fn right(self) -> Self {
        match self {
            Self::Up => Self::Right,
            Self::Right => Self::Down,
            Self::Down => Self::Left,
            Self::Left => Self::Up,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Turn {
    Left,
    Straight,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Cart {
    direction: Direction,
    turn: Turn,
}

/// A sparse track map and the carts initially occupying it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    tracks: BTreeMap<Position, Track>,
    carts: BTreeMap<Position, Cart>,
}

/// Invalid map syntax.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// No carts were present.
    #[error("map contains no carts")]
    Empty,
    /// A map cell was not a track, cart, or space.
    #[error("invalid map character {character:?} at {x},{y}")]
    Character {
        /// Column of the invalid cell.
        x: usize,
        /// Row of the invalid cell.
        y: usize,
        /// Unexpected character.
        character: char,
    },
}

/// Parses a map without trimming its significant leading spaces.
///
/// # Errors
/// Returns [`ParseError`] for unknown characters or a map without carts.
pub fn parse(input: &str) -> Result<Puzzle, ParseError> {
    let mut tracks = BTreeMap::new();
    let mut carts = BTreeMap::new();
    for (y, line) in input.lines().enumerate() {
        for (x, character) in line.chars().enumerate() {
            let (track, direction) = match character {
                ' ' => continue,
                '-' => (Track::Horizontal, None),
                '|' => (Track::Vertical, None),
                '/' => (Track::Slash, None),
                '\\' => (Track::Backslash, None),
                '+' => (Track::Intersection, None),
                '^' => (Track::Vertical, Some(Direction::Up)),
                'v' => (Track::Vertical, Some(Direction::Down)),
                '<' => (Track::Horizontal, Some(Direction::Left)),
                '>' => (Track::Horizontal, Some(Direction::Right)),
                _ => return Err(ParseError::Character { x, y, character }),
            };
            tracks.insert((y, x), track);
            if let Some(direction) = direction {
                carts.insert(
                    (y, x),
                    Cart {
                        direction,
                        turn: Turn::Left,
                    },
                );
            }
        }
    }
    if carts.is_empty() {
        return Err(ParseError::Empty);
    }
    Ok(Puzzle { tracks, carts })
}

#[cfg(test)]
mod tests {
    use super::{ParseError, Track, parse};

    #[test]
    fn empty_input_is_rejected() {
        assert_eq!(parse(""), Err(ParseError::Empty), "no carts");
    }

    #[test]
    fn lines_keep_their_order() {
        let puzzle = parse("  >\n v\n").unwrap();
        assert_eq!(
            puzzle.carts.keys().copied().collect::<Vec<_>>(),
            [(0, 2), (1, 1)],
            "leading spaces preserve coordinates"
        );
    }

    #[test]
    fn lines_borrow_the_input_and_strip_crlf() {
        let puzzle = parse(" >\r\n\r\nv\r\n").unwrap();
        assert_eq!(
            puzzle.tracks.get(&(0, 1)),
            Some(&Track::Horizontal),
            "cart hides horizontal track"
        );
        assert_eq!(
            puzzle.tracks.get(&(2, 0)),
            Some(&Track::Vertical),
            "blank row and CRLF retained correctly"
        );
    }

    #[test]
    fn unknown_cells_are_rejected() {
        assert_eq!(
            parse(" >?"),
            Err(ParseError::Character {
                x: 2,
                y: 0,
                character: '?'
            }),
            "invalid cell has coordinates"
        );
    }
}
