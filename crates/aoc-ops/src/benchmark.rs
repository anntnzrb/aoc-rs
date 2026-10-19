//! The `bench` and `record` operations: verify agreement, then measure release binaries.
//!
//! Output never contains answers: disagreement and failures are reported by solution name.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use aoc_core::{Day, Year};

use crate::bench::{self, CSV_HEADER, Measurement, Metadata, Summary};
use crate::cli::{Record, Variant};
use crate::error::OpsError;
use crate::output;
use crate::process::{self, Finished};
use crate::repo::{self, Workspace};

/// Full/empty instruction-count pairs per solution.
const WORK_PAIRS: usize = 5;
/// Timing runs per solution.
const TIMING_RUNS: usize = 10;
/// Timing runs when the pilot run exceeds [`SLOW_PILOT`].
const SLOW_TIMING_RUNS: usize = 3;
/// Pilot wall time above which a solution counts as slow.
const SLOW_PILOT: Duration = Duration::from_secs(1);

/// One timed run: wall µs, CPU µs and peak RSS KiB.
#[derive(Debug, Clone, Copy)]
struct Timing {
    wall: i128,
    cpu: i128,
    rss: i128,
}

/// A solution under measurement, its binary and the samples gathered so far.
struct Candidate {
    variant: Variant,
    binary: PathBuf,
    runs: usize,
    /// Full and empty-input instruction counts.
    work: Vec<(i128, i128)>,
    timings: Vec<Timing>,
}

impl Candidate {
    fn label(&self) -> &str {
        self.variant.label()
    }

    fn measurement(&self) -> Result<Measurement, OpsError> {
        let summarize = |values: Vec<i128>| {
            Summary::of(&values).ok_or_else(|| bench_error("no timing samples"))
        };
        Ok(Measurement {
            solution: self.label().to_owned(),
            runs: self.timings.len(),
            work: bench::work(&self.work),
            instructions: Summary::of(&self.work.iter().map(|(full, _)| *full).collect::<Vec<_>>()),
            wall: summarize(self.timings.iter().map(|t| t.wall).collect())?,
            cpu: summarize(self.timings.iter().map(|t| t.cpu).collect())?,
            rss_kib: self.timings.iter().map(|t| t.rss).max().unwrap_or(0),
        })
    }
}

/// A measuring wrapper placed before the solution binary.
#[derive(Clone, Copy)]
enum Probe {
    Bare,
    Perf,
    Time,
}

/// Launches measured processes from the workspace root, pinned to one CPU when possible.
struct Launcher<'a> {
    workspace: &'a Workspace,
    cpu: Option<String>,
    expected: Vec<u8>,
    input: Vec<u8>,
}

impl Launcher<'_> {
    fn run(&self, probe: Probe, binary: &Path, input: &[u8]) -> Result<Finished, OpsError> {
        let mut words: Vec<OsString> = match probe {
            Probe::Bare => Vec::new(),
            Probe::Perf => vec![
                self.workspace.perf.clone(),
                "stat".into(),
                "-x,".into(),
                "-e".into(),
                "instructions:u".into(),
                "--".into(),
            ],
            Probe::Time => vec![
                self.workspace.time.clone(),
                "-f".into(),
                "%U,%S,%M".into(),
                "--".into(),
            ],
        };
        if let Some(cpu) = &self.cpu {
            words.extend(["taskset".into(), "-c".into(), cpu.into()]);
        }
        words.push(binary.as_os_str().to_owned());
        let (program, args) = words
            .split_first()
            .ok_or_else(|| bench_error("empty command"))?;
        process::capture(
            &self.workspace.root,
            program,
            args.iter().map(OsString::as_os_str),
            input,
        )
    }

    /// Runs on the real input and requires the primary's answers.
    fn full(&self, probe: Probe, candidate: &Candidate) -> Result<Finished, OpsError> {
        let finished = self.run(probe, &candidate.binary, &self.input)?;
        agree(candidate.label(), &self.expected, &finished)?;
        Ok(finished)
    }

    /// Runs on empty input and requires no output: the startup baseline.
    fn empty(&self, probe: Probe, candidate: &Candidate) -> Result<Finished, OpsError> {
        let finished = self.run(probe, &candidate.binary, &[])?;
        agree(
            &format!("{} empty baseline", candidate.label()),
            &[],
            &finished,
        )?;
        Ok(finished)
    }
}

fn bench_error(text: impl Into<String>) -> OpsError {
    OpsError::Bench(text.into())
}

/// Requires a successful run whose stdout equals `expected`.
fn agree(name: &str, expected: &[u8], finished: &Finished) -> Result<(), OpsError> {
    if !finished.success {
        Err(bench_error(format!(
            "{name} failed ({}); diagnostics withheld",
            finished.status
        )))
    } else if finished.stdout != expected {
        Err(bench_error(format!(
            "{name} disagrees with the primary; answers withheld"
        )))
    } else {
        Ok(())
    }
}

/// The first CPU in a `/proc/self/status` text's `Cpus_allowed_list`.
fn allowed_cpu(status: &str) -> Option<String> {
    status
        .lines()
        .find_map(|line| line.strip_prefix("Cpus_allowed_list:"))?
        .trim()
        .split([',', '-'])
        .next()
        .filter(|cpu| !cpu.is_empty())
        .map(str::to_owned)
}

fn micros(duration: Duration) -> i128 {
    i128::try_from(duration.as_micros()).unwrap_or(i128::MAX)
}

/// Timing runs for a solution whose pilot took `pilot`: fewer for slow solutions.
fn timing_runs(pilot: Duration) -> usize {
    if pilot > SLOW_PILOT {
        SLOW_TIMING_RUNS
    } else {
        TIMING_RUNS
    }
}

/// Requires every solution to agree with the primary and to stay silent on empty input.
fn verify(launcher: &Launcher<'_>, candidates: &mut [Candidate]) -> Result<(), OpsError> {
    for candidate in candidates {
        let pilot = launcher.full(Probe::Bare, candidate)?;
        launcher.empty(Probe::Bare, candidate)?;
        candidate.runs = timing_runs(pilot.wall);
    }
    Ok(())
}

/// Collects full/empty instruction pairs, or explains why work is unavailable.
fn measure_work(launcher: &Launcher<'_>, candidates: &mut [Candidate]) -> Result<String, OpsError> {
    let count = |f: &Finished| bench::instructions(&String::from_utf8_lossy(&f.stderr));
    let supported = candidates.first().is_some_and(|primary| {
        launcher
            .run(Probe::Perf, &primary.binary, &[])
            .is_ok_and(|f| f.success && count(&f).is_some())
    });
    if !supported {
        return Ok(
            "Work unavailable: perf instructions:u is missing, unsupported or denied; no substitute metric."
                .to_owned(),
        );
    }
    for _ in 0..WORK_PAIRS {
        for candidate in candidates.iter_mut() {
            let full = launcher.full(Probe::Perf, candidate)?;
            let empty = launcher.empty(Probe::Perf, candidate)?;
            let (Some(full), Some(empty)) = (count(&full), count(&empty)) else {
                return Err(bench_error(
                    "perf counter became unavailable during measurement",
                ));
            };
            candidate.work.push((full, empty));
        }
    }
    Ok("Work counts user-space instructions of the whole process; paired startup subtraction retains noise.".to_owned())
}

/// Times every solution under GNU time, interleaved round by round.
fn measure_time(launcher: &Launcher<'_>, candidates: &mut [Candidate]) -> Result<(), OpsError> {
    let rounds = candidates.iter().map(|c| c.runs).max().unwrap_or(0);
    for round in 0..rounds {
        for candidate in candidates.iter_mut().filter(|c| round < c.runs) {
            // GNU time appends its report to stderr; stdout stays the answers.
            let finished = launcher.full(Probe::Time, candidate)?;
            let (cpu, rss) = bench::resources(&String::from_utf8_lossy(&finished.stderr))
                .ok_or_else(|| bench_error("invalid GNU time resource observation"))?;
            candidate.timings.push(Timing {
                wall: micros(finished.wall),
                cpu,
                rss,
            });
        }
    }
    Ok(())
}

/// The CPU model from `/proc/cpuinfo` text.
fn cpu_model(cpuinfo: &str) -> Option<String> {
    cpuinfo
        .lines()
        .find(|line| line.starts_with("model name"))
        .and_then(|line| line.split_once(':'))
        .map(|(_, model)| model.trim().to_owned())
}

fn metadata(root: &Path) -> Result<Metadata, OpsError> {
    let model = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|text| cpu_model(&text));
    let cpu = match model {
        Some(model) => model,
        None => process::tool(root, "sysctl", ["-n", "machdep.cpu.brand_string"])?,
    };
    Ok(Metadata {
        date: process::tool(root, "date", ["-u", "+%Y-%m-%dT%H:%M:%SZ"])?,
        commit: process::tool(root, "git", ["rev-parse", "HEAD"])?,
        host: process::tool(root, "uname", ["-n"])?,
        os: process::tool(root, "uname", ["-srm"])?,
        compiler: process::tool(root, "rustc", ["--version"])?,
        standard: "Rust 2024".to_owned(),
        cpu,
    })
}

fn require_canonical(root: &Path) -> Result<(), OpsError> {
    let branch = process::tool(root, "git", ["branch", "--show-current"])?;
    let status = process::tool(
        root,
        "git",
        ["status", "--porcelain", "--untracked-files=no"],
    )?;
    bench::canonical(&branch, &status).map_err(bench_error)
}

/// A finished benchmark of one day.
struct Report {
    year: Year,
    day: Day,
    revision: String,
    measurements: Vec<Measurement>,
    limitations: Vec<String>,
}

impl Report {
    fn lines(&self) -> Vec<String> {
        let render =
            |s: Option<Summary>| s.map_or_else(|| "unavailable".to_owned(), Summary::render);
        let mut lines = vec![
            format!(
                "Benchmark {}/{}: release build; all {} solution(s) agree; answers withheld.",
                self.year,
                self.day,
                self.measurements.len()
            ),
            "Statistics: median [minimum, maximum]; wall/CPU in us; work in user-space instructions."
                .to_owned(),
        ];
        lines.extend(self.measurements.iter().map(|m| {
            format!(
                "{}: runs={}; work={}; instructions={}; wall={}; cpu={}; rss_peak_kib={}",
                m.solution,
                m.runs,
                render(m.work),
                render(m.instructions),
                m.wall.render(),
                m.cpu.render(),
                m.rss_kib
            )
        }));
        lines.extend(self.limitations.iter().map(|l| format!("Limitation: {l}")));
        lines
    }

    /// The CSV after appending this report's rows to `existing` (absent for a new file).
    fn csv(&self, meta: &Metadata, existing: Option<String>) -> Result<String, OpsError> {
        let mut text = match existing {
            Some(text) => {
                if !text.starts_with(&format!("{CSV_HEADER}\n")) || !text.ends_with('\n') {
                    return Err(bench_error(
                        "existing CSV has an invalid header or unterminated row",
                    ));
                }
                text
            }
            None => format!("{CSV_HEADER}\n"),
        };
        for m in &self.measurements {
            text.push_str(&bench::csv_row(meta, self.year.get(), self.day.get(), m));
            text.push('\n');
        }
        Ok(text)
    }

    /// The host snapshot written once per measuring machine.
    fn host_snapshot(&self, meta: &Metadata) -> String {
        let mut host = format!(
            "# {}\n\nOS: {}\nCPU: {}\nCompiler: {}\nLanguage: {}\n\n",
            meta.host, meta.os, meta.cpu, meta.compiler, meta.standard
        );
        for limitation in &self.limitations {
            host.push_str(limitation);
            host.push('\n');
        }
        host
    }

    /// Appends canonical rows to `bench/YEAR.csv` and creates the host snapshot once.
    fn append(&self, root: &Path) -> Result<(), OpsError> {
        require_canonical(root)?;
        let meta = metadata(root)?;
        if meta.commit != self.revision {
            return Err(bench_error("revision changed during benchmark"));
        }
        let csv = root.join(format!("bench/{}.csv", self.year));
        let existing = if csv.is_file() {
            Some(repo::read_text(&csv)?)
        } else {
            None
        };
        let text = self.csv(&meta, existing)?;
        let snapshot = root.join(format!("bench/hosts/{}.md", meta.host));
        if !snapshot.is_file() {
            repo::atomic_write(&snapshot, self.host_snapshot(&meta).as_bytes())?;
        }
        repo::atomic_write(&csv, text.as_bytes())?;
        output::line(&format!(
            "Recorded {} row(s) in bench/{}.csv.",
            self.measurements.len(),
            self.year
        ))
    }
}

/// Measures every solution of a day; with [`Record::Append`], appends canonical rows.
pub(crate) fn run(
    workspace: &Workspace,
    year: Year,
    day: Day,
    record: Record,
) -> Result<(), OpsError> {
    let root = &workspace.root;
    if record == Record::Append {
        require_canonical(root)?;
    }
    let revision = process::tool(root, "git", ["rev-parse", "HEAD"])?;
    let mut candidates: Vec<Candidate> = repo::variants(&workspace.day_dir(year, day))?
        .into_iter()
        .map(|variant| Candidate {
            binary: workspace.release.join(variant.binary(day)),
            variant,
            runs: TIMING_RUNS,
            work: Vec::new(),
            timings: Vec::new(),
        })
        .collect();
    let input_path = workspace.input_path(year, day);
    let input = std::fs::read(&input_path).map_err(|source| OpsError::read(&input_path, source))?;
    if input.is_empty() {
        return Err(bench_error("cached input is empty"));
    }
    let binaries: Vec<String> = candidates.iter().map(|c| c.variant.binary(day)).collect();
    repo::build(workspace, binaries.iter().map(|b| (year, b.as_str())))?;
    let primary = candidates
        .first()
        .ok_or_else(|| bench_error("no solutions"))?;
    let pilot = process::capture(root, &primary.binary, process::NO_ARGS, &input)?;
    if !pilot.success || pilot.stdout.is_empty() {
        return Err(bench_error(
            "primary failed or printed no answers; diagnostics withheld",
        ));
    }
    let launcher = Launcher {
        workspace,
        cpu: std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|status| allowed_cpu(&status)),
        expected: pilot.stdout,
        input,
    };
    // No measured phase starts until every solution agrees with the primary.
    verify(&launcher, &mut candidates)?;
    let work_note = measure_work(&launcher, &mut candidates)?;
    measure_time(&launcher, &mut candidates)?;
    let report = Report {
        year,
        day,
        revision,
        measurements: candidates
            .iter()
            .map(Candidate::measurement)
            .collect::<Result<_, _>>()?,
        limitations: vec![
            work_note,
            launcher.cpu.as_ref().map_or_else(
                || "CPU affinity unavailable; scheduler migration may affect comparisons.".to_owned(),
                |pinned| format!("Pinned to allowed CPU {pinned}."),
            ),
            "Wall microseconds include process launch, redirected IO and GNU time; no baseline subtraction.".to_owned(),
            "CPU microseconds come from GNU time user+system at 10 ms resolution; small samples are coarse.".to_owned(),
            "RSS KiB is GNU time maximum resident set per fresh process, peak across timing runs.".to_owned(),
            format!(
                "Solutions are interleaved per phase; work uses {WORK_PAIRS} full/empty pairs when available. \
                 Timing uses {TIMING_RUNS} runs, or {SLOW_TIMING_RUNS} after a pilot exceeds 1 s."
            ),
        ],
    };
    for line in report.lines() {
        output::line(&line)?;
    }
    if record == Record::Append {
        report.append(root)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use aoc_core::{Day, Year};

    use super::{Report, agree, allowed_cpu, cpu_model, micros, timing_runs};
    use crate::bench::{CSV_HEADER, Measurement, Metadata, Summary};
    use crate::process::Finished;

    fn finished(success: bool, stdout: &[u8]) -> Finished {
        Finished {
            success,
            status: "exit status: 9".to_owned(),
            stdout: stdout.to_vec(),
            stderr: Vec::new(),
            wall: Duration::ZERO,
        }
    }

    fn report() -> (Report, Metadata) {
        let one = Summary {
            median: 2,
            minimum: 1,
            maximum: 3,
        };
        let year = Year::new(2018).unwrap();
        let report = Report {
            year,
            day: Day::new(year, 4).unwrap(),
            revision: "abc".to_owned(),
            measurements: vec![Measurement {
                solution: "primary".to_owned(),
                runs: 10,
                work: None,
                instructions: Some(one),
                wall: one,
                cpu: one,
                rss_kib: 9,
            }],
            limitations: vec!["Pinned to allowed CPU 0.".to_owned()],
        };
        let meta = Metadata {
            date: "d".to_owned(),
            commit: "abc".to_owned(),
            host: "h".to_owned(),
            os: "o".to_owned(),
            cpu: "c".to_owned(),
            compiler: "r".to_owned(),
            standard: "Rust 2024".to_owned(),
        };
        (report, meta)
    }

    #[test]
    fn agreement_requires_success_and_identical_output() {
        assert!(
            agree("p", b"1\n", &finished(true, b"1\n")).is_ok(),
            "agrees"
        );
        let failed = agree("p", b"1\n", &finished(false, b"1\n")).unwrap_err();
        assert_eq!(
            failed.to_string(),
            "p failed (exit status: 9); diagnostics withheld",
            "failure"
        );
        let differs = agree("p", b"1\n", &finished(true, b"2\n")).unwrap_err();
        assert_eq!(
            differs.to_string(),
            "p disagrees with the primary; answers withheld",
            "no answers leak"
        );
    }

    #[test]
    fn slow_pilots_get_fewer_timing_runs() {
        assert_eq!(timing_runs(Duration::from_millis(5)), 10, "fast");
        assert_eq!(
            timing_runs(Duration::from_secs(1)),
            10,
            "the boundary is fast"
        );
        assert_eq!(timing_runs(Duration::from_millis(1001)), 3, "slow");
        assert_eq!(micros(Duration::from_millis(3)), 3000, "microseconds");
    }

    #[test]
    fn host_facts_are_parsed_from_proc_text() {
        assert_eq!(
            allowed_cpu("Name:\tx\nCpus_allowed_list:\t2-7,9\n"),
            Some("2".to_owned()),
            "first allowed CPU"
        );
        assert_eq!(allowed_cpu("Cpus_allowed_list:\t\n"), None, "empty list");
        assert_eq!(allowed_cpu("Name:\tx\n"), None, "missing field");
        assert_eq!(
            cpu_model("processor\t: 0\nmodel name\t: Fast CPU\n"),
            Some("Fast CPU".to_owned()),
            "model"
        );
        assert_eq!(cpu_model("processor\t: 0\n"), None, "no model");
    }

    #[test]
    fn report_lines_withhold_answers_and_list_limitations() {
        let (report, _) = report();
        assert_eq!(
            report.lines(),
            [
                "Benchmark 2018/04: release build; all 1 solution(s) agree; answers withheld.",
                "Statistics: median [minimum, maximum]; wall/CPU in us; work in user-space instructions.",
                "primary: runs=10; work=unavailable; instructions=2 [1, 3]; wall=2 [1, 3]; cpu=2 [1, 3]; rss_peak_kib=9",
                "Limitation: Pinned to allowed CPU 0.",
            ],
            "report"
        );
    }

    #[test]
    fn csv_appends_rows_and_rejects_corrupt_files() {
        let (report, meta) = report();
        let fresh = report.csv(&meta, None).unwrap();
        assert!(
            fresh.starts_with(&format!("{CSV_HEADER}\n")),
            "header first"
        );
        assert_eq!(fresh.lines().count(), 2, "header and one row");
        let appended = report.csv(&meta, Some(fresh.clone())).unwrap();
        assert_eq!(appended.lines().count(), 3, "rows accumulate");
        assert!(appended.starts_with(&fresh), "existing rows are kept");
        assert!(
            report.csv(&meta, Some("other\n".to_owned())).is_err(),
            "foreign header"
        );
        assert!(
            report
                .csv(&meta, Some(fresh.trim_end().to_owned()))
                .is_err(),
            "unterminated row"
        );
        assert_eq!(
            report.host_snapshot(&meta),
            "# h\n\nOS: o\nCPU: c\nCompiler: r\nLanguage: Rust 2024\n\nPinned to allowed CPU 0.\n",
            "snapshot"
        );
    }
}
