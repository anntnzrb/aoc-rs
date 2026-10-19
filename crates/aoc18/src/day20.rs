//! Validated, flat route expressions describing rooms connected by doors.

pub mod primary;

/// A route expression with balanced groups and explicit start/end anchors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle<'a> {
    route: &'a str,
}

impl Puzzle<'_> {
    /// Returns the validated expression, excluding its anchors.
    #[must_use]
    pub const fn route(&self) -> &str {
        self.route
    }
}

/// Invalid route syntax or an unrepresentable map.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// The input has no expression.
    #[error("missing route expression")]
    Empty,
    /// Both anchors must surround the entire expression.
    #[error("route must start with ^ and end with $")]
    Anchors,
    /// A character is not part of the route grammar.
    #[error("invalid route character {0:?}")]
    Character(char),
    /// A branch separator or closer has no opening group.
    #[error("branch delimiter outside a group")]
    Group,
    /// At least one group remains open.
    #[error("unclosed route group")]
    Unclosed,
    /// Coordinates, distances, or nesting exceed their integer representation.
    #[error("route exceeds map representation")]
    Overflow,
}

/// Parses an anchored route without recursion, borrowing the original text.
///
/// # Errors
/// Returns [`ParseError`] for missing anchors, invalid characters or unbalanced groups.
pub fn parse(input: &str) -> Result<Puzzle<'_>, ParseError> {
    let input = input.trim_end_matches(['\r', '\n']);
    if input.is_empty() {
        return Err(ParseError::Empty);
    }
    let route = input
        .strip_prefix('^')
        .and_then(|text| text.strip_suffix('$'))
        .ok_or(ParseError::Anchors)?;
    let mut depth = 0_usize;
    for character in route.chars() {
        match character {
            'N' | 'S' | 'E' | 'W' => {}
            '(' => depth = depth.checked_add(1).ok_or(ParseError::Overflow)?,
            ')' => depth = depth.checked_sub(1).ok_or(ParseError::Group)?,
            '|' if depth != 0 => {}
            '|' => return Err(ParseError::Group),
            other => return Err(ParseError::Character(other)),
        }
    }
    if depth != 0 {
        return Err(ParseError::Unclosed);
    }
    Ok(Puzzle { route })
}

#[cfg(test)]
mod tests {
    use super::{ParseError, parse};

    #[test]
    fn empty_input_is_rejected() {
        assert_eq!(parse(""), Err(ParseError::Empty), "no expression");
    }

    #[test]
    fn lines_keep_their_order() {
        assert_eq!(parse("^NESW$").unwrap().route(), "NESW", "route order");
    }

    #[test]
    fn lines_borrow_the_input_and_strip_crlf() {
        let input = String::from("^(N|)E$\r\n");
        let puzzle = parse(&input).unwrap();
        assert_eq!(
            puzzle.route(),
            "(N|)E",
            "strip only line ending and anchors"
        );
        assert_eq!(
            puzzle.route().as_ptr(),
            input.as_bytes()[1..].as_ptr(),
            "borrow storage"
        );
    }

    #[test]
    fn rejects_malformed_grammar() {
        for input in [
            "NESW", "^N", "N$", "^N|E$", "^N)$", "^(N$", "^Q$", "^N\nS$", "^N^S$",
        ] {
            assert!(parse(input).is_err(), "reject {input:?}");
        }
        assert_eq!(
            parse("^()(|N)$").unwrap().route(),
            "()(|N)",
            "empty options are valid"
        );
    }
}
