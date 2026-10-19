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
    /// Completion times exceed the supported range.
    #[error("completion time overflow")]
    Overflow,
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

fn duration(graph: &Graph, workers: usize, base_seconds: u64) -> Result<u64, SolveError> {
    let mut complete = BTreeSet::new();
    let mut active = BTreeMap::new();
    let mut seconds = 0_u64;
    while complete.len() < graph.len() {
        for (&step, prerequisites) in graph {
            if active.len() == workers {
                break;
            }
            if !complete.contains(&step)
                && !active.contains_key(&step)
                && prerequisites.is_subset(&complete)
            {
                let offset = step.checked_sub(b'A').ok_or(SolveError::Overflow)?;
                let finish = seconds
                    .checked_add(base_seconds)
                    .and_then(|time| time.checked_add(u64::from(offset)))
                    .and_then(|time| time.checked_add(1))
                    .ok_or(SolveError::Overflow)?;
                active.insert(step, finish);
            }
        }
        seconds = active.values().copied().min().ok_or(SolveError::Cycle)?;
        // All simultaneous completions release prerequisites before the next assignment.
        active.retain(|&step, finish| {
            if *finish == seconds {
                complete.insert(step);
                false
            } else {
                true
            }
        });
    }
    Ok(seconds)
}

/// Orders assembly steps and schedules them with five workers and a 60-second base.
///
/// # Errors
/// Returns [`SolveError`] for invalid instructions or cyclic dependencies.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let puzzle = parse(input)?;
    let graph = graph(&puzzle);
    Ok(Answers::new(order(&graph)?, duration(&graph, 5, 60)?))
}

#[cfg(test)]
mod tests {
    use super::{SolveError, duration, graph, order, solve};
    use crate::day07::parse;

    #[test]
    fn newly_available_steps_compete_alphabetically() {
        let puzzle = parse("Step B must be finished before step A can begin.\nStep C must be finished before step D can begin.\n").unwrap();
        assert_eq!(
            order(&graph(&puzzle)).unwrap(),
            "BACD",
            "newly ready A precedes C"
        );
        assert_eq!(
            duration(&graph(&puzzle), 2, 0).unwrap(),
            7,
            "parallel chains finish independently"
        );
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
