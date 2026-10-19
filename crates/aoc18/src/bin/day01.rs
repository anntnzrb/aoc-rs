//! Advent of Code 2018, day 1: reads the input on stdin and prints the primary solution's answers.

#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    aoc_core::run(aoc18::day01::primary::solve)
}
