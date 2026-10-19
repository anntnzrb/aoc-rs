//! Rendered puzzle answers in part order.

use std::fmt;

/// A day's rendered answers: part 1, then part 2 when the puzzle has a computational second part.
///
/// An unfinished part is rendered as `0` by passing `0` for it.
///
/// # Examples
///
/// ```
/// let answers = aoc_core::Answers::new(42, 0);
/// assert_eq!(answers.to_string(), "42\n0\n");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Answers {
    part1: String,
    part2: Option<String>,
}

impl Answers {
    /// Builds answers for both parts.
    #[must_use]
    pub fn new(part1: impl fmt::Display, part2: impl fmt::Display) -> Self {
        Self {
            part1: part1.to_string(),
            part2: Some(part2.to_string()),
        }
    }

    /// Builds answers for a puzzle whose second part has nothing to compute.
    #[must_use]
    pub fn part1_only(part1: impl fmt::Display) -> Self {
        Self {
            part1: part1.to_string(),
            part2: None,
        }
    }

    /// Returns the rendered part 1 answer.
    #[must_use]
    pub fn part1(&self) -> &str {
        &self.part1
    }

    /// Returns the rendered part 2 answer, absent when the puzzle has no computational part 2.
    #[must_use]
    pub fn part2(&self) -> Option<&str> {
        self.part2.as_deref()
    }
}

/// Emits one answer per line, each terminated by LF, part 1 first.
impl fmt::Display for Answers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.part1)?;
        if let Some(part2) = &self.part2 {
            writeln!(f, "{part2}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Answers;

    #[test]
    fn both_parts_render_on_separate_lines_in_order() {
        let answers = Answers::new("abc", 7);
        assert_eq!(answers.to_string(), "abc\n7\n", "part 1 precedes part 2");
        assert_eq!(answers.part1(), "abc", "part 1 is kept verbatim");
        assert_eq!(answers.part2(), Some("7"), "part 2 is rendered");
    }

    #[test]
    fn single_part_renders_one_line() {
        let answers = Answers::part1_only(-3);
        assert_eq!(answers.to_string(), "-3\n", "only part 1 is printed");
        assert_eq!(answers.part2(), None, "part 2 is absent");
    }
}
