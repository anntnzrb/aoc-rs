// approach: Sort guard events, tally half-open sleep intervals, select peaks by total sleep and by frequency.
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

/// Selects the sleepiest guard's peak minute and the globally most frequent guard/minute pair.
///
/// Ties select the lowest guard identifier, then the earliest minute.
///
/// # Errors
///
/// Returns [`ParseError`] for malformed schedules, missing sleep or integer overflow.
pub fn solve(input: &str) -> Result<Answers, ParseError> {
    let puzzle = parse(input)?;
    let guards = histogram(&puzzle)?;
    let (&sleepiest_id, sleepiest) = guards
        .iter()
        .max_by_key(|&(id, guard)| (guard.total, Reverse(id)))
        .ok_or(ParseError::NoSleep)?;
    let (sleepiest_minute, _) = peak(sleepiest)?;
    let part1 = NonZeroU64::from(sleepiest_id)
        .get()
        .checked_mul(sleepiest_minute)
        .ok_or(ParseError::Overflow)?;
    let (id, minute, _) = guards
        .iter()
        .flat_map(|(&id, guard)| {
            guard
                .minutes
                .iter()
                .zip(0_u64..)
                .map(move |(&count, minute)| (id, minute, count))
        })
        .max_by_key(|&(id, minute, count)| (count, Reverse(id), Reverse(minute)))
        .ok_or(ParseError::NoSleep)?;
    let part2 = NonZeroU64::from(id)
        .get()
        .checked_mul(minute)
        .ok_or(ParseError::Overflow)?;
    Ok(Answers::new(part1, part2))
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
            Answers::new(133, 44),
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
                    Answers::new(13 * start, 13 * start),
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
            Answers::new(249_108_103_110_u64, 249_108_103_110_u64),
            "answer multiplication is wider than guard identifiers"
        );
        let tied = format!(
            "{input}\n[1518-01-02 00:00] Guard #3 begins shift\n[1518-01-02 00:58] falls asleep\n[1518-01-02 00:59] wakes up"
        );
        assert_eq!(
            solve(&tied).unwrap(),
            Answers::new(174, 174),
            "lowest identifier wins tied totals"
        );
        assert_eq!(
            solve("[1518-01-01 00:00] Guard #7 begins shift"),
            Err(ParseError::NoSleep),
            "no fabricated sleeping minute"
        );
    }

    #[test]
    fn disjoint_sleeps_do_not_count_awake_gaps_or_wake_minutes() {
        let input = "[1518-01-01 00:00] Guard #5 begins shift\n[1518-01-01 00:10] falls asleep\n[1518-01-01 00:11] wakes up\n[1518-01-01 00:12] falls asleep\n[1518-01-01 00:13] wakes up\n[1518-01-02 00:00] Guard #5 begins shift\n[1518-01-02 00:11] falls asleep\n[1518-01-02 00:13] wakes up";
        assert_eq!(
            solve(input).unwrap(),
            Answers::new(60, 60),
            "minute 12 alone occurs twice"
        );
        let reversed = input.lines().rev().collect::<Vec<_>>().join("\n");
        assert_eq!(
            solve(&reversed).unwrap(),
            Answers::new(60, 60),
            "arbitrary input order preserves both strategies"
        );
    }
}
