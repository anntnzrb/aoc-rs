// approach: repeatedly choose the alphabetically first step whose prerequisites are complete
// cost: O(V(V+E)) time, O(V+E) memory, one input parsing pass
// trade: ordered maps make tie-breaking explicit without mutable indegree bookkeeping

//! Alphabetically ordered dependency traversal.

use std::collections::{BTreeMap, BTreeSet};

use aoc_core::Answers;

use super::{ParseError, Puzzle, parse};

type Graph = BTreeMap<u8, BTreeSet<u8>>;

/// Parsing or dependency traversal failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// Invalid instruction input.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// Dependencies prevent all remaining steps from starting.
    #[error("step dependencies contain a cycle")]
    Cycle,
}

fn graph(puzzle: &Puzzle) -> Graph {
    let mut graph = Graph::new();
    for dependency in &puzzle.dependencies {
        graph.entry(dependency.before).or_default();
        graph
            .entry(dependency.after)
            .or_default()
            .insert(dependency.before);
    }
    graph
}

fn order(graph: &Graph) -> Result<String, SolveError> {
    let mut complete = BTreeSet::new();
    let mut result = String::with_capacity(graph.len());
    while complete.len() < graph.len() {
        let (&step, _) = graph
            .iter()
            .find(|(step, prerequisites)| {
                !complete.contains(*step) && prerequisites.is_subset(&complete)
            })
            .ok_or(SolveError::Cycle)?;
        complete.insert(step);
        result.push(char::from(step));
    }
    Ok(result)
}

/// Orders assembly steps; unfinished part two renders `0`.
///
/// # Errors
/// Returns [`SolveError`] for invalid instructions or cyclic dependencies.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let puzzle = parse(input)?;
    Ok(Answers::new(order(&graph(&puzzle))?, 0))
}

#[cfg(test)]
mod tests {
    use super::{SolveError, solve};

    #[test]
    fn newly_available_steps_compete_alphabetically() {
        assert_eq!(solve("Step B must be finished before step A can begin.\nStep C must be finished before step D can begin.\n").unwrap(), aoc_core::Answers::new("BACD", 0), "newly ready A precedes C");
        assert!(
            matches!(
                solve(
                    "Step A must be finished before step B can begin.\nStep B must be finished before step A can begin."
                ),
                Err(SolveError::Cycle)
            ),
            "cycle rejected"
        );
    }
}
