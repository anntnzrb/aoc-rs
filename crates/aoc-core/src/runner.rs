//! The stdin-to-stdout filter shared by every day binary.

use std::error::Error;
use std::io::{self, ErrorKind, Read, Write};
use std::process::ExitCode;

use crate::Answers;

/// Largest accepted puzzle input, 64 MiB; real inputs stay far below this bound.
pub const MAX_INPUT_BYTES: u64 = 67_108_864;

/// A failure while running a solver as a filter.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RunError<E> {
    /// Reading the input failed.
    #[error("cannot read input")]
    Read(#[source] io::Error),
    /// The input exceeds [`MAX_INPUT_BYTES`].
    #[error("input exceeds {MAX_INPUT_BYTES} bytes")]
    TooLarge,
    /// The input is not UTF-8.
    #[error("input is not UTF-8")]
    Utf8(#[source] std::string::FromUtf8Error),
    /// The solver rejected the input.
    #[error("cannot solve input")]
    Solve(#[source] E),
    /// Writing the answers failed.
    #[error("cannot write answers")]
    Write(#[source] io::Error),
}

/// Reads `input`, solves it and writes the rendered answers to `output`.
///
/// Empty input writes nothing and succeeds without calling `solve`; benchmarks use
/// that run as the process-startup baseline. A closed output pipe counts as success.
///
/// # Errors
///
/// Returns [`RunError`] when reading, decoding, solving or writing fails.
///
/// # Examples
///
/// ```
/// use aoc_core::{Answers, execute};
///
/// let solve = |text: &str| Ok::<_, std::convert::Infallible>(Answers::new(text.len(), 0));
/// let mut output = Vec::new();
/// execute(&b"abc"[..], &mut output, solve)?;
/// assert_eq!(output, b"3\n0\n");
/// # Ok::<(), aoc_core::RunError<std::convert::Infallible>>(())
/// ```
pub fn execute<E>(
    input: impl Read,
    mut output: impl Write,
    solve: impl FnOnce(&str) -> Result<Answers, E>,
) -> Result<(), RunError<E>> {
    let mut bytes = Vec::new();
    input
        .take(MAX_INPUT_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(RunError::Read)?;
    if u64::try_from(bytes.len()).map_or(true, |length| length > MAX_INPUT_BYTES) {
        return Err(RunError::TooLarge);
    }
    if bytes.is_empty() {
        return Ok(());
    }
    let text = String::from_utf8(bytes).map_err(RunError::Utf8)?;
    let answers = solve(&text).map_err(RunError::Solve)?;
    match output
        .write_all(answers.to_string().as_bytes())
        .and_then(|()| output.flush())
    {
        Err(error) if error.kind() != ErrorKind::BrokenPipe => Err(RunError::Write(error)),
        Ok(()) | Err(_) => Ok(()),
    }
}

/// Runs `solve` over stdin and stdout, reporting failures with their causes on stderr.
///
/// Day binaries call this from `main` and return its exit code.
#[must_use = "return the exit code from main"]
pub fn run<E: Error + 'static>(solve: impl FnOnce(&str) -> Result<Answers, E>) -> ExitCode {
    match execute(io::stdin().lock(), io::stdout().lock(), solve) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let mut stderr = io::stderr().lock();
            let mut cause: Option<&dyn Error> = Some(&error);
            let mut text = String::from("error");
            while let Some(current) = cause {
                text.push_str(": ");
                text.push_str(&current.to_string());
                cause = current.source();
            }
            match writeln!(stderr, "{text}") {
                Ok(()) | Err(_) => ExitCode::FAILURE,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::io::{self, ErrorKind, Write};

    use super::{MAX_INPUT_BYTES, RunError, execute};
    use crate::Answers;

    #[derive(Debug, PartialEq, Eq)]
    struct Rejected;

    /// A solver that answers with the input length, exercising the success path.
    fn length() -> impl Fn(&str) -> Result<Answers, Infallible> {
        |text| Ok(Answers::new(text.len(), 0))
    }

    struct Failing(ErrorKind);

    impl Write for Failing {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(self.0.into())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn writes_rendered_answers() {
        let mut output = Vec::new();
        let result = execute(&b"hello"[..], &mut output, length());
        assert!(matches!(result, Ok(())), "solving succeeds");
        assert_eq!(output, b"5\n0\n", "answers are rendered");
    }

    #[test]
    fn empty_input_is_the_startup_baseline() {
        let mut output = Vec::new();
        let result = execute(
            &b""[..],
            &mut output,
            |_: &str| -> Result<Answers, Rejected> { Err(Rejected) },
        );
        assert!(matches!(result, Ok(())), "the solver is not called");
        assert!(output.is_empty(), "nothing is printed");
    }

    #[test]
    fn solver_errors_are_propagated() {
        let result = execute(
            &b"x"[..],
            Vec::new(),
            |_: &str| -> Result<Answers, Rejected> { Err(Rejected) },
        );
        assert!(
            matches!(result, Err(RunError::Solve(Rejected))),
            "solver error is kept"
        );
    }

    #[test]
    fn invalid_utf8_is_rejected() {
        let result = execute(&[0xff_u8][..], Vec::new(), length());
        assert!(
            matches!(result, Err(RunError::Utf8(_))),
            "bytes must be UTF-8"
        );
    }

    #[test]
    fn oversized_input_is_rejected_at_the_bound() {
        let limit = usize::try_from(MAX_INPUT_BYTES).unwrap_or(usize::MAX);
        let exact = vec![b'a'; limit];
        let mut output = Vec::new();
        assert!(
            matches!(execute(&exact[..], &mut output, length()), Ok(())),
            "the bound itself is accepted"
        );
        let over = vec![b'a'; limit + 1];
        assert!(
            matches!(
                execute(&over[..], Vec::new(), length()),
                Err(RunError::TooLarge)
            ),
            "one byte over the bound fails"
        );
    }

    #[test]
    fn closed_pipe_is_success_but_other_write_errors_fail() {
        assert!(
            matches!(
                execute(&b"x"[..], Failing(ErrorKind::BrokenPipe), length()),
                Ok(())
            ),
            "a closed reader is not an error"
        );
        assert!(
            matches!(
                execute(&b"x"[..], Failing(ErrorKind::PermissionDenied), length()),
                Err(RunError::Write(_))
            ),
            "other write failures propagate"
        );
    }
}
