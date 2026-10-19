set positional-arguments
set shell := ["bash", "-cu"]

ops := "cargo run -q --locked -p aoc-ops --"

default:
    @just --list

# Download inputs/YEAR/dayDD.txt (cached unless --force)
fetch year day *flags:
    {{ops}} fetch "$@"

# Refresh puzzles/YEAR/DD.txt (articles only, past answers stripped)
puzzle year day:
    {{ops}} puzzle "$@"

# Submit an answer; exit 0 right, 1 wrong, 2 wait/done, 3 unknown, 4 failure
submit year day part answer:
    {{ops}} submit "$@"

# Scaffold a primary solution (and the year crate on first use) or a named exploration
new year day *exploration:
    {{ops}} new "$@"

# Run a release build of a solution on the cached input
run year day *exploration:
    {{ops}} run "$@"

# Check every solution against its fixtures (optionally one year or day)
fixtures *scope:
    {{ops}} test "$@"

# Measure every solution of a day against the primary
bench year day:
    {{ops}} bench "$@"

# Measure and append canonical rows to bench/YEAR.csv (clean main only)
record year day:
    {{ops}} record "$@"

# Check solution header tags
doc:
    {{ops}} doc

# Format Rust sources and flake.nix
fmt:
    cargo fmt --all
    nix fmt flake.nix

# Fast loop: format drift, Clippy, unit tests, fixtures and header tags
check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
    cargo nextest run --workspace --all-features --locked
    {{ops}} test
    {{ops}} doc

# The full gate: the Rust skill's checks minus mutation testing (removed by decision),
# with governance run by BASE's own trusted checker. Miri is absent: no escape hatches exist.
gate base="main":
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
    cargo nextest run --workspace --all-features --locked
    uv run --script .gate/governance-check.py --doctests
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace --locked
    cargo deny --locked check
    cargo machete
    mkdir -p .cache && git show "$1:.gate/governance-check.py" > .cache/trusted-governance-check.py
    uv run --script .cache/trusted-governance-check.py "$1" --inventory
