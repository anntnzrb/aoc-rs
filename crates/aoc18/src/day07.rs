//! Validated step dependencies for sleigh assembly.

pub mod primary;

/// A prerequisite edge between uppercase ASCII step labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dependency {
    before: u8,
    after: u8,
}

/// A nonempty list of step dependencies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    dependencies: Vec<Dependency>,
}

/// Invalid instruction grammar or step label.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("expected a dependency between two uppercase ASCII steps")]
pub struct DependencyError;

/// Input error retaining the failing instruction's line.
pub type ParseError = aoc_core::RecordsError<DependencyError>;

const fn label(text: &str) -> Result<u8, DependencyError> {
    match text.as_bytes() {
        &[value] if value.is_ascii_uppercase() => Ok(value),
        _ => Err(DependencyError),
    }
}

/// Parses the statement's prerequisite instruction grammar.
///
/// # Errors
/// Returns [`ParseError`] for empty input or malformed instructions.
pub fn parse(input: &str) -> Result<Puzzle, ParseError> {
    let dependencies = aoc_core::parse_records(input, |line| {
        let middle = line
            .strip_prefix("Step ")
            .and_then(|text| text.strip_suffix(" can begin."))
            .ok_or(DependencyError)?;
        let (before, after) = middle
            .split_once(" must be finished before step ")
            .ok_or(DependencyError)?;
        Ok(Dependency {
            before: label(before)?,
            after: label(after)?,
        })
    })?;
    Ok(Puzzle { dependencies })
}

#[cfg(test)]
mod tests {
    use super::{Dependency, ParseError, parse};

    #[test]
    fn empty_input_is_rejected() {
        assert!(
            matches!(parse(""), Err(ParseError::Empty)),
            "empty instructions"
        );
    }

    #[test]
    fn lines_keep_their_order() {
        let puzzle = parse("Step Z must be finished before step B can begin.\nStep B must be finished before step A can begin.\n").unwrap();
        assert_eq!(
            puzzle.dependencies,
            [
                Dependency {
                    before: b'Z',
                    after: b'B'
                },
                Dependency {
                    before: b'B',
                    after: b'A'
                }
            ],
            "dependency order"
        );
        for input in [
            "Step aa must be finished before step B can begin.",
            "Step A must be finished before step b can begin.",
            "A B",
        ] {
            assert!(
                matches!(parse(input), Err(ParseError::Line { .. })),
                "malformed instruction {input}"
            );
        }
    }

    #[test]
    fn lines_borrow_the_input_and_strip_crlf() {
        assert_eq!(
            parse("Step X must be finished before step Y can begin.\r\n")
                .unwrap()
                .dependencies,
            [Dependency {
                before: b'X',
                after: b'Y'
            }],
            "CRLF terminator"
        );
    }
}
