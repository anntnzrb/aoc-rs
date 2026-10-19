// approach: Count each letter per box, then multiply the counts of boxes with pairs and triples.
// cost: O(n L) time, O(n) borrowed identifiers and O(1) counters; two full input passes.
// trade: A fixed byte histogram avoids per-box allocation; checked counts keep overflow explicit.

//! Compute the inventory checksum from exact letter multiplicities.

use aoc_core::Answers;

use super::{ParseError, parse};

/// A malformed identifier or an unrepresentable checksum.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// An identifier does not match the input grammar.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// A count or checksum exceeds the supported range.
    #[error("inventory count exceeds the unsigned 64-bit range")]
    Overflow,
}

/// Solves the checksum; the unfinished second part renders as `0`.
///
/// # Errors
///
/// Returns [`SolveError`] for malformed identifiers or count overflow.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let puzzle = parse(input)?;
    let mut pairs = 0_u64;
    let mut triples = 0_u64;
    for id in puzzle.lines() {
        let mut counts = [0_u64; 256];
        for byte in id.bytes() {
            if let Some(count) = counts.get_mut(usize::from(byte)) {
                *count = count.checked_add(1).ok_or(SolveError::Overflow)?;
            }
        }
        pairs = pairs
            .checked_add(u64::from(counts.contains(&2)))
            .ok_or(SolveError::Overflow)?;
        triples = triples
            .checked_add(u64::from(counts.contains(&3)))
            .ok_or(SolveError::Overflow)?;
    }
    let checksum = pairs.checked_mul(triples).ok_or(SolveError::Overflow)?;
    Ok(Answers::new(checksum, 0))
}

#[cfg(test)]
mod tests {
    use aoc_core::Answers;

    use super::solve;

    #[test]
    fn exact_multiplicities_count_each_box_once() {
        assert_eq!(
            solve("aabb\nccc\nddeee\nffff").unwrap(),
            Answers::new(4, 0),
            "two pair boxes and two triple boxes"
        );
        assert_eq!(
            solve("aaaa\nbbbbb\nxyz").unwrap(),
            Answers::new(0, 0),
            "larger multiplicities do not qualify"
        );
        assert_eq!(
            solve("mmnnnooo").unwrap(),
            Answers::new(1, 0),
            "one box can qualify for both counts"
        );
    }
}
