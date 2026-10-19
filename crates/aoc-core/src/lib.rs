//! Shared puzzle dates, answers, line-record parsing and stdin execution for Advent of Code.

#![forbid(unsafe_code)]

mod answers;
mod calendar;
mod records;
mod runner;

pub use answers::Answers;
pub use calendar::{CalendarError, Day, Year};
#[doc(inline)]
pub use records::{RecordsError, parse_records};
pub use runner::{MAX_INPUT_BYTES, RunError, execute, run};
