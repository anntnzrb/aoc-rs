//! Advent of Code 2018, day 1: reads the input on stdin and prints the exploration's answers.

#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    aoc_core::run(aoc18::day01::residue_buckets::solve)
}
