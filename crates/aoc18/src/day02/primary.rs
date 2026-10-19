// approach: Count exact letter multiplicities, then find the pair with one positional mismatch.
// cost: O(n² L) time, O(n + L) memory; parse and checksum passes, then buffered ID pairs.
// trade: Direct pair comparisons avoid signature allocation and stop after two mismatches.

//! Compute the checksum and recover common letters from the matching box identifiers.

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

/// Solves both parts, omitting the second answer when no qualifying pair exists.
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
    Ok(common_letters(puzzle.lines()).map_or_else(
        || Answers::part1_only(checksum),
        |common| Answers::new(checksum, common),
    ))
}

fn common_letters(mut ids: &[&str]) -> Option<String> {
    while let Some((id, rest)) = ids.split_first() {
        for other in rest {
            if id.len() == other.len()
                && id
                    .bytes()
                    .zip(other.bytes())
                    .filter(|(a, b)| a != b)
                    .take(2)
                    .count()
                    == 1
            {
                return Some(
                    id.bytes()
                        .zip(other.bytes())
                        .filter_map(|(a, b)| (a == b).then_some(char::from(a)))
                        .collect(),
                );
            }
        }
        ids = rest;
    }
    None
}

#[cfg(test)]
mod tests {
    use aoc_core::Answers;

    use super::solve;

    #[test]
    fn exact_multiplicities_count_each_box_once() {
        assert_eq!(
            solve("aabb\nccc\nddeee\nffff").unwrap(),
            Answers::part1_only(4),
            "two pair boxes and two triple boxes"
        );
        assert_eq!(
            solve("aaaa\nbbbbb\nxyz").unwrap(),
            Answers::part1_only(0),
            "larger multiplicities do not qualify"
        );
        assert_eq!(
            solve("mmnnnooo").unwrap(),
            Answers::part1_only(1),
            "one box can qualify for both counts"
        );
    }

    #[test]
    fn only_one_positional_difference_qualifies() {
        for (input, expected) in [
            ("abc\nxbc", "bc"),
            ("abc\naxc", "ac"),
            ("abc\nabx", "ab"),
            ("abc\nabc\naxc", "ac"),
        ] {
            assert_eq!(
                solve(input).unwrap(),
                Answers::new(0, expected),
                "{input:?}"
            );
        }
        for input in ["abc\nabc", "abc\naxy", "abc\nabcd", "abc\nxy"] {
            assert_eq!(solve(input).unwrap(), Answers::part1_only(0), "{input:?}");
        }
    }

    #[test]
    fn single_letter_ids_have_empty_common_letters() {
        assert_eq!(
            solve("a\nb").unwrap(),
            Answers::new(0, ""),
            "the mismatch is the entire identifier"
        );
    }
}
