//! User-facing stdout and stderr lines; a closed stdout pipe counts as delivered.

use std::io::{self, ErrorKind, Write};

use crate::error::OpsError;

/// Writes one line and flushes; a closed pipe is success, other failures are errors.
fn write_line(mut writer: impl Write, text: &str) -> Result<(), OpsError> {
    match writeln!(writer, "{text}").and_then(|()| writer.flush()) {
        Err(error) if error.kind() != ErrorKind::BrokenPipe => {
            Err(OpsError::write("stdout", error))
        }
        Ok(()) | Err(_) => Ok(()),
    }
}

/// Writes one line to stdout.
pub(crate) fn line(text: &str) -> Result<(), OpsError> {
    write_line(io::stdout().lock(), text)
}

/// Writes one diagnostic line to stderr; a failing stderr has nowhere left to report to.
pub(crate) fn diagnostic(text: &str) {
    match writeln!(io::stderr().lock(), "{text}") {
        Ok(()) | Err(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, ErrorKind, Write};

    use super::write_line;

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
    fn lines_end_with_a_newline() {
        let mut buffer = Vec::new();
        write_line(&mut buffer, "a").unwrap();
        assert_eq!(buffer, b"a\n", "one terminated line");
    }

    #[test]
    fn closed_pipes_are_success_and_other_failures_are_not() {
        assert!(
            write_line(Failing(ErrorKind::BrokenPipe), "a").is_ok(),
            "a closed reader is not an error"
        );
        assert!(
            write_line(Failing(ErrorKind::PermissionDenied), "a").is_err(),
            "other write failures propagate"
        );
    }
}
