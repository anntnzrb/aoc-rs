// approach: move carts in reading order, removing each colliding pair immediately
// cost: O(S log S + T C log(S C T)) time, O(S + T C) memory, one input pass
// trade: separate pending and moved maps prevent removed carts taking turns; detect cycles

//! Reading-order cart simulation with immediate collision detection.

use std::collections::BTreeSet;

use aoc_core::Answers;

use super::{Cart, Direction, ParseError, Position, Puzzle, Track, Turn, parse};

/// Parsing or simulation failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// Invalid map syntax.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// A cart left the mapped track or entered an incompatible straight.
    #[error("cart left its track at {x},{y}")]
    OffTrack {
        /// Column of the cart.
        x: usize,
        /// Row of the cart.
        y: usize,
    },
    /// A coordinate exceeded its representation.
    #[error("cart coordinate overflow")]
    Coordinate,
    /// The repeated state cannot reach the requested collision or survivor.
    #[error("cart state repeated without reaching a terminal outcome")]
    NoCollision,
}

fn advance(position: Position, cart: &mut Cart, puzzle: &Puzzle) -> Result<Position, SolveError> {
    let (y, x) = position;
    let destination = match cart.direction {
        Direction::Up => y.checked_sub(1).map(|next| (next, x)),
        Direction::Right => x.checked_add(1).map(|next| (y, next)),
        Direction::Down => y.checked_add(1).map(|next| (next, x)),
        Direction::Left => x.checked_sub(1).map(|next| (y, next)),
    }
    .ok_or(SolveError::Coordinate)?;
    let (destination_y, destination_x) = destination;
    match puzzle.tracks.get(&destination) {
        Some(Track::Horizontal) if matches!(cart.direction, Direction::Left | Direction::Right) => {
        }
        Some(Track::Vertical) if matches!(cart.direction, Direction::Up | Direction::Down) => {}
        Some(Track::Slash) => {
            cart.direction = match cart.direction {
                Direction::Up => Direction::Right,
                Direction::Right => Direction::Up,
                Direction::Down => Direction::Left,
                Direction::Left => Direction::Down,
            };
        }
        Some(Track::Backslash) => {
            cart.direction = match cart.direction {
                Direction::Up => Direction::Left,
                Direction::Right => Direction::Down,
                Direction::Down => Direction::Right,
                Direction::Left => Direction::Up,
            };
        }
        Some(Track::Intersection) => {
            (cart.direction, cart.turn) = match cart.turn {
                Turn::Left => (cart.direction.left(), Turn::Straight),
                Turn::Straight => (cart.direction, Turn::Right),
                Turn::Right => (cart.direction.right(), Turn::Left),
            };
        }
        Some(Track::Horizontal | Track::Vertical) | None => {
            return Err(SolveError::OffTrack {
                x: destination_x,
                y: destination_y,
            });
        }
    }
    Ok(destination)
}

fn simulate(mut puzzle: Puzzle) -> Result<(Position, Option<Position>), SolveError> {
    let mut first = None;
    let mut seen = BTreeSet::new();
    loop {
        if !seen.insert(
            puzzle
                .carts
                .iter()
                .map(|(&position, &cart)| (position, cart))
                .collect::<Vec<_>>(),
        ) {
            return Err(SolveError::NoCollision);
        }
        let mut pending = std::mem::take(&mut puzzle.carts);
        while let Some((position, mut cart)) = pending.pop_first() {
            let destination = advance(position, &mut cart, &puzzle)?;
            if pending.remove(&destination).is_some() || puzzle.carts.remove(&destination).is_some()
            {
                first.get_or_insert(destination);
            } else {
                puzzle.carts.insert(destination, cart);
            }
        }
        if puzzle.carts.len() <= 1 {
            return Ok((
                first.ok_or(SolveError::NoCollision)?,
                puzzle
                    .carts
                    .first_key_value()
                    .map(|(&position, _)| position),
            ));
        }
    }
}

/// Finds the first crash and the last cart after a completed tick.
///
/// Inputs where every cart crashes render only part one.
///
/// # Errors
/// Returns [`SolveError`] for invalid maps, derailed carts, or nonterminating cycles.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let ((y, x), survivor) = simulate(parse(input)?)?;
    let first = format!("{x},{y}");
    Ok(survivor.map_or_else(
        || Answers::part1_only(&first),
        |(last_y, last_x)| Answers::new(&first, format!("{last_x},{last_y}")),
    ))
}

#[cfg(test)]
mod tests {
    use super::SolveError;
    use crate::day13::parse;

    fn first_collision(puzzle: crate::day13::Puzzle) -> Result<crate::day13::Position, SolveError> {
        super::simulate(puzzle).map(|(first, _)| first)
    }

    #[test]
    fn detects_collision_with_unmoved_cart() {
        assert_eq!(
            first_collision(parse("><").unwrap()).unwrap(),
            (0, 1),
            "head-on collision precedes second movement"
        );
    }

    #[test]
    fn detects_collision_with_moved_cart() {
        assert_eq!(
            first_collision(parse(">-<").unwrap()).unwrap(),
            (0, 1),
            "second cart hits first cart"
        );
    }

    #[test]
    fn rejects_derailment() {
        assert!(
            matches!(
                first_collision(parse(">").unwrap()),
                Err(SolveError::OffTrack { x: 1, y: 0 })
            ),
            "missing track is an error"
        );
    }

    #[test]
    fn rejects_noncolliding_loop() {
        assert!(
            matches!(
                first_collision(parse("/>\\\n| |\n\\-/\n").unwrap()),
                Err(SolveError::NoCollision)
            ),
            "finite state repetition terminates"
        );
    }

    #[test]
    fn last_cart_finishes_its_tick_after_a_collision() {
        let result = super::simulate(parse(">-< >-").unwrap()).unwrap();
        assert_eq!(
            result,
            ((0, 1), Some((0, 5))),
            "survivor moves even when collision occurs earlier in the tick"
        );
    }

    #[test]
    fn removed_cart_does_not_move_a_later_arrival_twice() {
        let result = super::simulate(parse(" v \n>+<\n ^ ").unwrap()).unwrap();
        assert_eq!(
            result,
            ((1, 1), None),
            "third cart may enter cleared crash cell before fourth crashes into it"
        );
    }
}
