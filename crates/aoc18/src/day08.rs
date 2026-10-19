//! Iteratively parsed license trees with child indices and metadata.

pub mod primary;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Node {
    children: Vec<usize>,
    metadata: Vec<u64>,
}

/// A validated license tree, stored in child-before-parent order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    nodes: Vec<Node>,
}

/// Invalid license encoding.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// Missing root node.
    #[error("empty license")]
    Empty,
    /// Invalid unsigned number.
    #[error("invalid license number")]
    Integer(#[from] std::num::ParseIntError),
    /// Missing header, child or metadata entries.
    #[error("truncated license node")]
    Truncated,
    /// Extra numbers after the root.
    #[error("trailing license numbers")]
    Trailing,
    /// Metadata entries are required by the grammar.
    #[error("node must have metadata")]
    NoMetadata,
}

struct Pending {
    remaining: u64,
    metadata_count: u64,
    children: Vec<usize>,
}

fn header(numbers: &mut impl Iterator<Item = u64>) -> Result<Pending, ParseError> {
    let remaining = numbers.next().ok_or(ParseError::Truncated)?;
    let metadata_count = numbers.next().ok_or(ParseError::Truncated)?;
    if metadata_count == 0 {
        return Err(ParseError::NoMetadata);
    }
    Ok(Pending {
        remaining,
        metadata_count,
        children: Vec::new(),
    })
}

/// Parses exactly one complete tree without recursive call-stack growth.
///
/// # Errors
/// Returns [`ParseError`] for malformed numbers, incomplete nodes or trailing entries.
pub fn parse(input: &str) -> Result<Puzzle, ParseError> {
    let numbers = input
        .split_whitespace()
        .map(str::parse)
        .collect::<Result<Vec<u64>, _>>()?;
    if numbers.is_empty() {
        return Err(ParseError::Empty);
    }
    let mut numbers = numbers.into_iter();
    let mut pending = vec![header(&mut numbers)?];
    let mut nodes = Vec::new();
    while let Some(frame) = pending.last_mut() {
        if let Some(remaining) = frame.remaining.checked_sub(1) {
            frame.remaining = remaining;
            pending.push(header(&mut numbers)?);
        } else {
            let finished = pending.pop().ok_or(ParseError::Truncated)?;
            let mut metadata = Vec::new();
            for _ in 0..finished.metadata_count {
                metadata.push(numbers.next().ok_or(ParseError::Truncated)?);
            }
            let index = nodes.len();
            nodes.push(Node {
                children: finished.children,
                metadata,
            });
            if let Some(parent) = pending.last_mut() {
                parent.children.push(index);
            }
        }
    }
    if numbers.next().is_some() {
        return Err(ParseError::Trailing);
    }
    Ok(Puzzle { nodes })
}

#[cfg(test)]
mod tests {
    use super::{ParseError, parse};

    #[test]
    fn empty_input_is_rejected() {
        assert!(matches!(parse(""), Err(ParseError::Empty)), "empty license");
    }

    #[test]
    fn lines_keep_their_order() {
        let puzzle = parse("1 1 0 2 7 8 9").unwrap();
        assert_eq!(puzzle.nodes[0].metadata, [7, 8], "leaf metadata order");
        assert_eq!(puzzle.nodes[1].children, [0], "parent references child");
        for input in ["0", "1 1 0 1 2", "0 1", "0 1 7 8", "0 0", "0 1 -2"] {
            assert!(parse(input).is_err(), "malformed tree {input}");
        }
    }

    #[test]
    fn lines_borrow_the_input_and_strip_crlf() {
        assert_eq!(
            parse("0 1 4\r\n").unwrap().nodes[0].metadata,
            [4],
            "trailing whitespace"
        );
    }
}
