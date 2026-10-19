//! Fixture discovery and answer comparison.
//!
//! A day's fixtures live in `fixtures/YEAR/DD/`: `p1-*.in` with a one-line `p1-*.out`, and
//! `p2-*.in` with a two-line `p2-*.out`. Only as many output lines as expected are compared.

/// Compares rendered answers with an expectation, checking only the expected line count.
pub(crate) fn check(expected: &str, rendered: &str) -> Result<(), String> {
    let Some(body) = expected.strip_suffix('\n') else {
        return Err(if expected.is_empty() {
            "empty expectation".to_owned()
        } else {
            "unterminated expectation".to_owned()
        });
    };
    let expected_lines: Vec<&str> = body.split('\n').collect();
    let actual: Vec<&str> = rendered.split('\n').take(expected_lines.len()).collect();
    if actual == expected_lines {
        Ok(())
    } else {
        Err(format!("expected lines {expected_lines:?}, got {actual:?}"))
    }
}

/// Whether a file name is a fixture input.
pub(crate) fn is_input(name: &str) -> bool {
    std::path::Path::new(name)
        .extension()
        .is_some_and(|extension| extension == "in")
}

#[cfg(test)]
mod tests {
    use super::{check, is_input};

    #[test]
    fn compares_only_the_expected_lines() {
        assert_eq!(check("1\n2\n", "1\n2\n"), Ok(()), "both lines agree");
        assert_eq!(
            check("1\n", "1\n2\n"),
            Ok(()),
            "part 1 fixture ignores part 2"
        );
        assert!(check("1\n", "2\n0\n").is_err(), "different answer");
        assert!(check("1\n2\n", "1\n").is_err(), "missing part 2");
    }

    #[test]
    fn expectations_must_be_terminated() {
        assert_eq!(
            check("", "1\n"),
            Err("empty expectation".to_owned()),
            "empty"
        );
        assert_eq!(
            check("1", "1\n"),
            Err("unterminated expectation".to_owned()),
            "no LF"
        );
    }

    #[test]
    fn inputs_end_in_dot_in() {
        assert!(is_input("p1-a.in"), "input");
        assert!(!is_input("p1-a.out"), "expectation");
        assert!(!is_input("p1-a.in.bak"), "backup");
    }
}
