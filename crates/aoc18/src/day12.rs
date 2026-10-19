//! Plant positions and validated five-pot propagation rules.

use std::collections::{BTreeMap, BTreeSet};

pub mod primary;

/// Initial plants and the patterns that produce plants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    pub(crate) plants: BTreeSet<i64>,
    pub(crate) growing: BTreeSet<u8>,
}

/// Invalid plant configuration or propagation rule.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// No initial state was supplied.
    #[error("missing initial state")]
    Empty,
    /// The initial-state header is malformed.
    #[error("invalid initial state header")]
    Header,
    /// Only dots and hashes encode pots.
    #[error("invalid pot symbol {0:?}")]
    Symbol(char),
    /// Rules have exactly five input pots and one output pot.
    #[error("invalid propagation rule")]
    Rule,
    /// Each pattern may be specified once.
    #[error("duplicate propagation rule")]
    Duplicate,
    /// An empty neighborhood cannot grow a finite set of plants.
    #[error("empty neighborhoods produce infinitely many plants")]
    Infinite,
    /// Input positions exceed the coordinate representation.
    #[error("initial position is too large")]
    Position(#[from] std::num::TryFromIntError),
}

const fn pot(symbol: char) -> Result<bool, ParseError> {
    match symbol {
        '#' => Ok(true),
        '.' => Ok(false),
        other => Err(ParseError::Symbol(other)),
    }
}

/// Parses an initial state and optional rules; omitted rules produce no plant.
///
/// # Errors
/// Returns an error for malformed symbols, duplicate rules, or infinite growth
/// from the all-empty neighborhood.
pub fn parse(input: &str) -> Result<Puzzle, ParseError> {
    let mut lines = input.lines();
    let initial = lines.next().ok_or(ParseError::Empty)?;
    let state = initial
        .strip_prefix("initial state: ")
        .ok_or(ParseError::Header)?;
    if state.is_empty() {
        return Err(ParseError::Header);
    }
    let mut plants = BTreeSet::new();
    for (position, symbol) in state.chars().enumerate() {
        if pot(symbol)? {
            plants.insert(i64::try_from(position)?);
        }
    }
    let mut rules = BTreeMap::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let (pattern, output) = line.split_once(" => ").ok_or(ParseError::Rule)?;
        if pattern.len() != 5 || output.len() != 1 {
            return Err(ParseError::Rule);
        }
        let mut mask = 0_u8;
        for symbol in pattern.chars() {
            mask = (mask << 1) | u8::from(pot(symbol)?);
        }
        let result = pot(output.chars().next().ok_or(ParseError::Rule)?)?;
        if mask == 0 && result {
            return Err(ParseError::Infinite);
        }
        if rules.insert(mask, result).is_some() {
            return Err(ParseError::Duplicate);
        }
    }
    let growing = rules
        .into_iter()
        .filter_map(|(mask, grows)| grows.then_some(mask))
        .collect();
    Ok(Puzzle { plants, growing })
}

#[cfg(test)]
mod tests {
    use super::{ParseError, parse};

    #[test]
    fn empty_input_is_rejected() {
        assert_eq!(parse(""), Err(ParseError::Empty), "no initial state");
    }

    #[test]
    fn lines_keep_their_order() {
        let puzzle = parse("initial state: .#..#\n\n..#.. => #\n").unwrap();
        assert_eq!(
            puzzle.plants.into_iter().collect::<Vec<_>>(),
            [1, 4],
            "positions are relative to the first pot"
        );
        assert_eq!(
            puzzle.growing.into_iter().collect::<Vec<_>>(),
            [4],
            "five-pot pattern encoding"
        );
    }

    #[test]
    fn lines_borrow_the_input_and_strip_crlf() {
        let puzzle = parse("initial state: #.\r\n\r\n..#.. => #\r\n").unwrap();
        assert_eq!(
            puzzle.plants.into_iter().collect::<Vec<_>>(),
            [0],
            "CRLF input retains positions"
        );
    }

    #[test]
    fn rejects_invalid_rules() {
        for (input, expected) in [
            ("initial state: x", ParseError::Symbol('x')),
            ("initial state: #\n.... => #", ParseError::Rule),
            ("initial state: #\n..... => #", ParseError::Infinite),
            (
                "initial state: #\n..#.. => #\n..#.. => .",
                ParseError::Duplicate,
            ),
        ] {
            assert_eq!(parse(input), Err(expected), "reject {input:?}");
        }
    }
}
