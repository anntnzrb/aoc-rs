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

/// Sums license metadata and evaluates child references in postorder.
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
    let mut values = Vec::with_capacity(puzzle.nodes.len());
    for node in &puzzle.nodes {
        let mut value = 0_u64;
        for &entry in &node.metadata {
            let contribution = if node.children.is_empty() {
                entry
            } else if let Some(index) = entry.checked_sub(1) {
                // An index wider than usize cannot name an existing child.
                usize::try_from(index).map_or(0, |index| {
                    node.children
                        .get(index)
                        .and_then(|&child| values.get(child))
                        .copied()
                        .unwrap_or(0)
                })
            } else {
                0
            };
            value = value
                .checked_add(contribution)
                .ok_or(SolveError::Overflow)?;
        }
        values.push(value);
    }
    Ok(Answers::new(sum, values.last().copied().unwrap_or(0)))
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

    #[test]
    fn repeated_child_references_and_invalid_indices() {
        assert_eq!(
            solve("1 4 0 1 7 1 1 0 2").unwrap(),
            aoc_core::Answers::new(11, 14),
            "repeat counts; zero and absent children do not"
        );
        assert_eq!(
            solve("0 2 3 4").unwrap(),
            aoc_core::Answers::new(7, 7),
            "leaf value is metadata sum"
        );
    }
}
