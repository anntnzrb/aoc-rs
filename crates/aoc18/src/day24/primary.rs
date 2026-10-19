// approach: Select targets by damage and power, then apply attacks in initiative order.
// cost: O(B R G² + B R G log G) time and O(G) memory; one input pass.
// trade: Exhaustive ascending boosts prove minimality without assuming monotone battle outcomes.

//! Direct simulation of the two armies' target selection and attacks.

use std::cmp::Reverse;

use aoc_core::Answers;

use super::{Army, Group, ParseError, Puzzle, parse};

/// Parsing or combat failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// Invalid army description.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// Combat arithmetic exceeded its representation.
    #[error("combat statistic overflow")]
    Overflow,
    /// A casualty count could not be represented.
    #[error("casualty count conversion failed")]
    Casualties(#[from] std::num::TryFromIntError),
    /// An internal group slot was not present.
    #[error("invalid combat group slot")]
    Slot,
    /// Neither army can kill a unit.
    #[error("combat ended in a stalemate")]
    Stalemate,
}

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Victory(Army, u64),
    Stalemate,
}

fn damage(group: &Group<'_>, defender: &Group<'_>, power: u128) -> Result<u128, SolveError> {
    if defender.immune.contains(&group.attack) {
        Ok(0)
    } else if defender.weak.contains(&group.attack) {
        power.checked_mul(2).ok_or(SolveError::Overflow)
    } else {
        Ok(power)
    }
}

fn power(group: &Group<'_>, units: u64, boost: u64) -> Result<u128, SolveError> {
    let attack = group
        .damage
        .get()
        .checked_add(match group.army {
            Army::Immune => boost,
            Army::Infection => 0,
        })
        .ok_or(SolveError::Overflow)?;
    u128::from(units)
        .checked_mul(u128::from(attack))
        .ok_or(SolveError::Overflow)
}

fn targets(
    puzzle: &Puzzle<'_>,
    units: &[u64],
    boost: u64,
) -> Result<Vec<(u64, usize, usize)>, SolveError> {
    let powers = puzzle
        .groups
        .iter()
        .zip(units)
        .map(|(g, &n)| power(g, n, boost))
        .collect::<Result<Vec<_>, _>>()?;
    let mut order = puzzle
        .groups
        .iter()
        .zip(&powers)
        .enumerate()
        .map(|(id, (g, &p))| (Reverse((p, g.initiative.get())), id))
        .collect::<Vec<_>>();
    order.sort_unstable();
    let mut claimed = vec![false; units.len()];
    let mut attacks = Vec::with_capacity(units.len());
    for (_, attacker) in order {
        let group = puzzle.groups.get(attacker).ok_or(SolveError::Slot)?;
        let attack_power = *powers.get(attacker).ok_or(SolveError::Slot)?;
        if attack_power == 0 {
            continue;
        }
        let mut best = None;
        for (id, defender) in puzzle.groups.iter().enumerate() {
            if group.army == defender.army
                || *claimed.get(id).ok_or(SolveError::Slot)?
                || *units.get(id).ok_or(SolveError::Slot)? == 0
            {
                continue;
            }
            let dealt = damage(group, defender, attack_power)?;
            let key = (
                dealt,
                *powers.get(id).ok_or(SolveError::Slot)?,
                defender.initiative.get(),
            );
            if dealt > 0 && best.is_none_or(|(old, _)| key > old) {
                best = Some((key, id));
            }
        }
        if let Some((_, defender)) = best {
            *claimed.get_mut(defender).ok_or(SolveError::Slot)? = true;
            attacks.push((group.initiative.get(), attacker, defender));
        }
    }
    attacks.sort_unstable_by_key(|&(initiative, _, _)| Reverse(initiative));
    Ok(attacks)
}

fn fight(puzzle: &Puzzle<'_>, boost: u64) -> Result<Outcome, SolveError> {
    let mut units = puzzle
        .groups
        .iter()
        .map(|g| g.units.get())
        .collect::<Vec<_>>();
    loop {
        let mut immune = 0_u64;
        let mut infection = 0_u64;
        for (g, &n) in puzzle.groups.iter().zip(&units) {
            let total = match g.army {
                Army::Immune => &mut immune,
                Army::Infection => &mut infection,
            };
            *total = total.checked_add(n).ok_or(SolveError::Overflow)?;
        }
        if immune == 0 {
            return Ok(Outcome::Victory(Army::Infection, infection));
        }
        if infection == 0 {
            return Ok(Outcome::Victory(Army::Immune, immune));
        }
        let mut killed = 0_u64;
        for (_, attacker, defender) in targets(puzzle, &units, boost)? {
            let group = puzzle.groups.get(attacker).ok_or(SolveError::Slot)?;
            let target = puzzle.groups.get(defender).ok_or(SolveError::Slot)?;
            let power = power(group, *units.get(attacker).ok_or(SolveError::Slot)?, boost)?;
            let casualties =
                damage(group, target, power)? / std::num::NonZeroU128::from(target.hit_points);
            let remaining = units.get_mut(defender).ok_or(SolveError::Slot)?;
            let casualties = u64::try_from(casualties.min(u128::from(*remaining)))?;
            *remaining = remaining
                .checked_sub(casualties)
                .ok_or(SolveError::Overflow)?;
            killed = killed.checked_add(casualties).ok_or(SolveError::Overflow)?;
        }
        if killed == 0 {
            return Ok(Outcome::Stalemate);
        }
    }
}

/// Solves unboosted combat and searches ascending boosts for the first immune victory.
///
/// # Errors
/// Returns an error for invalid input, overflow, or an unboosted stalemate.
pub fn solve(input: &str) -> Result<Answers, SolveError> {
    let puzzle = parse(input)?;
    let Outcome::Victory(_, units) = fight(&puzzle, 0)? else {
        return Err(SolveError::Stalemate);
    };
    let mut boost = 0_u64;
    loop {
        if let Outcome::Victory(Army::Immune, survivors) = fight(&puzzle, boost)? {
            return Ok(Answers::new(units, survivors));
        }
        boost = boost.checked_add(1).ok_or(SolveError::Overflow)?;
    }
}

#[cfg(test)]
mod tests {
    use super::{Army, Outcome, fight, parse};

    #[test]
    fn casualties_reduce_attack_and_dead_groups_do_not_attack() {
        let puzzle = parse("Immune System:\n3 units each with 10 hit points with an attack that does 10 fire damage at initiative 2\nInfection:\n2 units each with 10 hit points with an attack that does 100 cold damage at initiative 1\n").unwrap();
        assert_eq!(
            fight(&puzzle, 0).unwrap(),
            Outcome::Victory(Army::Immune, 3),
            "fast group kills its enemy before retaliation"
        );
    }

    #[test]
    fn no_kills_is_a_stalemate() {
        let puzzle = parse("Immune System:\n1 units each with 10 hit points with an attack that does 1 fire damage at initiative 2\nInfection:\n1 units each with 10 hit points with an attack that does 1 cold damage at initiative 1\n").unwrap();
        assert_eq!(
            fight(&puzzle, 0).unwrap(),
            Outcome::Stalemate,
            "partial damage does not accumulate"
        );
    }

    #[test]
    fn boost_crosses_stalemates_and_uses_the_smallest_winner() {
        let puzzle = parse("Immune System:\n1 units each with 10 hit points with an attack that does 1 fire damage at initiative 2\nInfection:\n1 units each with 10 hit points with an attack that does 1 cold damage at initiative 1\n").unwrap();
        assert_eq!(
            fight(&puzzle, 8).unwrap(),
            Outcome::Stalemate,
            "boost eight cannot kill a unit"
        );
        assert_eq!(
            fight(&puzzle, 9).unwrap(),
            Outcome::Victory(Army::Immune, 1),
            "boost nine is the first lethal attack"
        );
    }
}
