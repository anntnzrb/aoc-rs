// approach: descend from the spring; spread on support and revisit water above newly settled rows
// cost: O(C + W squared) worst-case time, O(C + W) memory, one input pass; C clay, W wet tiles
// trade: sparse cells avoid coordinate-sized grids; an explicit work stack avoids recursive depth

//! Iterative gravity-driven water propagation through a sparse clay scan.

use std::collections::HashMap;

use aoc_core::Answers;

use super::{Axis, ParseError, Puzzle, parse};

type Point = (i64, i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cell {
    Clay,
    Flowing,
    Settled,
}

/// Failure to parse or simulate the scan.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// Invalid clay scan.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// Coordinate movement exceeded the simulation domain.
    #[error("water coordinate overflow")]
    Coordinate,
    /// A nonempty scan unexpectedly contained no clay.
    #[error("scan contains no clay")]
    Empty,
}

struct Ground {
    cells: HashMap<Point, Cell>,
    min_y: i64,
    max_y: i64,
}

impl Ground {
    fn new(puzzle: &Puzzle) -> Result<Self, SolveError> {
        let mut cells = HashMap::new();
        let mut min_y = i64::MAX;
        let mut max_y = i64::MIN;
        for vein in &puzzle.veins {
            for coordinate in vein.start..=vein.end {
                let fixed = i64::from(vein.fixed);
                let coordinate = i64::from(coordinate);
                let point = match vein.axis {
                    Axis::X => (fixed, coordinate),
                    Axis::Y => (coordinate, fixed),
                };
                cells.insert(point, Cell::Clay);
                min_y = min_y.min(point.1);
                max_y = max_y.max(point.1);
            }
        }
        if cells.is_empty() {
            return Err(SolveError::Empty);
        }
        Ok(Self {
            cells,
            min_y,
            max_y,
        })
    }

    fn supported(&self, point: Point) -> bool {
        matches!(self.cells.get(&point), Some(Cell::Clay | Cell::Settled))
    }

    fn edge(&mut self, (mut x, y): Point, direction: i64) -> Result<Edge, SolveError> {
        loop {
            self.cells.insert((x, y), Cell::Flowing);
            let below = (x, y.checked_add(1).ok_or(SolveError::Coordinate)?);
            if !self.supported(below) {
                return Ok(Edge::Spill(x));
            }
            let next = x.checked_add(direction).ok_or(SolveError::Coordinate)?;
            if self.supported((next, y)) {
                return Ok(Edge::Wall(x));
            }
            x = next;
        }
    }

    fn fill(&mut self) -> Result<(), SolveError> {
        let mut pending = vec![(500, 0)];
        while let Some((x, mut y)) = pending.pop() {
            if self.supported((x, y)) {
                continue;
            }
            loop {
                if y > self.max_y {
                    break;
                }
                self.cells.insert((x, y), Cell::Flowing);
                let below_y = y.checked_add(1).ok_or(SolveError::Coordinate)?;
                if self.supported((x, below_y)) {
                    let left = self.edge((x, y), -1)?;
                    let right = self.edge((x, y), 1)?;
                    if let (Edge::Wall(left), Edge::Wall(right)) = (left, right) {
                        let above_y = y.checked_sub(1).ok_or(SolveError::Coordinate)?;
                        for column in left..=right {
                            self.cells.insert((column, y), Cell::Settled);
                            if above_y >= 0
                                && self.cells.get(&(column, above_y)) == Some(&Cell::Flowing)
                            {
                                pending.push((column, above_y));
                            }
                        }
                    } else {
                        for edge in [left, right] {
                            if let Edge::Spill(column) = edge {
                                let below = (column, below_y);
                                if !self.cells.contains_key(&below) {
                                    pending.push(below);
                                }
                            }
                        }
                    }
                    break;
                }
                if self.cells.get(&(x, below_y)) == Some(&Cell::Flowing) {
                    break;
                }
                y = below_y;
            }
        }
        Ok(())
    }

    fn reachable(&self) -> usize {
        self.cells
            .iter()
            .filter(|((_, y), cell)| {
                (self.min_y..=self.max_y).contains(y)
                    && matches!(cell, Cell::Flowing | Cell::Settled)
            })
            .count()
    }
}

#[derive(Clone, Copy)]
enum Edge {
    Wall(i64),
    Spill(i64),
}

/// Counts reachable water; the unfinished second part renders as zero.
///
/// # Errors
/// Returns [`SolveError`] for invalid scans or coordinate overflow.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let puzzle = parse(input)?;
    let mut ground = Ground::new(&puzzle)?;
    ground.fill()?;
    Ok(Answers::new(ground.reachable(), 0))
}

#[cfg(test)]
mod tests {
    use super::{Cell, Ground, solve};
    use crate::day17::parse;

    #[test]
    fn falling_water_respects_counted_y_range() {
        let answer = solve("x=700, y=2..4\n").unwrap();
        assert_eq!(
            answer,
            aoc_core::Answers::new(3, 0),
            "three counted falling tiles"
        );
    }

    #[test]
    fn closed_cup_settles_and_overflows_both_sides() {
        let mut ground =
            Ground::new(&parse("x=499, y=1..3\nx=501, y=1..3\ny=3, x=499..501\n").unwrap())
                .unwrap();
        ground.fill().unwrap();
        assert_eq!(
            ground.reachable(),
            8,
            "two stored and six exterior falling tiles"
        );
        assert_eq!(
            ground.cells.get(&(500, 1)),
            Some(&Cell::Settled),
            "cup fills upward"
        );
        assert_eq!(
            ground.cells.get(&(498, 3)),
            Some(&Cell::Flowing),
            "left overflow"
        );
        assert_eq!(
            ground.cells.get(&(502, 3)),
            Some(&Cell::Flowing),
            "right overflow"
        );
    }

    #[test]
    fn deep_fall_uses_no_recursive_calls() {
        assert_eq!(
            solve("x=700, y=1..100000\n").unwrap(),
            aoc_core::Answers::new(100_000, 0),
            "deep scan"
        );
    }
}
