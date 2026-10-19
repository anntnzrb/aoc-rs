//! Ordered, nonempty line-record parsing with borrowed records and contextual errors.

/// Failure to parse a nonempty sequence of line records.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RecordsError<E> {
    /// The input contains no lines.
    #[error("input is empty")]
    Empty,
    /// A record parser rejected a line.
    #[error("invalid record on line {line}")]
    Line {
        /// The 1-based input line number.
        line: usize,
        /// The caller's record parsing error.
        #[source]
        source: E,
    },
}

/// Parses each input line in order, rejecting input with no records.
///
/// Uses [`str::lines`]: CRLF terminators are removed, a final newline does not
/// add a record, and interior empty lines are passed to the caller unchanged.
/// Records may borrow from `input`; only the output vector is allocated by this
/// function. Parsing stops at the first error.
///
/// # Errors
///
/// Returns [`RecordsError::Empty`] for input with no lines, or
/// [`RecordsError::Line`] with the first rejected line's number and cause.
///
/// # Examples
///
/// ```
/// use aoc_core::parse_records;
///
/// let records = parse_records("12\r\n34\r\n", str::parse::<u32>)?;
/// assert_eq!(records, [12, 34]);
/// # Ok::<(), aoc_core::RecordsError<std::num::ParseIntError>>(())
/// ```
pub fn parse_records<'a, T, E>(
    input: &'a str,
    mut parse_line: impl FnMut(&'a str) -> Result<T, E>,
) -> Result<Vec<T>, RecordsError<E>> {
    let records = input
        .lines()
        .zip(1..)
        .map(|(text, line)| parse_line(text).map_err(|source| RecordsError::Line { line, source }))
        .collect::<Result<Vec<_>, _>>()?;
    if records.is_empty() {
        return Err(RecordsError::Empty);
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use std::{convert::Infallible, error::Error};

    use super::{RecordsError, parse_records};

    #[test]
    fn empty_input_is_rejected() {
        assert_eq!(
            parse_records("", Ok::<_, Infallible>),
            Err(RecordsError::Empty),
            "no record parser is needed for empty input"
        );
    }

    #[test]
    fn records_keep_their_order() {
        assert_eq!(
            parse_records("8\n3\n5\n", str::parse::<u32>).unwrap(),
            [8, 3, 5],
            "records preserve input order"
        );
    }

    #[test]
    fn first_error_retains_line_and_source() {
        let mut visited = Vec::new();
        let error = parse_records("7\nwrong\nalso wrong", |line| {
            visited.push(line);
            line.parse::<u32>()
        })
        .unwrap_err();
        assert_eq!(visited, ["7", "wrong"], "stop at the first error");
        assert!(
            matches!(error, RecordsError::Line { line: 2, .. }),
            "line numbers start at one"
        );
        assert_eq!(error.to_string(), "invalid record on line 2", "diagnostic");
        assert!(
            error.source().unwrap().is::<std::num::ParseIntError>(),
            "the original error remains the source"
        );
    }

    #[test]
    fn crlf_and_empty_lines_follow_str_lines() {
        assert_eq!(
            parse_records("a\r\n\r\nb\r\n", Ok::<_, Infallible>).unwrap(),
            ["a", "", "b"],
            "strip CRLF but preserve interior empty records"
        );
        assert_eq!(
            parse_records("\n", Ok::<_, Infallible>).unwrap(),
            [""],
            "an empty line is a record, not absent input"
        );
    }

    #[test]
    fn records_borrow_the_input() {
        let input = String::from("alpha\nbeta");
        let records = parse_records(&input, Ok::<_, Infallible>).unwrap();
        assert_eq!(records, ["alpha", "beta"], "borrowed record contents");
        assert_eq!(
            records[0].as_ptr(),
            input.as_ptr(),
            "the record uses the input's storage"
        );
    }
}
