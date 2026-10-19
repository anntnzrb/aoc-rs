//! Child processes with separate arguments, an explicit working directory and fed stdin.

use std::ffi::OsStr;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use crate::error::OpsError;

/// A finished child process.
#[derive(Debug)]
pub(crate) struct Finished {
    pub(crate) success: bool,
    pub(crate) status: String,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
    pub(crate) wall: Duration,
}

/// An empty argument list for programs run without arguments.
pub(crate) const NO_ARGS: [&str; 0] = [];

fn name(program: &OsStr) -> String {
    program.to_string_lossy().into_owned()
}

/// Runs `program` in `dir`, writes `input` to its stdin, and captures both output streams.
///
/// A writer thread feeds stdin while the caller drains stdout and stderr, so large inputs
/// and outputs cannot deadlock on full pipes.
pub(crate) fn capture<I, S>(
    dir: &Path,
    program: impl AsRef<OsStr>,
    args: I,
    input: &[u8],
) -> Result<Finished, OpsError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let program = program.as_ref();
    let spawn_error = |source| OpsError::Spawn {
        program: name(program),
        source,
    };
    let started = Instant::now();
    let mut child = Command::new(program)
        .args(args)
        .current_dir(dir)
        .env("LC_ALL", "C")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(spawn_error)?;
    let stdin = child.stdin.take();
    let (written, output) = std::thread::scope(|scope| {
        let writer = scope.spawn(move || {
            stdin.map_or(Ok(()), |mut pipe| match pipe.write_all(input) {
                // A child that stops reading early is judged by its own exit status.
                Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
                other => other,
            })
        });
        let output = child.wait_with_output();
        (writer.join(), output)
    });
    let wall = started.elapsed();
    let Output {
        status,
        stdout,
        stderr,
    } = output.map_err(spawn_error)?;
    match written {
        Ok(Ok(())) => {}
        Ok(Err(source)) => return Err(spawn_error(source)),
        Err(_panic) => {
            return Err(OpsError::Child {
                program: name(program),
                status: "stdin writer panicked".to_owned(),
            });
        }
    }
    Ok(Finished {
        success: status.success(),
        status: status.to_string(),
        stdout,
        stderr,
        wall,
    })
}

/// Runs a tool and returns its trimmed stdout, failing on a non-zero exit with stderr withheld.
pub(crate) fn tool<I, S>(dir: &Path, program: &str, args: I) -> Result<String, OpsError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let finished = capture(dir, program, args, &[])?;
    if !finished.success {
        return Err(OpsError::Child {
            program: program.to_owned(),
            status: finished.status,
        });
    }
    Ok(String::from_utf8_lossy(&finished.stdout).trim().to_owned())
}

/// Runs a tool with inherited stdio so its progress and diagnostics reach the user.
pub(crate) fn visible<I, S>(dir: &Path, program: impl AsRef<OsStr>, args: I) -> Result<(), OpsError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let program = program.as_ref();
    let status = Command::new(program)
        .args(args)
        .current_dir(dir)
        .status()
        .map_err(|source| OpsError::Spawn {
            program: name(program),
            source,
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(OpsError::Child {
            program: name(program),
            status: status.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{NO_ARGS, capture, tool};

    #[test]
    fn capture_round_trips_large_stdin_without_deadlock() {
        let input = vec![b'x'; 1 << 20];
        let finished = capture(&std::env::temp_dir(), "cat", ["-"], &input).unwrap();
        assert!(finished.success, "cat succeeds");
        assert_eq!(finished.stdout.len(), input.len(), "every byte returns");
    }

    #[test]
    fn tool_reports_failures_and_trims_output() {
        let dir = std::env::temp_dir();
        assert_eq!(
            tool(&dir, "echo", ["  hi  "]).unwrap(),
            "hi",
            "output is trimmed"
        );
        assert!(tool(&dir, "false", NO_ARGS).is_err(), "non-zero exit fails");
        assert!(
            tool(&dir, "aoc-ops-missing-program", NO_ARGS).is_err(),
            "spawn fails"
        );
    }
}
