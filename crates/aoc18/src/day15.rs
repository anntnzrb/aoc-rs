//! Beverage Bandits: validated cavern terrain and initial combatants.

use std::collections::{BTreeMap, BTreeSet};

pub mod primary;

type Position = (i32, i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Faction {
    Elf,
    Goblin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Unit {
    id: usize,
    faction: Faction,
    hp: u32,
}

/// Validated open terrain and initial combatants in reading order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    floor: BTreeSet<Position>,
    units: BTreeMap<Position, Unit>,
}

/// Invalid cavern input.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseError {
    /// No map was supplied.
    #[error("empty cavern map")]
    Empty,
    /// Rows must have equal, nonzero widths.
    #[error("cavern rows have inconsistent widths")]
    Width,
    /// A cell was not a wall, floor, elf, or goblin.
    #[error("invalid cavern cell {0:?}")]
    Cell(char),
    /// Coordinates exceed the representation.
    #[error("cavern coordinates exceed i32")]
    Coordinate(#[from] std::num::TryFromIntError),
    /// Combat requires at least one unit.
    #[error("cavern contains no units")]
    NoUnits,
}

/// Parses a rectangular map, accepting either line-ending convention.
///
/// # Errors
/// Returns [`ParseError`] for empty, malformed, oversized, or unitless maps.
pub fn parse(input: &str) -> Result<Puzzle, ParseError> {
    let width = input.lines().next().ok_or(ParseError::Empty)?.len();
    if width == 0 {
        return Err(ParseError::Empty);
    }
    let mut floor = BTreeSet::new();
    let mut units = BTreeMap::new();
    for (y, line) in input.lines().enumerate() {
        if line.len() != width {
            return Err(ParseError::Width);
        }
        for (x, cell) in line.chars().enumerate() {
            let position = (i32::try_from(y)?, i32::try_from(x)?);
            match cell {
                '#' => {}
                '.' | 'E' | 'G' => {
                    floor.insert(position);
                    if cell != '.' {
                        units.insert(
                            position,
                            Unit {
                                id: units.len(),
                                faction: if cell == 'E' {
                                    Faction::Elf
                                } else {
                                    Faction::Goblin
                                },
                                hp: 200,
                            },
                        );
                    }
                }
                other => return Err(ParseError::Cell(other)),
            }
        }
    }
    if units.is_empty() {
        return Err(ParseError::NoUnits);
    }
    Ok(Puzzle { floor, units })
}

fn neighbors((y, x): Position) -> impl Iterator<Item = Position> {
    [
        y.checked_sub(1).map(|v| (v, x)),
        x.checked_sub(1).map(|v| (y, v)),
        x.checked_add(1).map(|v| (y, v)),
        y.checked_add(1).map(|v| (v, x)),
    ]
    .into_iter()
    .flatten()
}

#[cfg(test)]
mod tests {
    use super::{ParseError, parse};

    #[test]
    fn empty_input_is_rejected() {
        assert_eq!(parse(""), Err(ParseError::Empty), "no rows");
    }

    #[test]
    fn lines_keep_their_order() {
        let puzzle = parse("G.E\n...").unwrap();
        assert_eq!(
            puzzle.units.keys().copied().collect::<Vec<_>>(),
            [(0, 0), (0, 2)],
            "reading order"
        );
        assert_eq!(parse("E.\nG"), Err(ParseError::Width), "ragged rows");
        assert_eq!(parse("E?"), Err(ParseError::Cell('?')), "invalid cell");
        assert_eq!(parse(".."), Err(ParseError::NoUnits), "unitless map");
    }

    #[test]
    fn lines_borrow_the_input_and_strip_crlf() {
        assert_eq!(parse("EG\r\n..\r\n"), parse("EG\n..\n"), "line endings");
    }
}
