// approach: simulate reading-order turns; two breadth-first searches resolve destination and step ties
// cost: O(R U V log V) time, O(V + U) memory, one input pass; R rounds, U units, V floor cells
// trade: ordered sparse maps make tie-breaking explicit at the cost of tree lookups

//! Exact round-by-round cavern combat.

use std::collections::{BTreeMap, VecDeque};

use aoc_core::Answers;

use super::{Faction, ParseError, Position, Puzzle, Unit, neighbors, parse};

/// Failure to parse or finish combat.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// Invalid cavern.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// Outcome or distance exceeded the numeric representation.
    #[error("combat arithmetic overflow")]
    Overflow,
    /// Neither side can make progress.
    #[error("combat is a stalemate")]
    Stalemate,
}

fn distances(
    start: Position,
    puzzle: &Puzzle,
    units: &BTreeMap<Position, Unit>,
) -> Result<BTreeMap<Position, u32>, SolveError> {
    let mut distance = BTreeMap::from([(start, 0_u32)]);
    let mut queue = VecDeque::from([start]);
    while let Some(position) = queue.pop_front() {
        let next_distance = distance
            .get(&position)
            .ok_or(SolveError::Overflow)?
            .checked_add(1)
            .ok_or(SolveError::Overflow)?;
        for next in neighbors(position) {
            if puzzle.floor.contains(&next)
                && !units.contains_key(&next)
                && !distance.contains_key(&next)
            {
                distance.insert(next, next_distance);
                queue.push_back(next);
            }
        }
    }
    Ok(distance)
}

fn victim(
    position: Position,
    faction: Faction,
    units: &BTreeMap<Position, Unit>,
) -> Option<Position> {
    neighbors(position)
        .filter_map(|next| {
            units
                .get(&next)
                .filter(|unit| unit.faction != faction)
                .map(|unit| (unit.hp, next))
        })
        .min()
        .map(|(_, next)| next)
}

fn step(
    position: Position,
    faction: Faction,
    puzzle: &Puzzle,
    units: &BTreeMap<Position, Unit>,
) -> Result<Position, SolveError> {
    let forward = distances(position, puzzle, units)?;
    let destination = units
        .iter()
        .filter(|(_, unit)| unit.faction != faction)
        .flat_map(|(&enemy, _)| neighbors(enemy))
        .filter(|next| !units.contains_key(next))
        .filter_map(|next| forward.get(&next).map(|&distance| (distance, next)))
        .min();
    let Some((_, destination)) = destination else {
        return Ok(position);
    };
    let backward = distances(destination, puzzle, units)?;
    Ok(neighbors(position)
        .filter_map(|next| backward.get(&next).map(|&distance| (distance, next)))
        .min()
        .map_or(position, |(_, next)| next))
}

fn outcome(rounds: u32, units: &BTreeMap<Position, Unit>) -> Result<u32, SolveError> {
    let hp = units.values().try_fold(0_u32, |sum, unit| {
        sum.checked_add(unit.hp).ok_or(SolveError::Overflow)
    })?;
    rounds.checked_mul(hp).ok_or(SolveError::Overflow)
}

fn combat(puzzle: &Puzzle) -> Result<u32, SolveError> {
    // Each run owns its evolving positions independently of the parsed map.
    let mut units = puzzle.units.clone();
    let mut rounds = 0_u32;
    loop {
        let turns: Vec<_> = units
            .iter()
            .map(|(&position, unit)| (position, unit.id))
            .collect();
        let mut changed = false;
        for (position, id) in turns {
            let Some(unit) = units.get(&position).copied().filter(|unit| unit.id == id) else {
                continue;
            };
            if !units.values().any(|enemy| enemy.faction != unit.faction) {
                return outcome(rounds, &units);
            }
            let destination = if victim(position, unit.faction, &units).is_some() {
                position
            } else {
                step(position, unit.faction, puzzle, &units)?
            };
            if destination != position {
                units.remove(&position);
                units.insert(destination, unit);
                changed = true;
            }
            if let Some(target) = victim(destination, unit.faction, &units) {
                let enemy = units.get_mut(&target).ok_or(SolveError::Overflow)?;
                // Damage bottoms out at death; negative hit points are irrelevant.
                enemy.hp = enemy.hp.saturating_sub(3);
                if enemy.hp == 0 {
                    units.remove(&target);
                }
                changed = true;
            }
        }
        if !changed {
            return Err(SolveError::Stalemate);
        }
        rounds = rounds.checked_add(1).ok_or(SolveError::Overflow)?;
    }
}

/// Solves the combat outcome, with part two not yet implemented.
///
/// # Errors
/// Returns [`SolveError`] for invalid maps, stalemates, or numeric overflow.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    Ok(Answers::new(combat(&parse(input)?)?, 0))
}

#[cfg(test)]
mod tests {
    use super::{SolveError, combat, parse, solve};

    #[test]
    fn adjacent_duel_counts_only_complete_rounds() {
        assert_eq!(
            combat(&parse("EG").unwrap()).unwrap(),
            134,
            "elf wins after 67 rounds with two hp"
        );
        assert_eq!(
            combat(&parse("EGE").unwrap()).unwrap(),
            9933,
            "dead turns do not finish combat prematurely"
        );
    }

    #[test]
    fn separated_factions_are_a_stalemate() {
        assert!(
            matches!(solve("E#G"), Err(SolveError::Stalemate)),
            "unreachable enemies"
        );
        assert_eq!(
            combat(&parse("E").unwrap()).unwrap(),
            0,
            "no enemies ends before the first round"
        );
    }
}
