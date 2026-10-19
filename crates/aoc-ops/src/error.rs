//! Failures of workflow operations; none carries the session secret or puzzle answers.

use std::io;
use std::path::PathBuf;

/// A failed operation. Every variant maps to exit code 4.
#[derive(Debug, thiserror::Error)]
pub(crate) enum OpsError {
    #[error("{0}")]
    Usage(String),
    #[error(transparent)]
    Calendar(#[from] aoc_core::CalendarError),
    #[error("cannot find the workspace root above the current directory")]
    Root,
    #[error("cannot read {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("cannot write {path}")]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{path} is not UTF-8")]
    Utf8 { path: PathBuf },
    #[error("cannot run {program}")]
    Spawn {
        program: String,
        #[source]
        source: io::Error,
    },
    #[error("{program} failed with {status}")]
    Child { program: String, status: String },
    #[error("AOC_SESSION not set in .env")]
    SessionMissing,
    #[error("AOC_SESSION in .env is not a valid cookie value")]
    SessionInvalid,
    #[error("{0}")]
    Http(String),
    #[error("{0}")]
    Html(&'static str),
    #[error("refusing to overwrite {0}")]
    Exists(PathBuf),
    #[error("{0}")]
    Scaffold(String),
    #[error("{0}")]
    Bench(String),
    #[error("scaffold failed and rollback also failed for {paths}")]
    Rollback {
        paths: String,
        #[source]
        source: Box<Self>,
    },
}

impl OpsError {
    pub(crate) fn read(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Read {
            path: path.into(),
            source,
        }
    }

    pub(crate) fn write(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Write {
            path: path.into(),
            source,
        }
    }
}
