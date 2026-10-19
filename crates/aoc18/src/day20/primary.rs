// approach: Track every branch endpoint with an explicit stack, build doors, then breadth-first search.
// cost: Two input passes; O(T log R + R log R) time, O(R + S) memory for T endpoint steps and S saved endpoints.
// trade: Ordered sets deduplicate converging routes; a flat stack avoids nesting-dependent call depth.

//! Route interpretation and shortest door distances on the resulting room graph.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use aoc_core::Answers;

use super::{ParseError, Puzzle, parse};

type Room = (i64, i64);
type Doors = BTreeMap<Room, BTreeSet<Room>>;

struct Branch {
    starts: BTreeSet<Room>,
    ends: BTreeSet<Room>,
}

fn step((x, y): Room, direction: char) -> Result<Room, ParseError> {
    let (dx, dy) = match direction {
        'N' => (0, -1),
        'S' => (0, 1),
        'E' => (1, 0),
        'W' => (-1, 0),
        other => return Err(ParseError::Character(other)),
    };
    Ok((
        x.checked_add(dx).ok_or(ParseError::Overflow)?,
        y.checked_add(dy).ok_or(ParseError::Overflow)?,
    ))
}

fn map(puzzle: &Puzzle<'_>) -> Result<Doors, ParseError> {
    let mut doors = Doors::new();
    let mut positions = BTreeSet::from([(0, 0)]);
    let mut branches: Vec<Branch> = Vec::new();
    for token in puzzle.route().chars() {
        match token {
            '(' => branches.push(Branch {
                // Each alternative must retain the same independent starting frontier.
                starts: positions.clone(),
                ends: BTreeSet::new(),
            }),
            '|' => {
                let branch = branches.last_mut().ok_or(ParseError::Group)?;
                branch.ends.append(&mut positions);
                positions.clone_from(&branch.starts);
            }
            ')' => {
                let mut branch = branches.pop().ok_or(ParseError::Group)?;
                branch.ends.append(&mut positions);
                positions = branch.ends;
            }
            direction => {
                let mut next = BTreeSet::new();
                for position in positions {
                    let neighbor = step(position, direction)?;
                    doors.entry(position).or_default().insert(neighbor);
                    doors.entry(neighbor).or_default().insert(position);
                    next.insert(neighbor);
                }
                positions = next;
            }
        }
    }
    Ok(doors)
}

fn distances(doors: &Doors) -> Result<Vec<usize>, ParseError> {
    let mut seen = BTreeSet::from([(0, 0)]);
    let mut queue = VecDeque::from([((0, 0), 0_usize)]);
    let mut distances = Vec::new();
    while let Some((room, distance)) = queue.pop_front() {
        distances.push(distance);
        if let Some(neighbors) = doors.get(&room) {
            for &neighbor in neighbors {
                if seen.insert(neighbor) {
                    queue.push_back((
                        neighbor,
                        distance.checked_add(1).ok_or(ParseError::Overflow)?,
                    ));
                }
            }
        }
    }
    Ok(distances)
}

/// Solves the furthest-room distance and counts rooms at least 1000 doors away.
///
/// # Errors
/// Returns [`ParseError`] for malformed expressions or unrepresentable maps.
pub fn solve(input: &str) -> Result<Answers, ParseError> {
    let puzzle = parse(input)?;
    let distances = distances(&map(&puzzle)?)?;
    let distant = distances
        .iter()
        .filter(|&&distance| distance >= 1000)
        .count();
    Ok(Answers::new(
        distances.into_iter().max().unwrap_or(0),
        distant,
    ))
}

#[cfg(test)]
mod tests {
    use super::{ParseError, solve, step};

    #[test]
    fn distant_rooms_include_the_threshold() {
        for (length, count) in [(999, 0), (1000, 1), (1001, 2)] {
            let input = format!("^{}$", "N".repeat(length));
            assert_eq!(
                solve(&input).unwrap(),
                aoc_core::Answers::new(length, count),
                "straight corridor of {length} doors"
            );
        }
    }

    #[test]
    fn branches_continue_from_all_endpoints() {
        for (input, expected) in [
            ("^(N|E)NN$", 3),
            ("^(|E)NN$", 3),
            ("^N(E|W(N|S))E$", 4),
            ("^$", 0),
        ] {
            assert_eq!(
                solve(input).unwrap(),
                aoc_core::Answers::new(expected, 0),
                "{input}"
            );
        }
    }

    #[test]
    fn doors_are_bidirectional_and_shortest_paths_ignore_route_length() {
        assert_eq!(
            solve("^ENWS$").unwrap(),
            aoc_core::Answers::new(2, 0),
            "cycle shortcuts"
        );
        assert_eq!(
            solve("^EEWWNN$").unwrap(),
            aoc_core::Answers::new(2, 0),
            "retraced route"
        );
    }

    #[test]
    fn deeply_nested_groups_use_no_recursion() {
        let input = format!("^{}N{}$", "(".repeat(20_000), ")".repeat(20_000));
        assert_eq!(
            solve(&input).unwrap(),
            aoc_core::Answers::new(1, 0),
            "deep nesting"
        );
    }

    #[test]
    fn coordinate_overflow_is_typed() {
        assert_eq!(
            step((i64::MAX, 0), 'E'),
            Err(ParseError::Overflow),
            "east overflow"
        );
        assert_eq!(
            step((0, i64::MIN), 'N'),
            Err(ParseError::Overflow),
            "north overflow"
        );
    }
}
