//! Operations run end to end in scratch workspaces with fake tools and a loopback site.

use std::process::ExitCode;

use aoc_core::{Day, Year};

use super::{execute, report};
use crate::cli::{self, Command};
use crate::error::OpsError;
use crate::test_support::{Scratch, ok, serve};

const OK: ExitCode = ExitCode::SUCCESS;
const FAIL: ExitCode = ExitCode::FAILURE;

fn code(value: u8) -> ExitCode {
    ExitCode::from(value)
}

fn command(text: &str) -> Command {
    let words: Vec<String> = text.split_whitespace().map(str::to_owned).collect();
    cli::parse(&words).unwrap()
}

/// Runs a command and returns its exit code, as the binary would.
fn run(scratch: &Scratch, text: &str) -> ExitCode {
    report(try_run(scratch, text))
}

fn try_run(scratch: &Scratch, text: &str) -> Result<ExitCode, OpsError> {
    execute(&scratch.workspace, command(text))
}

fn date(year: u16, day: u8) -> (Year, Day) {
    let year = Year::new(year).unwrap();
    (year, Day::new(year, day).unwrap())
}

/// A scratch workspace pointed at a loopback site that serves `reply` once.
fn online(reply: impl Into<String>) -> (Scratch, std::thread::JoinHandle<String>) {
    let mut scratch = Scratch::new();
    let (base, server) = serve(reply);
    scratch.workspace.base_url = base;
    scratch.write(".env", "AOC_SESSION=c00kie\n");
    (scratch, server)
}

#[test]
fn report_maps_outcomes_to_exit_codes() {
    assert_eq!(report(Ok(code(2))), code(2), "verdict kept");
    assert_eq!(report(Err(OpsError::Root)), code(4), "failures exit 4");
}

#[test]
fn fetch_downloads_once_then_reuses_the_cache() {
    let (scratch, server) = online(ok("1\n2\n"));
    assert_eq!(run(&scratch, "fetch 2018 3"), OK, "fetched");
    assert!(
        server.join().unwrap().starts_with("GET /2018/day/3/input "),
        "input URL"
    );
    assert_eq!(scratch.read("inputs/2018/day03.txt"), "1\n2\n", "cached");
    // No server is listening any more: a second fetch must not touch the network.
    assert_eq!(run(&scratch, "fetch 2018 3"), OK, "cache hit");
    assert_eq!(
        run(&scratch, "fetch 2018 3 --force"),
        code(4),
        "--force goes online"
    );
    assert_eq!(
        scratch.read("inputs/2018/day03.txt"),
        "1\n2\n",
        "failures keep the cache"
    );
}

#[test]
fn network_operations_require_a_session() {
    let scratch = Scratch::new();
    assert!(
        matches!(
            try_run(&scratch, "fetch 2018 1"),
            Err(OpsError::SessionMissing)
        ),
        "no .env"
    );
    scratch.write(".env", "AOC_SESSION=\"bad value\"\n");
    assert!(
        matches!(
            try_run(&scratch, "puzzle 2018 1"),
            Err(OpsError::SessionInvalid)
        ),
        "invalid cookie"
    );
    std::fs::write(scratch.path(".env"), [0xff]).unwrap();
    assert!(
        matches!(
            try_run(&scratch, "submit 2018 1 1 7"),
            Err(OpsError::SessionInvalid)
        ),
        "non-UTF-8 .env"
    );
    std::fs::remove_file(scratch.path(".env")).unwrap();
    std::fs::create_dir(scratch.path(".env")).unwrap();
    assert!(
        matches!(
            try_run(&scratch, "fetch 2018 1"),
            Err(OpsError::Read { .. })
        ),
        "unreadable .env is not reported as missing"
    );
}

#[test]
fn puzzle_writes_articles_without_past_answers() {
    let page = "<main><article class=\"day-desc\"><p>Part one.</p></article>\
                <p>Your puzzle answer was <code>42</code>.</p></main>";
    let (scratch, server) = online(ok(page));
    assert_eq!(run(&scratch, "puzzle 2018 3"), OK, "refreshed");
    assert!(
        server.join().unwrap().starts_with("GET /2018/day/3 "),
        "puzzle URL"
    );
    assert_eq!(
        scratch.read("puzzles/2018/03.txt"),
        "Part one.\n",
        "statement"
    );
}

#[test]
fn submit_posts_the_answer_and_exits_with_the_verdict() {
    let page = "<main><article><p>That's not the right answer.</p></article></main>";
    let (scratch, server) = online(ok(page));
    assert_eq!(run(&scratch, "submit 2018 3 2 ab-1"), code(1), "wrong");
    let request = server.join().unwrap();
    assert!(
        request.starts_with("POST /2018/day/3/answer "),
        "answer URL"
    );
    assert!(request.ends_with("level=2&answer=ab-1"), "form: {request}");
    let (silent, silent_server) = online(ok("<main>nothing</main>"));
    assert_eq!(run(&silent, "submit 2018 3 1 7"), code(3), "no article");
    silent_server.join().unwrap();
}

#[test]
fn new_creates_a_year_crate_and_rolls_back_failed_scaffolds() {
    let scratch = Scratch::new();
    scratch.write("Cargo.lock", "lock\n");
    assert_eq!(run(&scratch, "new 2018 1"), OK, "scaffolded");
    assert!(
        scratch.path("crates/aoc18/src/day01/primary.rs").is_file(),
        "primary"
    );
    assert!(
        scratch.path("fixtures/2018/01").is_dir(),
        "fixture directory"
    );
    assert_eq!(
        scratch.cargo_log(),
        "update --workspace --offline\nfmt -p aoc18\n",
        "lockfile and format"
    );
    assert!(
        matches!(try_run(&scratch, "new 2018 1"), Err(OpsError::Exists(_))),
        "never overwrites"
    );

    // A failing formatter must restore every touched file and remove new directories.
    let lib = scratch.read("crates/aoc18/src/lib.rs");
    scratch.script(
        ".tools/cargo",
        "case \"$1\" in fmt) echo bad > Cargo.lock; exit 1;; esac",
    );
    assert!(
        matches!(try_run(&scratch, "new 2018 2"), Err(OpsError::Child { .. })),
        "formatter failure"
    );
    assert_eq!(
        scratch.read("crates/aoc18/src/lib.rs"),
        lib,
        "lib.rs restored"
    );
    assert_eq!(scratch.read("Cargo.lock"), "lock\n", "lockfile restored");
    assert!(
        !scratch.path("crates/aoc18/src/day02").exists(),
        "day directory removed"
    );
    assert!(
        !scratch.path("crates/aoc18/src/day02.rs").exists(),
        "module removed"
    );
    assert!(
        scratch.path("crates/aoc18/src/bin").is_dir(),
        "shared directory kept"
    );
    assert!(
        !scratch.path("fixtures/2018/02").exists(),
        "fixture removed"
    );
}

#[test]
fn rollback_reports_files_it_cannot_restore() {
    let scratch = Scratch::new();
    scratch.write("Cargo.lock", "lock\n");
    // The formatter turns the lockfile into a directory, so restoring it fails.
    scratch.script(
        ".tools/cargo",
        "case \"$1\" in fmt) rm Cargo.lock; mkdir -p Cargo.lock/x; exit 1;; esac",
    );
    let error = try_run(&scratch, "new 2018 1").unwrap_err();
    assert!(
        matches!(error, OpsError::Rollback { ref paths, .. } if paths.ends_with("Cargo.lock")),
        "{error:?}"
    );
}

#[test]
fn run_builds_then_feeds_the_cached_input() {
    let scratch = Scratch::new();
    scratch.solution("18", "01", "primary", "cat > \"$(dirname \"$0\")/seen\"");
    assert_eq!(run(&scratch, "run 2018 1"), code(4), "missing input");
    scratch.write("inputs/2018/day01.txt", "puzzle input\n");
    assert_eq!(run(&scratch, "run 2018 1"), OK, "ran");
    assert_eq!(
        scratch.read("release/seen"),
        "puzzle input\n",
        "stdin is the input"
    );
    assert_eq!(
        scratch.cargo_log(),
        "build --release --locked -p aoc18 --bin day01\n",
        "release build of one binary"
    );
    scratch.solution("18", "01", "fold", "exit 3");
    assert_eq!(run(&scratch, "run 2018 1 fold"), code(4), "failure");
}

#[test]
fn fixtures_check_every_solution_of_started_days() {
    let scratch = Scratch::new();
    assert_eq!(run(&scratch, "test"), OK, "nothing to test");
    scratch.solution("18", "01", "primary", "echo 1; echo 0");
    scratch.solution("18", "01", "fold", "echo 1; echo 9");
    scratch.solution("25", "02", "primary", "exit 1");
    scratch.write("crates/aoc18/src/day02/fold.rs", ""); // no primary: not started
    scratch.write("fixtures/2018/01/p1-a.in", "x\n");
    scratch.write("fixtures/2018/01/p1-a.out", "1\n");
    scratch.write("fixtures/2018/01/p2-a.in", "x\n");
    scratch.write("fixtures/2018/01/p2-a.out", "1\n0\n");
    scratch.write("fixtures/2018/01/p2-b.in", "x\n");
    scratch.write("fixtures/2025/02/p1-a.in", "x\n");
    scratch.write("fixtures/2025/02/p1-a.out", "1\n");
    assert_eq!(
        run(&scratch, "test 2018 1"),
        FAIL,
        "fold fails p2 and p2-b has no expectation"
    );
    assert_eq!(
        scratch.cargo_log(),
        "build --release --locked -p aoc18 --bin day01 -p aoc18 --bin day01-fold\n",
        "primary first, then explorations"
    );
    std::fs::remove_file(scratch.path("fixtures/2018/01/p2-b.in")).unwrap();
    scratch.solution("18", "01", "fold", "echo 1; echo 0");
    assert_eq!(run(&scratch, "test 2018"), OK, "2018 passes");
    assert_eq!(run(&scratch, "test 2025 2"), FAIL, "crash fails");
    assert_eq!(run(&scratch, "test"), FAIL, "all years");
}

#[test]
fn doc_checks_headers_of_started_days_only() {
    let scratch = Scratch::new();
    let good = "// approach: a\n// cost: b\n// trade: c\n";
    scratch.write("crates/aoc18/src/day01/primary.rs", good);
    scratch.write("crates/aoc18/src/day01.rs", "");
    scratch.write("crates/aoc18/src/day02/notes.rs", ""); // no primary: ignored
    assert_eq!(run(&scratch, "doc"), OK, "complete headers");
    scratch.write("crates/aoc18/src/day01/fold.rs", "// approach:\n");
    assert_eq!(run(&scratch, "doc"), FAIL, "missing tags");
}

#[test]
fn workspace_paths_follow_the_layout() {
    let scratch = Scratch::new();
    let (year, day) = date(2025, 7);
    let w = &scratch.workspace;
    assert_eq!(
        w.input_path(year, day),
        scratch.path("inputs/2025/day07.txt"),
        "input"
    );
    assert_eq!(
        w.puzzle_path(year, day),
        scratch.path("puzzles/2025/07.txt"),
        "puzzle"
    );
    assert_eq!(
        w.fixture_dir(year, day),
        scratch.path("fixtures/2025/07"),
        "fixtures"
    );
    assert_eq!(
        w.day_dir(year, day),
        scratch.path("crates/aoc25/src/day07"),
        "day"
    );
}

/// A scratch git repository on `main` with a committed tree.
fn committed(scratch: &Scratch) {
    let git = |args: &[&str]| {
        let status = std::process::Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .current_dir(scratch.root())
            .output()
            .unwrap();
        assert!(status.status.success(), "git {args:?}");
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["add", "-A"]);
    git(&["commit", "-qm", "base"]);
}

#[test]
fn bench_requires_agreement_before_measuring() {
    let scratch = Scratch::new();
    scratch.solution(
        "18",
        "01",
        "primary",
        "input=$(cat); [ -z \"$input\" ] || echo 42",
    );
    scratch.solution(
        "18",
        "01",
        "fold",
        "input=$(cat); [ -z \"$input\" ] || echo 41",
    );
    committed(&scratch);
    assert_eq!(run(&scratch, "bench 2018 1"), code(4), "no cached input");
    scratch.write("inputs/2018/day01.txt", "");
    assert_eq!(run(&scratch, "bench 2018 1"), code(4), "empty input");
    scratch.write("inputs/2018/day01.txt", "data\n");
    let error = try_run(&scratch, "bench 2018 1").unwrap_err();
    assert_eq!(
        error.to_string(),
        "fold disagrees with the primary; answers withheld",
        "disagreement stops the benchmark"
    );
    scratch.solution("18", "01", "fold", "echo noisy");
    let noisy = try_run(&scratch, "bench 2018 1").unwrap_err();
    assert_eq!(
        noisy.to_string(),
        "fold disagrees with the primary; answers withheld",
        "output on empty input is not a baseline"
    );
    scratch.solution("18", "01", "primary", "cat > /dev/null");
    assert_eq!(
        run(&scratch, "bench 2018 1"),
        code(4),
        "primary prints nothing"
    );
}

#[test]
fn record_appends_canonical_rows_on_clean_main_only() {
    let scratch = Scratch::new();
    let echo = "input=$(cat); [ -z \"$input\" ] || echo 42";
    scratch.solution("18", "01", "primary", echo);
    scratch.solution("18", "01", "fold", echo);
    scratch.write("inputs/2018/day01.txt", "data\n");
    scratch.write(".gitignore", "inputs/\nrelease/\n.tools/\n");
    committed(&scratch);
    assert_eq!(run(&scratch, "bench 2018 1"), OK, "bench measures");
    assert!(!scratch.path("bench").exists(), "bench never records");
    assert_eq!(run(&scratch, "record 2018 1"), OK, "record measures");
    let csv = scratch.read("bench/2018.csv");
    let rows: Vec<&str> = csv.lines().collect();
    assert_eq!(rows.len(), 3, "header and one row per solution: {csv}");
    // The fake perf counts 10 instructions per input byte (newline stripped) above 1000.
    assert!(
        rows[1].contains(",2018,1,\"primary\",10,40,40,40,1040,"),
        "{}",
        rows[1]
    );
    assert!(rows[2].contains(",\"fold\",10,"), "{}", rows[2]);
    assert!(
        rows[1].ends_with(",30000,2048"),
        "CPU and RSS from GNU time: {}",
        rows[1]
    );
    assert!(
        scratch
            .read(&format!("bench/hosts/{}.md", host()))
            .contains("Language: Rust 2024"),
        "host snapshot"
    );
    assert_eq!(
        run(&scratch, "record 2018 1"),
        OK,
        "a second record appends"
    );
    assert_eq!(
        scratch.read("bench/2018.csv").lines().count(),
        5,
        "the header is written once"
    );
    scratch.write(".gitignore", "changed\n");
    assert_eq!(
        run(&scratch, "record 2018 1"),
        code(4),
        "a dirty tracked tree refuses"
    );
}

fn host() -> String {
    let output = std::process::Command::new("uname")
        .arg("-n")
        .output()
        .unwrap();
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}
