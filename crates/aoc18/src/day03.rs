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
    /// The input holds no claims.
    #[error("input is empty")]
    Empty,
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
    /// Two claims have the same identifier.
    #[error("duplicate claim identifier")]
    DuplicateId,
}

/// Parses positive, uniquely identified rectangles with nonnegative offsets.
///
/// # Errors
///
/// Returns [`ParseError`] for empty input, malformed claims, duplicate IDs or overflowing endpoints.
pub fn parse(input: &str) -> Result<Puzzle, ParseError> {
    let mut claims = Vec::new();
    let mut ids = HashSet::new();
    for line in input.lines() {
        let (id, rectangle) = line
            .strip_prefix('#')
            .and_then(|rest| rest.split_once(" @ "))
            .ok_or(ParseError::Syntax)?;
        let id = NonZeroU32::new(number(id)?).ok_or(ParseError::Zero)?;
        let (offset, dimensions) = rectangle.split_once(": ").ok_or(ParseError::Syntax)?;
        let (left, top) = offset.split_once(',').ok_or(ParseError::Syntax)?;
        let (width, height) = dimensions.split_once('x').ok_or(ParseError::Syntax)?;
        let x = extent(left, width)?;
        let y = extent(top, height)?;
        if !ids.insert(id) {
            return Err(ParseError::DuplicateId);
        }
        claims.push(Claim { id, x, y });
    }
    if claims.is_empty() {
        return Err(ParseError::Empty);
    }
    Ok(Puzzle { claims })
}

fn number(text: &str) -> Result<u32, ParseError> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ParseError::Syntax);
    }
    Ok(text.parse()?)
}

fn extent(offset: &str, length: &str) -> Result<Range<u32>, ParseError> {
    let start = number(offset)?;
    let length = NonZeroU32::new(number(length)?).ok_or(ParseError::Zero)?;
    let end = start
        .checked_add(length.get())
        .ok_or(ParseError::Overflow)?;
    Ok(start..end)
}

#[cfg(test)]
mod tests {
    use super::{ParseError, parse};

    #[test]
    fn empty_input_is_rejected() {
        assert_eq!(parse(""), Err(ParseError::Empty), "no claims");
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
            assert_eq!(parse(text), Err(ParseError::Syntax), "{text:?}");
        }
        for text in ["#0 @ 0,0: 1x1", "#1 @ 0,0: 0x1", "#1 @ 0,0: 1x0"] {
            assert_eq!(parse(text), Err(ParseError::Zero), "{text:?}");
        }
        assert_eq!(
            parse("#1 @ 4294967295,0: 1x1"),
            Err(ParseError::Overflow),
            "endpoint overflow"
        );
        assert_eq!(
            parse("#1 @ 0,0: 1x1\n#1 @ 2,2: 1x1"),
            Err(ParseError::DuplicateId),
            "IDs identify distinct claims"
        );
        assert!(
            matches!(parse("#4294967296 @ 0,0: 1x1"), Err(ParseError::Integer(_))),
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
                    assert_eq!(parse(&text), Err(ParseError::Overflow), "{text}");
                }
            }
        }
    }
}
