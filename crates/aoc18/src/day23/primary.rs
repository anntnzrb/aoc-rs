// approach: exact best-first cube subdivision with coverage upper bounds and distance lower bounds.
// cost: O(n V + V log V) time, O(n + V) memory for V visited cubes; two input passes.
// trade: no input-shape assumptions; loose bounds can require exponentially many cubes.

//! Strongest-transmitter coverage and exact closest maximum-overlap integer position.

use std::{cmp::Reverse, collections::BinaryHeap};

use aoc_core::Answers;

use super::{Nanobot, ParseError, parse};

/// Failure to parse observations or represent a geometric calculation.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// Invalid input observations.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// A geometric integer could not be converted to its working representation.
    #[error("nanobot geometry conversion failed")]
    Conversion(#[from] std::num::TryFromIntError),
    /// An intermediate geometric value exceeded its representation.
    #[error("nanobot geometry overflow")]
    Overflow,
    /// The parsed observations unexpectedly contained no nanobots.
    #[error("missing nanobots")]
    Empty,
}

fn distance(left: [i128; 3], right: [i128; 3]) -> Result<u128, SolveError> {
    left.into_iter().zip(right).try_fold(0_u128, |sum, (a, b)| {
        sum.checked_add(a.abs_diff(b)).ok_or(SolveError::Overflow)
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Cube {
    low: [i128; 3],
    size: i128,
}

impl Cube {
    fn distance(self, point: [i128; 3]) -> Result<u128, SolveError> {
        let offset = self.size.checked_sub(1).ok_or(SolveError::Overflow)?;
        self.low
            .into_iter()
            .zip(point)
            .try_fold(0_u128, |sum, (low, coordinate)| {
                let high = low.checked_add(offset).ok_or(SolveError::Overflow)?;
                sum.checked_add(coordinate.abs_diff(coordinate.clamp(low, high)))
                    .ok_or(SolveError::Overflow)
            })
    }

    fn coverage(self, bots: &[Nanobot]) -> Result<usize, SolveError> {
        bots.iter().try_fold(0_usize, |count, bot| {
            if self.distance(bot.position)? <= bot.radius {
                count.checked_add(1).ok_or(SolveError::Overflow)
            } else {
                Ok(count)
            }
        })
    }
}

fn enclosing_cube(bots: &[Nanobot]) -> Result<Cube, SolveError> {
    let mut low = [i128::MAX; 3];
    let mut high = [i128::MIN; 3];
    for bot in bots {
        let radius = i128::try_from(bot.radius)?;
        for ((minimum, maximum), coordinate) in low.iter_mut().zip(&mut high).zip(bot.position) {
            *minimum = (*minimum).min(coordinate.checked_sub(radius).ok_or(SolveError::Overflow)?);
            *maximum = (*maximum).max(coordinate.checked_add(radius).ok_or(SolveError::Overflow)?);
        }
    }
    let mut size = 1_i128;
    for (minimum, maximum) in low.into_iter().zip(high) {
        let span = maximum
            .checked_sub(minimum)
            .and_then(|difference| difference.checked_add(1))
            .ok_or(SolveError::Overflow)?;
        while size < span {
            size = size.checked_mul(2).ok_or(SolveError::Overflow)?;
        }
    }
    Ok(Cube { low, size })
}

fn closest(bots: &[Nanobot]) -> Result<u128, SolveError> {
    let root = enclosing_cube(bots)?;
    let mut pending = BinaryHeap::from([(
        root.coverage(bots)?,
        Reverse(root.distance([0; 3])?),
        Reverse(root.size),
        root,
    )]);
    // Every positive-coverage point is in the root. A cube's intersecting-bot
    // count is an upper bound, and its origin distance is a lower bound.
    // Thus the first singleton popped is globally optimal in both objectives.
    while let Some((_, Reverse(origin_distance), _, cube)) = pending.pop() {
        if cube.size == 1 {
            return Ok(origin_distance);
        }
        let half = cube.size.checked_div(2).ok_or(SolveError::Overflow)?;
        for x in [0, half] {
            for y in [0, half] {
                for z in [0, half] {
                    let mut low = cube.low;
                    for (coordinate, offset) in low.iter_mut().zip([x, y, z]) {
                        *coordinate = coordinate.checked_add(offset).ok_or(SolveError::Overflow)?;
                    }
                    let child = Cube { low, size: half };
                    let coverage = child.coverage(bots)?;
                    if coverage != 0 {
                        pending.push((
                            coverage,
                            Reverse(child.distance([0; 3])?),
                            Reverse(half),
                            child,
                        ));
                    }
                }
            }
        }
    }
    Err(SolveError::Empty)
}

/// Counts strongest-transmitter coverage and finds the closest maximum-overlap point.
///
/// # Errors
/// Returns an error for malformed input or unrepresentable geometry.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let puzzle = parse(input)?;
    let strongest = puzzle
        .bots
        .iter()
        .max_by_key(|bot| bot.radius)
        .ok_or(SolveError::Empty)?;
    let mut count = 0_usize;
    for bot in &puzzle.bots {
        if distance(bot.position, strongest.position)? <= strongest.radius {
            count = count.checked_add(1).ok_or(SolveError::Overflow)?;
        }
    }
    Ok(Answers::new(count, closest(&puzzle.bots)?))
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;

    use super::solve;

    #[test]
    fn includes_self_and_signal_boundary() {
        assert_eq!(
            solve("pos=<-5,0,0>, r=2\npos=<-3,0,0>, r=0\npos=<-2,0,0>, r=0\n")
                .unwrap()
                .to_string(),
            "2\n3\n",
            "inclusive radius and negative coordinates"
        );
    }

    #[test]
    fn wide_coordinate_distances_are_exact() {
        assert_eq!(
            solve("pos=<-9223372036854775808,0,0>, r=18446744073709551615\npos=<9223372036854775807,0,0>, r=0\n")
                .unwrap()
                .to_string(),
            "2\n9223372036854775807\n",
            "distance across the signed coordinate domain"
        );
    }

    #[test]
    fn overlap_precedes_distance_and_distance_breaks_ties() {
        for (input, expected) in [
            (
                "pos=<0,0,0>, r=0\npos=<8,0,0>, r=0\npos=<8,0,0>, r=0\n",
                "2\n8\n",
            ),
            ("pos=<-8,0,0>, r=0\npos=<3,0,0>, r=0\n", "1\n3\n"),
            ("pos=<2,2,2>, r=6\n", "1\n0\n"),
            ("pos=<-8,0,0>, r=2\n", "1\n6\n"),
        ] {
            assert_eq!(solve(input).unwrap().to_string(), expected, "{input}");
        }
    }

    #[test]
    fn exhaustive_small_formations_match_point_enumeration() {
        for seed in 0_i64..80 {
            let mut input = String::new();
            let mut bots = Vec::new();
            for index in 0..5 {
                let x = (seed * 7 + index * 3) % 9 - 4;
                let y = (seed * 3 + index * 7) % 9 - 4;
                let z = (seed * 5 + index * 2) % 9 - 4;
                let radius = (seed + index) % 4;
                writeln!(input, "pos=<{x},{y},{z}>, r={radius}").unwrap();
                bots.push((x, y, z, radius));
            }
            let mut best = (0, std::cmp::Reverse(i64::MAX));
            for x in -7_i64..=7 {
                for y in -7_i64..=7 {
                    for z in -7_i64..=7 {
                        let coverage = bots
                            .iter()
                            .filter(|&&(a, b, c, r)| {
                                (x - a).abs() + (y - b).abs() + (z - c).abs() <= r
                            })
                            .count();
                        best = best.max((coverage, std::cmp::Reverse(x.abs() + y.abs() + z.abs())));
                    }
                }
            }
            assert_eq!(
                super::closest(&super::parse(&input).unwrap().bots).unwrap(),
                u128::try_from(best.1.0).unwrap(),
                "formation {seed}"
            );
        }
    }
}
