// approach: minimize the convex bounding-box perimeter, then render occupied sky coordinates
// cost: O(n log t + A log n) time, O(n+A) memory, one input parsing pass
// trade: exact integer search avoids a guessed time horizon; slash-separated rows keep one answer line

//! Compact moving-light messages rendered as slash-separated raster rows.

use std::collections::BTreeSet;

use aoc_core::Answers;

use super::{Light, ParseError, parse};

type Bounds = (i64, i64, i64, i64);

/// Parsing, coordinate arithmetic or rendering failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// Invalid observations.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// Coordinates or time exceed the supported integer range.
    #[error("sky coordinate arithmetic overflow")]
    Overflow,
    /// The compact sky is too large to render.
    #[error("message raster exceeds one million cells")]
    RasterTooLarge,
}

fn position(light: Light, seconds: i64) -> Result<(i64, i64), SolveError> {
    let axis = |initial, velocity| {
        i64::from(velocity)
            .checked_mul(seconds)
            .and_then(|offset| i64::from(initial).checked_add(offset))
            .ok_or(SolveError::Overflow)
    };
    Ok((
        axis(light.position.0, light.velocity.0)?,
        axis(light.position.1, light.velocity.1)?,
    ))
}

fn bounds(lights: &[Light], seconds: i64) -> Result<Bounds, SolveError> {
    let first = lights.first().ok_or(ParseError::Empty)?;
    let (first_x, first_y) = position(*first, seconds)?;
    lights.iter().try_fold(
        (first_x, first_x, first_y, first_y),
        |(left, right, top, bottom), &light| {
            let (x, y) = position(light, seconds)?;
            Ok((left.min(x), right.max(x), top.min(y), bottom.max(y)))
        },
    )
}

fn span(lights: &[Light], seconds: i64) -> Result<u64, SolveError> {
    let (left, right, top, bottom) = bounds(lights, seconds)?;
    right
        .abs_diff(left)
        .checked_add(bottom.abs_diff(top))
        .ok_or(SolveError::Overflow)
}

fn rising(lights: &[Light], seconds: i64) -> Result<bool, SolveError> {
    Ok(
        span(lights, seconds.checked_add(1).ok_or(SolveError::Overflow)?)?
            >= span(lights, seconds)?,
    )
}

fn message_time(lights: &[Light]) -> Result<i64, SolveError> {
    // Each axis span is a maximum of affine differences, hence convex in time.
    let mut upper = 1_i64;
    while !rising(lights, upper)? {
        upper = upper.checked_mul(2).ok_or(SolveError::Overflow)?;
    }
    let mut lower = 0_i64;
    while lower < upper {
        let middle = lower
            .checked_add(
                upper
                    .checked_sub(lower)
                    .ok_or(SolveError::Overflow)?
                    .checked_div(2)
                    .ok_or(SolveError::Overflow)?,
            )
            .ok_or(SolveError::Overflow)?;
        if rising(lights, middle)? {
            upper = middle;
        } else {
            lower = middle.checked_add(1).ok_or(SolveError::Overflow)?;
        }
    }
    Ok(lower)
}

fn render(lights: &[Light], seconds: i64) -> Result<String, SolveError> {
    let (left, right, top, bottom) = bounds(lights, seconds)?;
    let width = right
        .abs_diff(left)
        .checked_add(1)
        .ok_or(SolveError::Overflow)?;
    let height = bottom
        .abs_diff(top)
        .checked_add(1)
        .ok_or(SolveError::Overflow)?;
    if width.checked_mul(height).ok_or(SolveError::Overflow)? > 1_000_000 {
        return Err(SolveError::RasterTooLarge);
    }
    let positions = lights
        .iter()
        .map(|&light| position(light, seconds))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let mut raster = String::new();
    for y in top..=bottom {
        if y != top {
            raster.push('/');
        }
        for x in left..=right {
            raster.push(if positions.contains(&(x, y)) {
                '#'
            } else {
                '.'
            });
        }
    }
    Ok(raster)
}

/// Renders the tightest sky with `/` row separators; unfinished part two is `0`.
///
/// # Errors
/// Returns [`SolveError`] for malformed observations, overflow or excessive raster size.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let puzzle = parse(input)?;
    let seconds = message_time(&puzzle.lights)?;
    Ok(Answers::new(render(&puzzle.lights, seconds)?, 0))
}

#[cfg(test)]
mod tests {
    use super::{message_time, render};
    use crate::day10::parse;

    #[test]
    fn finds_convergence_and_earliest_plateau() {
        let puzzle =
            parse("position=<-3, 0> velocity=<1, 0>\nposition=<3, 0> velocity=<-1, 0>").unwrap();
        assert_eq!(
            message_time(&puzzle.lights).unwrap(),
            3,
            "coincident lights"
        );
        assert_eq!(
            render(&puzzle.lights, 3).unwrap(),
            "#",
            "duplicates occupy one pixel"
        );
        let stationary =
            parse("position=<2, -1> velocity=<0, 0>\nposition=<3, 0> velocity=<0, 0>").unwrap();
        assert_eq!(
            message_time(&stationary.lights).unwrap(),
            0,
            "earliest stationary frame"
        );
        assert_eq!(
            render(&stationary.lights, 0).unwrap(),
            "#./.#",
            "negative coordinates and row separators"
        );
    }

    #[test]
    fn search_has_no_fixed_time_horizon() {
        let distant =
            parse("position=<-2000000, 0> velocity=<1, 0>\nposition=<2000001, 0> velocity=<-1, 0>")
                .unwrap();
        assert_eq!(
            message_time(&distant.lights).unwrap(),
            2_000_000,
            "earliest of two equally compact seconds"
        );
        let diverging =
            parse("position=<0, 0> velocity=<-1, 0>\nposition=<1, 0> velocity=<1, 0>").unwrap();
        assert_eq!(
            message_time(&diverging.lights).unwrap(),
            0,
            "already diverging"
        );
    }
}
