//! Classification of submission responses into exit codes.

/// The server's verdict on a submitted answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    Right,
    Wrong,
    /// Cooldown, or the part is already complete.
    Wait,
    Unknown,
}

impl Verdict {
    pub(crate) fn classify(text: &str) -> Self {
        if text.contains("That's the right answer") {
            Self::Right
        } else if text.contains("That's not the right answer") {
            Self::Wrong
        } else if text.contains("You gave an answer too recently")
            || text.contains("You don't seem to be solving the right level")
        {
            Self::Wait
        } else {
            Self::Unknown
        }
    }

    /// Exit code: 0 right, 1 wrong, 2 cooldown or already complete, 3 unknown.
    pub(crate) const fn code(self) -> u8 {
        match self {
            Self::Right => 0,
            Self::Wrong => 1,
            Self::Wait => 2,
            Self::Unknown => 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Verdict;

    #[test]
    fn verdicts_map_to_documented_exit_codes() {
        let cases = [
            ("That's the right answer! You are one gold star closer.", 0),
            ("That's not the right answer; your answer is too high.", 1),
            (
                "You gave an answer too recently; you have 30s left to wait.",
                2,
            ),
            (
                "You don't seem to be solving the right level. Did you already complete it?",
                2,
            ),
            ("Something new.", 3),
            // Each wait message classifies alone; neither requires the other.
            ("You gave an answer too recently.", 2),
            ("You don't seem to be solving the right level.", 2),
        ];
        for (text, code) in cases {
            assert_eq!(Verdict::classify(text).code(), code, "{text}");
        }
    }
}
