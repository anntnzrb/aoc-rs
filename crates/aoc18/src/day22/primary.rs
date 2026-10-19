// approach: cache erosion dependencies; A* searches position/equipment states with admissible distance
// cost: O(A + S log S) time, O(A + S) memory for cached area A and searched states S; one input pass
// trade: grow the cache on demand; no guessed coordinate bound can exclude an optimal route

//! Cave erosion and risk calculation for Advent of Code 2018, day 22.

use aoc_core::Answers;
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
};

use super::{ParseError, Puzzle, parse};

const EROSION_MODULUS: u64 = 20_183;

/// Failure to parse or calculate a cave survey.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// The survey is malformed.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// A coordinate, erosion calculation, or answer exceeds its integer representation.
    #[error("cave calculation exceeds integer range")]
    Overflow,
    /// A required predecessor is missing from the erosion grid.
    #[error("missing erosion predecessor")]
    Grid,
    /// A coordinate cannot be represented in the erosion calculation.
    #[error("coordinate exceeds erosion integer range")]
    Coordinate(#[from] std::num::TryFromIntError),
    /// The survey contradicts the rescue's rocky-target requirement.
    #[error("rescue requires a rocky mouth and target")]
    NonRockyTarget,
    /// No reachable state remains before reaching the target.
    #[error("target is unreachable")]
    Unreachable,
}

struct Cave {
    puzzle: Puzzle,
    erosion: Vec<Vec<u64>>,
    width: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Tool {
    Torch,
    Climbing,
    Neither,
}

impl Tool {
    const fn permits(self, erosion: u64) -> bool {
        match self {
            Self::Torch => erosion % 3 != 1,
            Self::Climbing => erosion % 3 != 2,
            Self::Neither => !erosion.is_multiple_of(3),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct State {
    x: usize,
    y: usize,
    tool: Tool,
}

impl State {
    fn estimate(self, target: (usize, usize)) -> Result<u64, SolveError> {
        let distance = self
            .x
            .abs_diff(target.0)
            .checked_add(self.y.abs_diff(target.1))
            .ok_or(SolveError::Overflow)?;
        u64::try_from(distance)?
            .checked_add(if self.tool == Tool::Torch { 0 } else { 7 })
            .ok_or(SolveError::Overflow)
    }
}

impl Cave {
    const fn new(puzzle: Puzzle) -> Self {
        Self {
            puzzle,
            erosion: Vec::new(),
            width: 0,
        }
    }

    fn get(&self, x: usize, y: usize) -> Result<u64, SolveError> {
        self.erosion
            .get(y)
            .and_then(|row| row.get(x))
            .copied()
            .ok_or(SolveError::Grid)
    }

    fn extend(&mut self, x: usize, y: usize) -> Result<(), SolveError> {
        self.width = self
            .width
            .max(x.checked_add(1).ok_or(SolveError::Overflow)?);
        let height = self
            .erosion
            .len()
            .max(y.checked_add(1).ok_or(SolveError::Overflow)?);
        self.erosion.resize_with(height, Vec::new);
        for row in 0..height {
            let start = self.erosion.get(row).ok_or(SolveError::Grid)?.len();
            for column in start..self.width {
                let index = if (column, row) == (0, 0) || (column, row) == self.puzzle.target {
                    0
                } else if row == 0 {
                    u64::try_from(column)?
                        .checked_mul(16_807)
                        .ok_or(SolveError::Overflow)?
                } else if column == 0 {
                    u64::try_from(row)?
                        .checked_mul(48_271)
                        .ok_or(SolveError::Overflow)?
                } else {
                    let left = self.get(column.checked_sub(1).ok_or(SolveError::Grid)?, row)?;
                    let above = self.get(column, row.checked_sub(1).ok_or(SolveError::Grid)?)?;
                    left.checked_mul(above).ok_or(SolveError::Overflow)?
                };
                let erosion = index
                    .checked_add(self.puzzle.depth % EROSION_MODULUS)
                    .ok_or(SolveError::Overflow)?
                    % EROSION_MODULUS;
                self.erosion
                    .get_mut(row)
                    .ok_or(SolveError::Grid)?
                    .push(erosion);
            }
        }
        Ok(())
    }

    fn risk(&mut self) -> Result<u64, SolveError> {
        self.extend(self.puzzle.target.0, self.puzzle.target.1)?;
        let width = self
            .puzzle
            .target
            .0
            .checked_add(1)
            .ok_or(SolveError::Overflow)?;
        self.erosion
            .iter()
            .take(
                self.puzzle
                    .target
                    .1
                    .checked_add(1)
                    .ok_or(SolveError::Overflow)?,
            )
            .flat_map(|row| row.iter().take(width))
            .try_fold(0_u64, |total, erosion| {
                total.checked_add(erosion % 3).ok_or(SolveError::Overflow)
            })
    }

    fn region(&mut self, x: usize, y: usize) -> Result<u64, SolveError> {
        if x >= self.width || y >= self.erosion.len() {
            // Doubling is cache capacity, not a search boundary. Every requested
            // nonnegative coordinate is generated, including beyond the target.
            let width = if x >= self.width {
                self.width
                    .max(1)
                    .checked_mul(2)
                    .ok_or(SolveError::Overflow)?
            } else {
                self.width
            };
            let height = if y >= self.erosion.len() {
                self.erosion
                    .len()
                    .max(1)
                    .checked_mul(2)
                    .ok_or(SolveError::Overflow)?
            } else {
                self.erosion.len()
            };
            self.extend(
                x.max(width.checked_sub(1).ok_or(SolveError::Overflow)?),
                y.max(height.checked_sub(1).ok_or(SolveError::Overflow)?),
            )?;
        }
        self.get(x, y)
    }

    fn rescue(&mut self) -> Result<u64, SolveError> {
        if !(self.puzzle.depth % EROSION_MODULUS).is_multiple_of(3) {
            return Err(SolveError::NonRockyTarget);
        }
        let start = State {
            x: 0,
            y: 0,
            tool: Tool::Torch,
        };
        let mut distances = HashMap::from([(start, 0_u64)]);
        let mut queue =
            BinaryHeap::from([Reverse((start.estimate(self.puzzle.target)?, 0_u64, start))]);
        // Manhattan distance plus a mandatory final tool switch is admissible.
        // Positive edge costs imply finitely many states precede the finite
        // feasible route, so unbounded A* terminates without a spatial cutoff.
        while let Some(Reverse((_, minutes, state))) = queue.pop() {
            if distances.get(&state) != Some(&minutes) {
                continue;
            }
            if (state.x, state.y) == self.puzzle.target && state.tool == Tool::Torch {
                return Ok(minutes);
            }
            let region = self.region(state.x, state.y)?;
            for tool in [Tool::Torch, Tool::Climbing, Tool::Neither] {
                if tool != state.tool && tool.permits(region) {
                    relax(
                        State { tool, ..state },
                        minutes.checked_add(7).ok_or(SolveError::Overflow)?,
                        self.puzzle.target,
                        &mut distances,
                        &mut queue,
                    )?;
                }
            }
            let neighbors = [
                state.x.checked_sub(1).map(|x| (x, state.y)),
                state.y.checked_sub(1).map(|y| (state.x, y)),
                Some((state.x.checked_add(1).ok_or(SolveError::Overflow)?, state.y)),
                Some((state.x, state.y.checked_add(1).ok_or(SolveError::Overflow)?)),
            ];
            for (x, y) in neighbors.into_iter().flatten() {
                if state.tool.permits(self.region(x, y)?) {
                    relax(
                        State { x, y, ..state },
                        minutes.checked_add(1).ok_or(SolveError::Overflow)?,
                        self.puzzle.target,
                        &mut distances,
                        &mut queue,
                    )?;
                }
            }
        }
        Err(SolveError::Unreachable)
    }
}

fn relax(
    state: State,
    arrival: u64,
    target: (usize, usize),
    distances: &mut HashMap<State, u64>,
    queue: &mut BinaryHeap<Reverse<(u64, u64, State)>>,
) -> Result<(), SolveError> {
    if distances
        .get(&state)
        .is_none_or(|&previous| arrival < previous)
    {
        distances.insert(state, arrival);
        let estimate = arrival
            .checked_add(state.estimate(target)?)
            .ok_or(SolveError::Overflow)?;
        queue.push(Reverse((estimate, arrival, state)));
    }
    Ok(())
}

/// Computes the target rectangle's risk and the minimum rescue time.
///
/// # Errors
/// Returns an error for invalid surveys or unrepresentable calculations.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let mut cave = Cave::new(parse(input)?);
    let risk = cave.risk()?;
    Ok(Answers::new(risk, cave.rescue()?))
}

#[cfg(test)]
mod tests {
    use super::{Cave, parse, solve};

    #[test]
    fn mouth_and_target_override_axis_rules() {
        assert_eq!(
            Cave::new(parse("depth: 2\ntarget: 0,0").unwrap())
                .risk()
                .unwrap(),
            2,
            "single region"
        );
        assert_eq!(
            Cave::new(parse("depth: 0\ntarget: 1,0").unwrap())
                .risk()
                .unwrap(),
            0,
            "target axis override"
        );
        assert_eq!(
            Cave::new(parse("depth: 0\ntarget: 0,1").unwrap())
                .risk()
                .unwrap(),
            0,
            "target vertical override"
        );
    }

    #[test]
    fn extending_preserves_predecessors() {
        let mut cave = Cave::new(parse("depth: 0\ntarget: 2,2").unwrap());
        cave.extend(0, 2).unwrap();
        let predecessor = cave.get(0, 1).unwrap();
        cave.extend(2, 2).unwrap();
        assert_eq!(cave.get(2, 2).unwrap(), 0, "target erosion");
        assert_eq!(
            cave.get(0, 1).unwrap(),
            predecessor,
            "preserved existing column"
        );
        assert!(solve("bad").is_err(), "parser error is propagated");
    }

    #[test]
    fn rescue_handles_mouth_and_axis_targets() {
        for (input, expected) in [
            ("depth: 0\ntarget: 0,0", 0),
            ("depth: 0\ntarget: 1,0", 1),
            ("depth: 0\ntarget: 0,1", 1),
        ] {
            assert_eq!(
                Cave::new(parse(input).unwrap()).rescue().unwrap(),
                expected,
                "short rescue: {input}"
            );
        }
        assert!(
            matches!(
                Cave::new(parse("depth: 1\ntarget: 0,0").unwrap()).rescue(),
                Err(super::SolveError::NonRockyTarget)
            ),
            "invalid rescue geology"
        );
    }

    #[test]
    fn risk_does_not_change_when_rescue_expands_the_grid() {
        let mut cave = Cave::new(parse("depth: 0\ntarget: 0,0").unwrap());
        cave.region(4, 4).unwrap();
        assert_eq!(
            cave.get(0, 0).unwrap(),
            0,
            "mouth remains intact after growth"
        );
        assert_eq!(cave.risk().unwrap(), 0, "risk excludes expanded regions");
    }
}
