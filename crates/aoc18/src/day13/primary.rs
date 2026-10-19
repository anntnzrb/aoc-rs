// approach: simulate carts in reading order, updating occupancy after each move
// cost: O(S + T C log(S C)) time, O(S + T C) memory, one input pass
// trade: sparse ordered maps preserve movement order; repeated states reject noncolliding maps

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
    /// No collision can occur from the repeated state.
    #[error("cart state repeated without a collision")]
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

fn first_collision(mut puzzle: Puzzle) -> Result<Position, SolveError> {
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
        let order: Vec<_> = puzzle.carts.keys().copied().collect();
        for position in order {
            if let Some(mut cart) = puzzle.carts.remove(&position) {
                let destination = advance(position, &mut cart, &puzzle)?;
                if puzzle.carts.contains_key(&destination) {
                    return Ok(destination);
                }
                puzzle.carts.insert(destination, cart);
            }
        }
    }
}

/// Finds the first crash; the unfinished second part renders as zero.
///
/// # Errors
/// Returns [`SolveError`] for invalid maps, derailed carts, or noncolliding cycles.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let (y, x) = first_collision(parse(input)?)?;
    Ok(Answers::new(format!("{x},{y}"), 0))
}

#[cfg(test)]
mod tests {
    use super::{SolveError, first_collision};
    use crate::day13::parse;

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
}
