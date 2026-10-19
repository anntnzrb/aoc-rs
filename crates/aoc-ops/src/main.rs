//! `aoc-ops`: workflow operations for this Advent of Code workspace.

#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    aoc_ops::run(std::env::args().skip(1))
}
