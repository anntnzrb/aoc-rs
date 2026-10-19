// approach: Sort guard events, tally each half-open sleep interval, choose the sleepiest guard's peak minute.
// cost: O(n log n + s log g + 60s + 60g) time, O(n + 60g) memory; 2 full input passes plus sorting.
// trade: Fixed midnight histograms make interval endpoints explicit; sorting retains unordered records in memory.

//! Guard sleep totals and minute frequencies derived from validated chronological intervals.

use std::{
    cmp::Reverse,
    collections::BTreeMap,
    num::{NonZeroU32, NonZeroU64},
};

use aoc_core::Answers;

use super::{ParseError, Puzzle, parse};

#[derive(Debug)]
struct GuardSleep {
    total: u64,
    minutes: [u64; 60],
}

impl Default for GuardSleep {
    fn default() -> Self {
        Self {
            total: 0,
            minutes: [0; 60],
        }
    }
}

fn histogram(puzzle: &Puzzle) -> Result<BTreeMap<NonZeroU32, GuardSleep>, ParseError> {
    let mut guards = BTreeMap::<_, GuardSleep>::new();
    for sleep in &puzzle.sleeps {
        let guard = guards.entry(sleep.guard).or_default();
        for count in guard
            .minutes
            .iter_mut()
            .take(usize::from(sleep.minutes.end))
            .skip(usize::from(sleep.minutes.start))
        {
            *count = count.checked_add(1).ok_or(ParseError::Overflow)?;
            guard.total = guard.total.checked_add(1).ok_or(ParseError::Overflow)?;
        }
    }
    Ok(guards)
}

fn peak(guard: &GuardSleep) -> Result<(u64, u64), ParseError> {
    let (minute, &count) = guard
        .minutes
        .iter()
        .zip(0_u64..)
        .map(|(count, minute)| (minute, count))
        .max_by_key(|&(minute, count)| (count, Reverse(minute)))
        .ok_or(ParseError::NoSleep)?;
    Ok((minute, count))
}

/// Selects the guard with the most total sleep and that guard's most frequent minute.
///
/// Ties select the lowest guard identifier, then the earliest minute.
///
/// # Errors
///
/// Returns [`ParseError`] for malformed schedules, missing sleep or integer overflow.
pub fn solve(input: &str) -> Result<Answers, ParseError> {
    let puzzle = parse(input)?;
    let guards = histogram(&puzzle)?;
    let (&id, guard) = guards
        .iter()
        .max_by_key(|&(id, guard)| (guard.total, Reverse(id)))
        .ok_or(ParseError::NoSleep)?;
    let (minute, _) = peak(guard)?;
    let part1 = NonZeroU64::from(id)
        .get()
        .checked_mul(minute)
        .ok_or(ParseError::Overflow)?;
    Ok(Answers::new(part1, 0))
}

#[cfg(test)]
mod tests {
    use super::{ParseError, solve};
    use aoc_core::Answers;

    #[test]
    fn totals_precede_peak_frequency() {
        let input = "[1518-01-01 00:00] Guard #7 begins shift\n[1518-01-01 00:10] falls asleep\n[1518-01-01 00:20] wakes up\n[1518-01-02 00:00] Guard #7 begins shift\n[1518-01-02 00:19] falls asleep\n[1518-01-02 00:21] wakes up\n[1518-01-03 00:00] Guard #11 begins shift\n[1518-01-03 00:04] falls asleep\n[1518-01-03 00:05] wakes up\n[1518-01-04 00:00] Guard #11 begins shift\n[1518-01-04 00:04] falls asleep\n[1518-01-04 00:05] wakes up\n[1518-01-05 00:00] Guard #11 begins shift\n[1518-01-05 00:04] falls asleep\n[1518-01-05 00:05] wakes up";
        assert_eq!(
            solve(input).unwrap(),
            Answers::new(133, 0),
            "total sleep, not highest frequency, selects guard"
        );
    }

    #[test]
    fn every_midnight_interval_round_trips() {
        for start in 0..59 {
            for end in start + 1..60 {
                let input = format!(
                    "[1518-01-01 00:00] Guard #13 begins shift\n[1518-01-02 00:{start:02}] falls asleep\n[1518-01-02 00:{end:02}] wakes up"
                );
                assert_eq!(
                    solve(&input).unwrap(),
                    Answers::new(13 * start, 0),
                    "[{start}, {end}) selects earliest tied minute"
                );
            }
        }
    }

    #[test]
    fn guard_ties_and_large_identifiers_are_deterministic() {
        let input = "[1518-01-01 00:00] Guard #4294967295 begins shift\n[1518-01-01 00:58] falls asleep\n[1518-01-01 00:59] wakes up";
        assert_eq!(
            solve(input).unwrap(),
            Answers::new(249_108_103_110_u64, 0),
            "answer multiplication is wider than guard identifiers"
        );
        let tied = format!(
            "{input}\n[1518-01-02 00:00] Guard #3 begins shift\n[1518-01-02 00:58] falls asleep\n[1518-01-02 00:59] wakes up"
        );
        assert_eq!(
            solve(&tied).unwrap(),
            Answers::new(174, 0),
            "lowest identifier wins tied totals"
        );
        assert_eq!(
            solve("[1518-01-01 00:00] Guard #7 begins shift"),
            Err(ParseError::NoSleep),
            "no fabricated sleeping minute"
        );
    }
}
