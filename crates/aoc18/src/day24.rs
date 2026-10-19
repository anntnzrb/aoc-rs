//! Army groups, validated combat statistics, and the day 24 input grammar.

use std::num::{NonZeroU64, ParseIntError};

pub mod primary;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Army {
    Immune,
    Infection,
}

#[derive(Debug)]
struct Group<'a> {
    army: Army,
    units: NonZeroU64,
    hit_points: NonZeroU64,
    damage: NonZeroU64,
    initiative: NonZeroU64,
    attack: &'a str,
    weak: Vec<&'a str>,
    immune: Vec<&'a str>,
}

/// Validated starting armies, borrowing attack names from the input.
#[derive(Debug)]
pub struct Puzzle<'a> {
    groups: Vec<Group<'a>>,
}

/// An invalid army description.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// No groups were supplied.
    #[error("no army groups")]
    Empty,
    /// A line does not follow the group grammar.
    #[error("invalid army or group syntax")]
    Syntax,
    /// A statistic is not a positive integer.
    #[error("invalid positive combat statistic")]
    Number(#[from] ParseIntError),
    /// A combat ordering tie is undefined by the statement.
    #[error("group initiatives must be distinct")]
    Initiative,
}

fn split<'a>(text: &'a str, separator: &str) -> Result<(&'a str, &'a str), ParseError> {
    text.split_once(separator).ok_or(ParseError::Syntax)
}

fn attack_name(text: &str) -> Result<&str, ParseError> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_lowercase()) {
        return Err(ParseError::Syntax);
    }
    Ok(text)
}

fn group(line: &str, army: Army) -> Result<Group<'_>, ParseError> {
    let (units, rest) = split(line, " units each with ")?;
    let (hit_points, rest) = split(rest, " hit points ")?;
    let (traits, attack) = split(rest, "with an attack that does ")?;
    let (damage, attack) = split(attack, " ")?;
    let (attack, initiative) = split(attack, " damage at initiative ")?;
    let mut weak = Vec::new();
    let mut immune = Vec::new();
    if !traits.is_empty() {
        let traits = traits
            .strip_prefix('(')
            .and_then(|s| s.strip_suffix(") "))
            .ok_or(ParseError::Syntax)?;
        for clause in traits.split("; ") {
            let (kind, names) = split(clause, " to ")?;
            let destination = match kind {
                "weak" => &mut weak,
                "immune" => &mut immune,
                _ => return Err(ParseError::Syntax),
            };
            if !destination.is_empty() {
                return Err(ParseError::Syntax);
            }
            for name in names.split(", ") {
                let name = attack_name(name)?;
                if destination.contains(&name) {
                    return Err(ParseError::Syntax);
                }
                destination.push(name);
            }
        }
    }
    if weak.iter().any(|name| immune.contains(name)) {
        return Err(ParseError::Syntax);
    }
    Ok(Group {
        army,
        units: units.parse()?,
        hit_points: hit_points.parse()?,
        damage: damage.parse()?,
        initiative: initiative.parse()?,
        attack: attack_name(attack)?,
        weak,
        immune,
    })
}

/// Parses the two named armies and their group descriptions.
///
/// # Errors
/// Returns an error for invalid syntax, nonpositive statistics, or duplicate initiatives.
pub fn parse(input: &str) -> Result<Puzzle<'_>, ParseError> {
    let mut groups: Vec<Group<'_>> = Vec::new();
    let mut army = None;
    for line in input.lines().map(str::trim).filter(|line| !line.is_empty()) {
        match line {
            "Immune System:" if army.is_none() => army = Some(Army::Immune),
            "Infection:" if army == Some(Army::Immune) => army = Some(Army::Infection),
            _ => {
                let next = group(line, army.ok_or(ParseError::Syntax)?)?;
                if groups.iter().any(|old| old.initiative == next.initiative) {
                    return Err(ParseError::Initiative);
                }
                groups.push(next);
            }
        }
    }
    if groups.is_empty() {
        return Err(ParseError::Empty);
    }
    if army != Some(Army::Infection) || !groups.iter().any(|g| g.army == Army::Immune) {
        return Err(ParseError::Syntax);
    }
    Ok(Puzzle { groups })
}

#[cfg(test)]
mod tests {
    use super::{ParseError, parse};

    const INPUT: &str = "Immune System:\n2 units each with 10 hit points (weak to cold; immune to fire) with an attack that does 3 slashing damage at initiative 2\nInfection:\n3 units each with 5 hit points with an attack that does 4 cold damage at initiative 1\n";

    #[test]
    fn empty_input_is_rejected() {
        assert!(matches!(parse(""), Err(ParseError::Empty)), "no groups");
    }

    #[test]
    fn lines_keep_their_order() {
        let puzzle = parse(INPUT).unwrap();
        assert_eq!(puzzle.groups[0].units.get(), 2, "first group");
        assert_eq!(puzzle.groups[1].units.get(), 3, "second group");
    }

    #[test]
    fn lines_borrow_the_input_and_strip_crlf() {
        let input = INPUT.replace('\n', "\r\n");
        let puzzle = parse(&input).unwrap();
        assert_eq!(puzzle.groups[0].weak, ["cold"], "weakness parsed");
        let attack = puzzle.groups[0].attack;
        assert!(attack.as_ptr() >= input.as_ptr(), "borrowed input storage");
        assert_eq!(attack, "slashing", "attack name");
    }

    #[test]
    fn rejects_invalid_statistics_and_traits() {
        for input in [
            INPUT.replace("2 units", "0 units"),
            INPUT.replace("cold;", "cold, cold;"),
            INPUT.replace("immune to fire", "immune to cold"),
            INPUT.replace("initiative 1", "initiative 2"),
            INPUT.replace("cold damage", "cold extra damage"),
        ] {
            assert!(parse(&input).is_err(), "reject invalid input: {input}");
        }
    }
}
