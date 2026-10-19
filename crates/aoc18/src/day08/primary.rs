// approach: parse nodes with an explicit stack and sum each metadata entry
// cost: O(n) time, O(n) memory, two passes including buffered number traversal
// trade: flat child-before-parent nodes avoid recursion depth limits

//! Metadata aggregation over the validated license tree.

use aoc_core::Answers;

use super::{ParseError, parse};

/// License parsing or aggregation failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// Invalid license tree.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// Metadata sum exceeds the answer range.
    #[error("license sum overflow")]
    Overflow,
}

/// Sums license metadata; unfinished part two renders `0`.
///
/// # Errors
/// Returns [`SolveError`] for invalid trees or an overflowing sum.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let puzzle = parse(input)?;
    let sum = puzzle
        .nodes
        .iter()
        .flat_map(|node| &node.metadata)
        .try_fold(0_u64, |sum, &entry| {
            sum.checked_add(entry).ok_or(SolveError::Overflow)
        })?;
    Ok(Answers::new(sum, 0))
}

#[cfg(test)]
mod tests {
    use super::{SolveError, solve};

    #[test]
    fn sums_parent_and_child_metadata() {
        assert_eq!(
            solve("1 1 0 2 7 8 9").unwrap(),
            aoc_core::Answers::new(24, 0),
            "all metadata counted once"
        );
        assert!(
            matches!(
                solve("0 2 18446744073709551615 1"),
                Err(SolveError::Overflow)
            ),
            "sum checked"
        );
    }
}
