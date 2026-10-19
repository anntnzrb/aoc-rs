//! Validated rectangular fabric claims with distinct identifiers and half-open coordinate ranges.

use std::{collections::HashSet, num::NonZeroU32, ops::Range};

pub mod dense_grid;
pub mod primary;

/// The fabric claims in their input order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    claims: Vec<Claim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Claim {
    id: NonZeroU32,
    x: Range<u32>,
    y: Range<u32>,
}

/// Input that does not match the fabric claim grammar.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// The input has no records or contains a malformed claim.
    #[error(transparent)]
    Records(#[from] aoc_core::RecordsError<ClaimError>),
    /// Two claims have the same identifier.
    #[error("duplicate claim identifier")]
    DuplicateId,
}

/// A line that does not match the fabric claim grammar.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ClaimError {
    /// A claim does not follow the required punctuation and field structure.
    #[error("expected a claim in the form #id @ left,top: widthxheight")]
    Syntax,
    /// A field is not an unsigned decimal integer.
    #[error("invalid claim integer")]
    Integer(#[from] std::num::ParseIntError),
    /// Claim IDs and dimensions must be positive.
    #[error("claim identifiers and dimensions must be positive")]
    Zero,
    /// A rectangle endpoint exceeds the coordinate range.
    #[error("claim endpoint exceeds the unsigned 32-bit range")]
    Overflow,
}

/// Parses positive, uniquely identified rectangles with nonnegative offsets.
///
/// # Errors
///
/// Returns [`ParseError`] for empty input, malformed claims, duplicate IDs or overflowing endpoints.
pub fn parse(input: &str) -> Result<Puzzle, ParseError> {
    let claims = aoc_core::parse_records(input, |line| {
        let (id, rectangle) = line
            .strip_prefix('#')
            .and_then(|rest| rest.split_once(" @ "))
            .ok_or(ClaimError::Syntax)?;
        let id = NonZeroU32::new(number(id)?).ok_or(ClaimError::Zero)?;
        let (offset, dimensions) = rectangle.split_once(": ").ok_or(ClaimError::Syntax)?;
        let (left, top) = offset.split_once(',').ok_or(ClaimError::Syntax)?;
        let (width, height) = dimensions.split_once('x').ok_or(ClaimError::Syntax)?;
        let x = extent(left, width)?;
        let y = extent(top, height)?;
        Ok(Claim { id, x, y })
    })?;
    let mut ids = HashSet::new();
    for claim in &claims {
        if !ids.insert(claim.id) {
            return Err(ParseError::DuplicateId);
        }
    }
    Ok(Puzzle { claims })
}

fn number(text: &str) -> Result<u32, ClaimError> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ClaimError::Syntax);
    }
    Ok(text.parse()?)
}

fn extent(offset: &str, length: &str) -> Result<Range<u32>, ClaimError> {
    let start = number(offset)?;
    let length = NonZeroU32::new(number(length)?).ok_or(ClaimError::Zero)?;
    let end = start
        .checked_add(length.get())
        .ok_or(ClaimError::Overflow)?;
    Ok(start..end)
}

#[cfg(test)]
mod tests {
    use super::{ClaimError, ParseError, parse};

    fn line_error(source: ClaimError, line: usize) -> ParseError {
        ParseError::Records(aoc_core::RecordsError::Line { line, source })
    }

    #[test]
    fn empty_input_is_rejected() {
        assert_eq!(
            parse(""),
            Err(ParseError::Records(aoc_core::RecordsError::Empty)),
            "no claims"
        );
    }

    #[test]
    fn lines_keep_their_order() {
        let puzzle = parse("#9 @ 0,2: 1x3\r\n#4 @ 7,0: 2x1\r\n").unwrap();
        let claims = puzzle.claims;
        assert_eq!(claims[0].id.get(), 9, "first identifier");
        assert_eq!(claims[0].x, 0..1, "origin and width");
        assert_eq!(claims[0].y, 2..5, "top and height");
        assert_eq!(claims[1].id.get(), 4, "second identifier");
    }

    #[test]
    fn malformed_claims_are_rejected() {
        for text in [
            "\n",
            "1 @ 0,0: 1x1",
            "#1 @ 0,0: 1x1 extra",
            "#1 @ +0,0: 1x1",
            "#1 @ 0,0: 1x",
            "#1 @ 0,0: 1x1\n\n",
        ] {
            let line = if text.contains('\n') && text != "\n" {
                2
            } else {
                1
            };
            assert_eq!(
                parse(text),
                Err(line_error(ClaimError::Syntax, line)),
                "{text:?}"
            );
        }
        for text in ["#0 @ 0,0: 1x1", "#1 @ 0,0: 0x1", "#1 @ 0,0: 1x0"] {
            assert_eq!(
                parse(text),
                Err(line_error(ClaimError::Zero, 1)),
                "{text:?}"
            );
        }
        assert_eq!(
            parse("#1 @ 4294967295,0: 1x1"),
            Err(line_error(ClaimError::Overflow, 1)),
            "endpoint overflow"
        );
        assert_eq!(
            parse("#1 @ 0,0: 1x1\n#1 @ 2,2: 1x1"),
            Err(ParseError::DuplicateId),
            "IDs identify distinct claims"
        );
        assert!(
            matches!(
                parse("#4294967296 @ 0,0: 1x1"),
                Err(ParseError::Records(aoc_core::RecordsError::Line {
                    line: 1,
                    source: ClaimError::Integer(_)
                }))
            ),
            "integer overflow retains its cause"
        );
    }

    #[test]
    fn coordinates_round_trip() {
        for offset in [0_u32, 1, 999, u32::MAX - 1] {
            for size in [1_u32, 2, 7] {
                let text = format!("#17 @ {offset},0: {size}x1");
                if let Some(end) = offset.checked_add(size) {
                    let puzzle = parse(&text).unwrap();
                    assert_eq!(puzzle.claims[0].x, offset..end, "{text}");
                } else {
                    assert_eq!(
                        parse(&text),
                        Err(line_error(ClaimError::Overflow, 1)),
                        "{text}"
                    );
                }
            }
        }
    }
}
