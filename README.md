# Advent of Code in Rust

Every Advent of Code year from 2015, one crate per year. Run commands from the
repository root in the locked Nix development shell:

```sh
nix develop
just            # list commands
just check      # format, Clippy, unit tests, fixtures, header tags
```

Solved: 2018 days 1 through 4, in crate `aoc18`. The first
`just new YEAR DAY` for another year creates its crate.

## Start a day

```sh
cp .env.example .env            # set AOC_SESSION to your adventofcode.com cookie
just fetch 2018 1               # inputs/2018/day01.txt (cached)
just puzzle 2018 1              # puzzles/2018/01.txt (articles only, answers stripped)
just new 2018 1                 # scaffold the primary solution and fixtures/2018/01/
just run 2018 1                 # release build on the cached input
just submit 2018 1 1 ANSWER     # exit 0 right, 1 wrong, 2 wait/done, 3 unknown
```

Solutions are filters: stdin in, part 1 then part 2 on stdout, one answer per
line, diagnostics on stderr. An unfinished part prints `0`; a puzzle without a
computational part 2 prints only part 1. Empty stdin prints nothing and exits 0;
benchmarks use that run as the startup baseline.

## Layout

```text
Cargo.toml                     Workspace: members crates/*, shared dependencies, lints, profiles
crates/aoc-core/               Shared library: Year/Day calendar, Answers, stdin runner
crates/aoc-ops/                Workflow binary behind the justfile
crates/aocYY/                  One crate per year, created by `just new YEAR DAY`
  src/lib.rs                   `pub mod dayDD;` per started day
  src/dayDD.rs                 Domain model, parser and parser tests
  src/dayDD/primary.rs         The primary solution
  src/dayDD/<exploration>.rs   Named explorations, on request
  src/bin/dayDD[-<name>].rs    Thin binaries: aoc_core::run(...::solve)
templates/                     Files rendered by `new`
bench/YEAR.csv, bench/hosts/   Canonical benchmark records and host snapshots
docs/journal/YEAR.md           Per-year learning log, created by hand
justfile, flake.nix            Commands and the locked toolchain
.gate/, .github/, *.toml       Gate policy (human-owned)
```

Crate names use the last two digits of the year (`aoc18`, `aoc25`); paths and
documentation use the full year. Years from 2025 have 12 days; 2015 to 2024
have 25. Years after 2099 are rejected so crate names stay unique.

## Workflow operations

| Command | Behaviour |
| --- | --- |
| `just fetch YEAR DAY [--force]` | Download `inputs/YEAR/dayDD.txt`; reuse the cache unless `--force`. Only HTTP 200 is cached. |
| `just puzzle YEAR DAY` | Always refresh `puzzles/YEAR/DD.txt`: `day-desc` articles as plain text, "Your puzzle answer was" paragraphs removed. |
| `just submit YEAR DAY PART ANSWER` | Validate `PART` and `ANSWER` (`[0-9A-Za-z,-]+`) before the network, post, print the article. Exit 0 right, 1 wrong, 2 cooldown or already complete, 3 unknown response, 4 local, HTTP or transport failure (429 reports its retry delay). |
| `just new YEAR DAY [EXPLORATION]` | Scaffold a primary (creating the year crate and its lockfile entry on first use) or a lowercase snake_case exploration beside an existing primary. Creates `fixtures/YEAR/DD/`, formats, and rolls back on failure. Never overwrites. |
| `just run YEAR DAY [EXPLORATION]` | Release build of one solution on the cached input. |
| `just fixtures [YEAR [DAY]]` | Run every solution of every started day against `fixtures/YEAR/DD/*.in`, comparing as many lines as the matching `.out` holds. An empty or unterminated `.out` fails. |
| `just bench YEAR DAY` | Build, require every solution to match the primary on the real input and print nothing on empty input, then measure work (5 `perf` full/empty pairs), wall, CPU and RSS (10 GNU `time` runs, 3 above 1 s), pinned with `taskset`. Never prints answers. |
| `just record YEAR DAY` | As `bench`, on a clean `main`, then append rows to `bench/YEAR.csv` and create `bench/hosts/HOST.md` once. |
| `just doc` | Require each solution file to open with non-empty `// approach:`, `// cost:` and `// trade:` lines. Scaffolds start as `TODO`. |

The direct form is `cargo run -q --locked -p aoc-ops -- COMMAND ...`. HTTP goes
through `curl` with the cookie on its stdin config, never the command line, and
the bare `aoc-rs` user agent. Only network operations read `.env`.

Fixtures are `p1-*.in` with a one-line `p1-*.out`, and `p2-*.in` with a
two-line `p2-*.out`, written by hand from the statement.

## Checks and gates

`just check` is the fast loop. `just gate [BASE]` is the full gate:
formatting, Clippy, Nextest, doctests, rustdoc, cargo-deny, cargo-machete and
governance with compiled test inventories, run by `BASE`'s own trusted
checker (default `main`). The Rust skill's assets were installed byte for byte;
the only later policy change is the removal of mutation testing. This is a
learning repository where fixtures and real-input answers catch wrong
solutions; mutations cost minutes per gate run, so the CI `mutants` job and
`cargo-mutants` are gone and `just gate` replaces the Rust skill's
`cli.py gate`, which always runs them. No escape-hatch paths are approved, so
no `expect` attributes, unsafe code or Miri step exist.

The Nix shell targets x86_64 Linux and Apple silicon macOS. It supplies the
pinned Rust 1.99.1 toolchain, Nextest, cargo-deny, cargo-machete, uv, zizmor,
curl, just and GNU time, plus `perf` and `taskset` on Linux. On macOS,
benchmarks report work as unavailable and run unpinned.

The `origin` remote is configured. Linux CI runs on publication; macOS remains unverified.

## Private files

These stay local and ignored; never commit them:

```text
.env                           AOC_SESSION=<cookie>
inputs/YEAR/dayDD.txt          Personal inputs
puzzles/YEAR/DD.txt            Puzzle statements
fixtures/YEAR/DD/              Statement-derived examples
.cache/                        Scratch work and regression answers
```

See [AGENTS.md](AGENTS.md) for the solving workflow and repository rules.
