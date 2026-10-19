//! Shared contracts for Advent of Code solutions: puzzle dates, answers and the stdin runner.

#![forbid(unsafe_code)]

mod answers;
mod calendar;
mod runner;

pub use answers::Answers;
pub use calendar::{CalendarError, Day, Year};
pub use runner::{MAX_INPUT_BYTES, RunError, execute, run};
