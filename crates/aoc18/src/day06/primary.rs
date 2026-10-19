// approach: enumerate the sites' bounding box and discard owners touching its boundary
// cost: O(nWH) time, O(n) memory, one parsing pass plus WH site-list passes
// trade: direct Manhattan comparisons avoid storing a grid or approximating infinite regions

//! Nearest-site areas on the finite coordinate bounding box.

use aoc_core::Answers;

use super::{ParseError, Point, parse};

/// Failures while parsing sites or counting their regions.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// Invalid coordinate input.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// A region count cannot be represented.
    #[error("region size overflow")]
    Overflow,
}

fn bounds(points: &[Point]) -> Result<(i64, i64, i64, i64), SolveError> {
    let first = points.first().ok_or(ParseError::Empty)?;
    Ok(points.iter().fold(
        (
            i64::from(first.x),
            i64::from(first.x),
            i64::from(first.y),
            i64::from(first.y),
        ),
        |(left, right, top, bottom), point| {
            (
                left.min(i64::from(point.x)),
                right.max(i64::from(point.x)),
                top.min(i64::from(point.y)),
                bottom.max(i64::from(point.y)),
            )
        },
    ))
}

fn distance(point: Point, x: i64, y: i64) -> Result<u64, SolveError> {
    i64::from(point.x)
        .abs_diff(x)
        .checked_add(i64::from(point.y).abs_diff(y))
        .ok_or(SolveError::Overflow)
}

fn owner(points: &[Point], x: i64, y: i64) -> Result<Option<usize>, SolveError> {
    let mut nearest = None;
    let mut minimum = u64::MAX;
    for (index, &point) in points.iter().enumerate() {
        let current = distance(point, x, y)?;
        match current.cmp(&minimum) {
            std::cmp::Ordering::Less => {
                minimum = current;
                nearest = Some(index);
            }
            std::cmp::Ordering::Equal => nearest = None,
            std::cmp::Ordering::Greater => {}
        }
    }
    Ok(nearest)
}

fn largest_finite(points: &[Point]) -> Result<u64, SolveError> {
    let (left, right, top, bottom) = bounds(points)?;
    let mut areas = vec![(0_u64, false); points.len()];
    for y in top..=bottom {
        for x in left..=right {
            if let Some(index) = owner(points, x, y)? {
                let (area, infinite) = areas.get_mut(index).ok_or(SolveError::Overflow)?;
                *area = area.checked_add(1).ok_or(SolveError::Overflow)?;
                // Outside the box, moving farther outward preserves the closest owner.
                *infinite |= x == left || x == right || y == top || y == bottom;
            }
        }
    }
    Ok(areas
        .into_iter()
        .filter(|&(_, infinite)| !infinite)
        .map(|(area, _)| area)
        .max()
        .unwrap_or(0))
}

/// Finds the largest finite area; unfinished part two renders `0`.
///
/// # Errors
/// Returns [`SolveError`] for invalid sites or an overflowing count.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let puzzle = parse(input)?;
    Ok(Answers::new(largest_finite(&puzzle.points)?, 0))
}

#[cfg(test)]
mod tests {
    use super::{largest_finite, owner};
    use crate::day06::Point;

    #[test]
    fn ties_and_infinite_regions_are_excluded() {
        let sites = [Point { x: -1, y: 0 }, Point { x: 1, y: 0 }];
        assert_eq!(owner(&sites, 0, 0).unwrap(), None, "tied midpoint");
        assert_eq!(largest_finite(&sites).unwrap(), 0, "both regions infinite");
        let enclosed_sites = [
            Point { x: 0, y: 0 },
            Point { x: -2, y: 0 },
            Point { x: 2, y: 0 },
            Point { x: 0, y: -2 },
            Point { x: 0, y: 2 },
        ];
        assert_eq!(
            largest_finite(&enclosed_sites).unwrap(),
            1,
            "isolated center owns itself"
        );
    }
}
