//! The effectful edge: dispatches validated commands to the filesystem, network and processes.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aoc_core::{Day, Year};

use crate::cli::{self, Command, Refresh, Scope, Variant};
use crate::error::OpsError;
use crate::repo::{self, Workspace};
use crate::scaffold::{self, Tree};
use crate::submission::Verdict;
use crate::{benchmark, doc, fixtures, http, output, process, session, statement};

/// Exit code for local, HTTP and transport failures; 0 through 3 are submission verdicts.
const FAILURE: u8 = 4;

/// Runs `aoc-ops` with the given arguments (excluding the program name).
pub fn run(args: impl Iterator<Item = String>) -> ExitCode {
    let args: Vec<String> = args.collect();
    report(cli::parse(&args).and_then(|command| execute(&Workspace::discover()?, command)))
}

/// Maps an outcome to its exit code, printing a failure with its causes on stderr.
fn report(outcome: Result<ExitCode, OpsError>) -> ExitCode {
    outcome.unwrap_or_else(|error| {
        output::diagnostic(&describe(&error));
        ExitCode::from(FAILURE)
    })
}

/// Renders an error and its source chain as one line.
fn describe(error: &OpsError) -> String {
    let mut text = error.to_string();
    let mut cause = std::error::Error::source(error);
    while let Some(current) = cause {
        text.push_str(": ");
        text.push_str(&current.to_string());
        cause = current.source();
    }
    text
}

pub(crate) fn execute(workspace: &Workspace, command: Command) -> Result<ExitCode, OpsError> {
    match command {
        Command::Fetch(year, day, refresh) => fetch(workspace, year, day, refresh),
        Command::Puzzle(year, day) => puzzle(workspace, year, day),
        Command::Submit(year, day, part, answer) => {
            let session = load_session(&workspace.root)?;
            let html = http::request(
                &workspace.root,
                &workspace.base_url,
                &session,
                &format!("{year}/day/{}/answer", day.get()),
                &[("level", part.level()), ("answer", answer.as_str())],
            )?;
            let text = statement::submission(&html).map_or_else(
                |_missing| "no submission article found\n".to_owned(),
                |text| text.replace(session.as_str(), "[redacted]"),
            );
            output::line(text.trim_end())?;
            Ok(ExitCode::from(Verdict::classify(&text).code()))
        }
        Command::New(year, day, variant) => new(workspace, year, day, &variant),
        Command::Run(year, day, variant) => {
            let input = workspace.input_path(year, day);
            let binary = variant.binary(day);
            let stdin =
                std::fs::File::open(&input).map_err(|source| OpsError::read(&input, source))?;
            repo::build(workspace, [(year, binary.as_str())])?;
            let status = std::process::Command::new(workspace.release.join(&binary))
                .current_dir(&workspace.root)
                .stdin(stdin)
                .status()
                .map_err(|source| OpsError::Spawn {
                    program: binary.clone(),
                    source,
                })?;
            Ok(if status.success() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(FAILURE)
            })
        }
        Command::Test(scope) => test(workspace, scope),
        Command::Bench(year, day, record) => {
            benchmark::run(workspace, year, day, record)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Doc => check_docs(workspace),
    }
}

/// Reads `AOC_SESSION` from `.env`; only network operations call this.
fn load_session(root: &Path) -> Result<session::Session, OpsError> {
    let path = root.join(".env");
    let bytes = zeroize::Zeroizing::new(match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(OpsError::SessionMissing);
        }
        Err(source) => return Err(OpsError::read(path, source)),
    });
    let text = std::str::from_utf8(&bytes).map_err(|_utf8| OpsError::SessionInvalid)?;
    session::parse(text)
}

fn fetch(
    workspace: &Workspace,
    year: Year,
    day: Day,
    refresh: Refresh,
) -> Result<ExitCode, OpsError> {
    let path = workspace.input_path(year, day);
    if refresh == Refresh::Force || !path.is_file() {
        let session = load_session(&workspace.root)?;
        let body = http::request(
            &workspace.root,
            &workspace.base_url,
            &session,
            &format!("{year}/day/{}/input", day.get()),
            &[],
        )?;
        repo::atomic_write(&path, body.as_bytes())?;
    }
    output::line(&path.display().to_string())?;
    Ok(ExitCode::SUCCESS)
}

fn puzzle(workspace: &Workspace, year: Year, day: Day) -> Result<ExitCode, OpsError> {
    let session = load_session(&workspace.root)?;
    let html = http::request(
        &workspace.root,
        &workspace.base_url,
        &session,
        &format!("{year}/day/{}", day.get()),
        &[],
    )?;
    let text = statement::extract(&html)?;
    let path = workspace.puzzle_path(year, day);
    repo::atomic_write(&path, text.as_bytes())?;
    output::line(&path.display().to_string())?;
    Ok(ExitCode::SUCCESS)
}

/// Reads the real workspace for scaffold planning.
struct Disk<'a>(&'a Path);

impl Tree for Disk<'_> {
    fn read(&self, path: &Path) -> Result<Option<String>, OpsError> {
        let full = self.0.join(path);
        if full.is_dir() {
            Ok(Some(String::new()))
        } else if full.exists() {
            repo::read_text(&full).map(Some)
        } else {
            Ok(None)
        }
    }
}

/// Reads a file that may not exist yet.
fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, OpsError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(OpsError::read(path, source)),
    }
}

fn new(
    workspace: &Workspace,
    year: Year,
    day: Day,
    variant: &Variant,
) -> Result<ExitCode, OpsError> {
    let root = &workspace.root;
    let mut writes = scaffold::plan(&Disk(root), year, day, variant)?;
    let fixture = workspace.fixture_dir(year, day);
    let fixture_existed = fixture.exists();
    let mut originals: Vec<(PathBuf, Option<Vec<u8>>)> = Vec::with_capacity(writes.len());
    for path in writes
        .iter()
        .map(|write| root.join(&write.path))
        .chain([root.join("Cargo.lock")])
    {
        let bytes = read_optional(&path)?;
        originals.push((path, bytes));
    }
    let krate = year.crate_name();
    let applied = writes
        .iter()
        .try_for_each(|write| repo::atomic_write(&root.join(&write.path), write.text.as_bytes()))
        .and_then(|()| repo::create_dir(&fixture))
        // A new year crate is a new lockfile entry; record it without touching dependencies.
        .and_then(|()| {
            process::visible(root, &workspace.cargo, ["update", "--workspace", "--offline"])
        })
        .and_then(|()| process::visible(root, &workspace.cargo, ["fmt", "-p", krate.as_str()]));
    if let Err(error) = applied {
        if !fixture_existed {
            remove_empty_dirs(root, &fixture);
        }
        return Err(rollback(root, &originals, error));
    }
    writes.sort_by(|a, b| a.path.cmp(&b.path));
    for write in &writes {
        output::line(&write.path.display().to_string())?;
    }
    output::line(&fixture.display().to_string())?;
    Ok(ExitCode::SUCCESS)
}

/// Removes `dir` and then each parent while empty, stopping below `root`.
fn remove_empty_dirs(root: &Path, dir: &Path) {
    for current in dir.ancestors().take_while(|p| *p != root) {
        if std::fs::remove_dir(current).is_err() {
            break;
        }
    }
}

/// Restores every touched file and removes directories the scaffold left empty.
fn rollback(root: &Path, originals: &[(PathBuf, Option<Vec<u8>>)], error: OpsError) -> OpsError {
    let mut failed = Vec::new();
    for (path, bytes) in originals {
        let restored = bytes.as_ref().map_or_else(
            || match std::fs::remove_file(path) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                other => other,
            },
            |bytes| std::fs::write(path, bytes),
        );
        if restored.is_err() {
            failed.push(path.display().to_string());
        }
        if let Some(parent) = path.parent() {
            remove_empty_dirs(root, parent);
        }
    }
    if failed.is_empty() {
        error
    } else {
        OpsError::Rollback {
            paths: failed.join(", "),
            source: Box::new(error),
        }
    }
}

/// The year of a crate directory name `aocYY`.
fn crate_year(name: &str) -> Option<Year> {
    let digits = name.strip_prefix("aoc")?;
    if digits.len() != 2 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Year::new(digits.parse::<u16>().ok()?.checked_add(2000)?).ok()
}

/// The day of a module directory name `dayDD` within `year`.
fn module_day(year: Year, name: &str) -> Option<Day> {
    let digits = name.strip_prefix("day")?;
    if digits.len() != 2 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Day::new(year, digits.parse().ok()?).ok()
}

/// The days that have a primary solution, filtered by scope.
fn started_days(root: &Path, scope: Scope) -> Result<Vec<(Year, Day)>, OpsError> {
    let mut days = Vec::new();
    for krate in repo::list(&root.join("crates"), |name| crate_year(name).is_some())? {
        let Some(year) = repo::file_name(&krate).and_then(crate_year) else {
            continue;
        };
        for dir in repo::list(&krate.join("src"), |name| module_day(year, name).is_some())? {
            let Some(day) = repo::file_name(&dir).and_then(|name| module_day(year, name)) else {
                continue;
            };
            let wanted = match scope {
                Scope::All => true,
                Scope::Year(y) => y == year,
                Scope::Day(y, d) => y == year && d == day,
            };
            if wanted && dir.join("primary.rs").is_file() {
                days.push((year, day));
            }
        }
    }
    Ok(days)
}

/// Runs one fixture: `Ok(Ok)` passes, `Ok(Err)` is a wrong answer or failed run.
fn run_fixture(binary: &Path, root: &Path, input: &Path) -> Result<Result<(), String>, OpsError> {
    let bytes = std::fs::read(input).map_err(|source| OpsError::read(input, source))?;
    let expected = repo::read_text(&input.with_extension("out"))?;
    let finished = process::capture(root, binary, process::NO_ARGS, &bytes)?;
    if !finished.success {
        return Ok(Err(format!("exited with {}", finished.status)));
    }
    Ok(fixtures::check(
        &expected,
        &String::from_utf8_lossy(&finished.stdout),
    ))
}

fn test(workspace: &Workspace, scope: Scope) -> Result<ExitCode, OpsError> {
    let mut binaries = Vec::new();
    for (year, day) in started_days(&workspace.root, scope)? {
        for variant in repo::variants(&workspace.day_dir(year, day))? {
            binaries.push((year, day, variant.binary(day)));
        }
    }
    if binaries.is_empty() {
        output::line("no solutions to test")?;
        return Ok(ExitCode::SUCCESS);
    }
    repo::build(
        workspace,
        binaries
            .iter()
            .map(|(year, _, binary)| (*year, binary.as_str())),
    )?;
    let (mut passed, mut failed) = (0_usize, 0_usize);
    for (year, day, binary) in &binaries {
        for input in repo::list(&workspace.fixture_dir(*year, *day), fixtures::is_input)? {
            let file = repo::file_name(&input).unwrap_or_default();
            let verdict =
                match run_fixture(&workspace.release.join(binary), &workspace.root, &input) {
                    Ok(Ok(())) => None,
                    Ok(Err(reason)) => Some(reason),
                    Err(error) => Some(describe(&error)),
                };
            if let Some(reason) = verdict {
                failed = failed.saturating_add(1);
                output::line(&format!("FAIL {year}/{day}/{binary}/{file}: {reason}"))?;
            } else {
                passed = passed.saturating_add(1);
                output::line(&format!("PASS {year}/{day}/{binary}/{file}"))?;
            }
        }
    }
    output::line(&format!("{passed} passed, {failed} failed"))?;
    Ok(if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn check_docs(workspace: &Workspace) -> Result<ExitCode, OpsError> {
    let root = &workspace.root;
    let mut violations = Vec::new();
    for (year, day) in started_days(root, Scope::All)? {
        for file in repo::list(&workspace.day_dir(year, day), repo::is_rust_file)? {
            let relative = file
                .strip_prefix(root)
                .map_or_else(|_| file.clone(), Path::to_path_buf)
                .to_string_lossy()
                .replace('\\', "/");
            violations.extend(doc::check(&relative, &repo::read_text(&file)?));
        }
    }
    for violation in &violations {
        output::line(violation)?;
    }
    Ok(if violations.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

#[cfg(test)]
mod tests {
    use super::{crate_year, describe, module_day};
    use crate::error::OpsError;
    use aoc_core::Year;

    #[test]
    fn crate_and_module_names_map_to_dates() {
        assert_eq!(crate_year("aoc18").map(Year::get), Some(2018), "crate");
        for bad in [
            "aoc", "aoc1", "aoc180", "aocx8", "aoc-ops", "abc18", "aoc14", "aoc+18", "aoc1+",
        ] {
            assert_eq!(crate_year(bad), None, "{bad} is not a year crate");
        }
        let year = Year::new(2025).unwrap();
        assert_eq!(
            module_day(year, "day12").map(aoc_core::Day::get),
            Some(12),
            "day"
        );
        for bad in [
            "day1", "day123", "dayx1", "day13", "day00", "dax01", "day01.rs", "day+1",
        ] {
            assert_eq!(module_day(year, bad), None, "{bad} is not a 2025 day");
        }
    }

    #[test]
    fn descriptions_include_the_cause_chain() {
        let error = OpsError::read("f", std::io::Error::other("disk"));
        assert_eq!(describe(&error), "cannot read f: disk", "cause appended");
    }
}

#[cfg(test)]
mod workflow_tests;
