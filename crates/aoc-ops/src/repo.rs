//! The workspace, its paths, and filesystem helpers with path-carrying errors.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use aoc_core::{Day, Year};

use crate::cli::{Exploration, Variant};
use crate::error::OpsError;

/// The live site.
const BASE_URL: &str = "https://adventofcode.com/";

/// Where operations run and which external programs they call; tests substitute fakes.
#[derive(Debug, Clone)]
pub(crate) struct Workspace {
    pub(crate) root: PathBuf,
    /// Base URL for requests, ending in `/`.
    pub(crate) base_url: String,
    /// Directory holding release binaries.
    pub(crate) release: PathBuf,
    pub(crate) cargo: OsString,
    pub(crate) perf: OsString,
    /// GNU time, not the shell keyword.
    pub(crate) time: OsString,
}

impl Workspace {
    /// The workspace around the current directory, using the live site and real tools.
    pub(crate) fn discover() -> Result<Self, OpsError> {
        let current = std::env::current_dir().map_err(|source| OpsError::read(".", source))?;
        let root = current
            .ancestors()
            .find(|path| path.join("crates/aoc-ops/Cargo.toml").is_file())
            .ok_or(OpsError::Root)?
            .to_path_buf();
        // Cargo's release output, honouring `CARGO_TARGET_DIR`.
        let release = std::env::var_os("CARGO_TARGET_DIR")
            .map_or_else(|| root.join("target"), PathBuf::from)
            .join("release");
        Ok(Self {
            root,
            base_url: BASE_URL.to_owned(),
            release,
            cargo: "cargo".into(),
            perf: "perf".into(),
            time: "time".into(),
        })
    }

    pub(crate) fn input_path(&self, year: Year, day: Day) -> PathBuf {
        self.root.join(format!("inputs/{year}/day{day}.txt"))
    }

    pub(crate) fn puzzle_path(&self, year: Year, day: Day) -> PathBuf {
        self.root.join(format!("puzzles/{year}/{day}.txt"))
    }

    pub(crate) fn fixture_dir(&self, year: Year, day: Day) -> PathBuf {
        self.root.join(format!("fixtures/{year}/{day}"))
    }

    /// The directory of a day's solution modules: `crates/aocYY/src/dayDD`.
    pub(crate) fn day_dir(&self, year: Year, day: Day) -> PathBuf {
        self.root
            .join("crates")
            .join(year.crate_name())
            .join(format!("src/day{day}"))
    }
}

/// Runs `<cargo> build --release --locked -p <crate> --bin <binary>...` for the given binaries.
pub(crate) fn build<'a>(
    workspace: &Workspace,
    binaries: impl IntoIterator<Item = (Year, &'a str)>,
) -> Result<(), OpsError> {
    let mut args: Vec<String> = vec!["build".into(), "--release".into(), "--locked".into()];
    for (year, binary) in binaries {
        args.extend([
            "-p".into(),
            year.crate_name(),
            "--bin".into(),
            binary.to_owned(),
        ]);
    }
    crate::process::visible(&workspace.root, &workspace.cargo, args)
}

pub(crate) fn read_text(path: &Path) -> Result<String, OpsError> {
    let bytes = fs::read(path).map_err(|source| OpsError::read(path, source))?;
    String::from_utf8(bytes).map_err(|_utf8| OpsError::Utf8 {
        path: path.to_path_buf(),
    })
}

pub(crate) fn create_dir(path: &Path) -> Result<(), OpsError> {
    fs::create_dir_all(path).map_err(|source| OpsError::write(path, source))
}

/// Writes through a sibling `.tmp` file and a rename, so readers never see partial content.
pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), OpsError> {
    if let Some(parent) = path.parent() {
        create_dir(parent)?;
    }
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    let temporary = PathBuf::from(temporary);
    fs::write(&temporary, bytes)
        .and_then(|()| fs::rename(&temporary, path))
        .map_err(|source| {
            // The rename failed or the write was partial; leave no stray temporary behind.
            match fs::remove_file(&temporary) {
                Ok(()) | Err(_) => OpsError::write(path, source),
            }
        })
}

/// Whether a file name ends in the `.rs` extension.
pub(crate) fn is_rust_file(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|extension| extension == "rs")
}

/// Lists the entries of `dir` whose file names satisfy `keep`, sorted; a missing directory is empty.
pub(crate) fn list(dir: &Path, keep: impl Fn(&str) -> bool) -> Result<Vec<PathBuf>, OpsError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(OpsError::read(dir, source)),
    };
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry.map_err(|source| OpsError::read(dir, source))?.path();
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(&keep)
        {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

/// A path's final component as UTF-8, if any.
pub(crate) fn file_name(path: &Path) -> Option<&str> {
    path.file_name().and_then(|name| name.to_str())
}

/// The solutions of a day directory: the primary first, then explorations by name.
pub(crate) fn variants(day_dir: &Path) -> Result<Vec<Variant>, OpsError> {
    if !day_dir.join("primary.rs").is_file() {
        return Err(OpsError::Scaffold(format!(
            "no primary solution in {}",
            day_dir.display()
        )));
    }
    let mut variants = vec![Variant::Primary];
    for path in list(day_dir, |name| is_rust_file(name) && name != "primary.rs")? {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        variants.push(Variant::Exploration(Exploration::parse(stem)?));
    }
    Ok(variants)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{atomic_write, list, read_text};

    #[test]
    fn atomic_write_replaces_content_and_leaves_no_temporary() {
        let dir = std::env::temp_dir().join(format!("aoc-ops-repo-{}", std::process::id()));
        let path = dir.join("nested/file.txt");
        atomic_write(&path, b"one").unwrap();
        atomic_write(&path, b"two").unwrap();
        assert_eq!(read_text(&path).unwrap(), "two", "content is replaced");
        let names = list(&dir.join("nested"), |_| true).unwrap();
        assert_eq!(names, [path], "no .tmp file remains");
        assert!(
            list(&dir.join("missing"), |_| true).unwrap().is_empty(),
            "missing dir"
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
