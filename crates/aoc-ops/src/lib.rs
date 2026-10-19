//! Workflow operations for the Advent of Code workspace.
//!
//! Pure decisions (argument parsing, HTML extraction, verdicts, scaffolding, fixture
//! comparison, statistics) live in their own modules; [`run`] is the effectful edge.

#![forbid(unsafe_code)]

mod app;
mod bench;
mod benchmark;
mod cli;
mod doc;
mod error;
mod fixtures;
mod http;
mod output;
mod process;
mod repo;
mod scaffold;
mod session;
mod statement;
mod submission;
#[cfg(test)]
mod test_support;

pub use app::run;
