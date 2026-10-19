//! Validated chronological guard records reduced to complete, half-open midnight sleep intervals.

use std::{num::NonZeroU32, ops::Range};

pub mod primary;

/// Complete sleep intervals in chronological order, independent of record order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    sleeps: Vec<Sleep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Sleep {
    guard: NonZeroU32,
    minutes: Range<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Timestamp {
    date: (u16, u8, u8),
    hour: u8,
    minute: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Event {
    Begin(NonZeroU32),
    Sleep,
    Wake,
}

#[derive(Debug, Clone, Copy)]
struct Record {
    time: Timestamp,
    event: Event,
}

#[derive(Debug, Clone, Copy)]
enum State {
    OffDuty,
    Awake(NonZeroU32),
    Asleep(NonZeroU32, Timestamp),
}

/// Failure to parse records or analyze a valid sleep schedule.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// No records or an invalid individual record.
    #[error(transparent)]
    Records(#[from] aoc_core::RecordsError<RecordError>),
    /// Two events occupy the same timestamp.
    #[error("duplicate event timestamp")]
    DuplicateTime,
    /// An event cannot follow the preceding guard state.
    #[error("invalid guard sleep transition")]
    Transition,
    /// A sleep interval was not closed on the same date.
    #[error("sleep must end later in the same midnight hour")]
    Interval,
    /// No guard has a recorded sleep interval.
    #[error("no recorded sleep")]
    NoSleep,
    /// A count or answer exceeds the supported integer range.
    #[error("sleep count or answer overflow")]
    Overflow,
}

/// An invalid timestamp or event description.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RecordError {
    /// Record punctuation, numeric width or event text is invalid.
    #[error("invalid timestamp or guard event syntax")]
    Syntax,
    /// A decimal integer exceeds its representation.
    #[error("invalid record integer")]
    Integer(#[from] std::num::ParseIntError),
    /// A numeric field cannot fit its target representation.
    #[error("record integer out of range")]
    Conversion(#[from] std::num::TryFromIntError),
    /// A date, time or guard identifier is out of range.
    #[error("record date, time or guard identifier out of range")]
    Range,
}

/// Parses unordered records and validates complete midnight sleep intervals.
///
/// # Errors
///
/// Returns [`ParseError`] for malformed records, duplicate timestamps, impossible
/// state transitions, or unclosed and cross-date sleep intervals.
pub fn parse(input: &str) -> Result<Puzzle, ParseError> {
    let mut records = aoc_core::parse_records(input, record)?;
    records.sort_unstable_by_key(|entry| entry.time);
    let mut previous = None;
    let mut state = State::OffDuty;
    let mut sleeps = Vec::new();
    for Record { time, event } in records {
        if previous == Some(time) {
            return Err(ParseError::DuplicateTime);
        }
        previous = Some(time);
        state = match (state, event) {
            (State::OffDuty | State::Awake(_), Event::Begin(guard)) => State::Awake(guard),
            (State::Awake(guard), Event::Sleep) => State::Asleep(guard, time),
            (State::Asleep(guard, start), Event::Wake) => {
                if start.date != time.date || start.minute >= time.minute {
                    return Err(ParseError::Interval);
                }
                sleeps.push(Sleep {
                    guard,
                    minutes: start.minute..time.minute,
                });
                State::Awake(guard)
            }
            (State::OffDuty | State::Awake(_) | State::Asleep(_, _), _) => {
                return Err(ParseError::Transition);
            }
        };
    }
    if matches!(state, State::Asleep(_, _)) {
        return Err(ParseError::Interval);
    }
    Ok(Puzzle { sleeps })
}

fn number(text: &str, width: usize) -> Result<u16, RecordError> {
    if text.len() != width || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(RecordError::Syntax);
    }
    Ok(text.parse()?)
}

fn record(line: &str) -> Result<Record, RecordError> {
    let (stamp, event) = line
        .strip_prefix('[')
        .and_then(|rest| rest.split_once("] "))
        .ok_or(RecordError::Syntax)?;
    let (date, time) = stamp.split_once(' ').ok_or(RecordError::Syntax)?;
    let mut date = date.split('-');
    let year = number(date.next().ok_or(RecordError::Syntax)?, 4)?;
    let month = u8::try_from(number(date.next().ok_or(RecordError::Syntax)?, 2)?)?;
    let day = u8::try_from(number(date.next().ok_or(RecordError::Syntax)?, 2)?)?;
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return Err(RecordError::Range),
    };
    if date.next().is_some() || year == 0 || day == 0 || day > days {
        return Err(RecordError::Range);
    }
    let (hour, minute) = time.split_once(':').ok_or(RecordError::Syntax)?;
    let hour = u8::try_from(number(hour, 2)?)?;
    let minute = u8::try_from(number(minute, 2)?)?;
    if hour > 23 || minute > 59 {
        return Err(RecordError::Range);
    }
    let event = match event {
        "falls asleep" => Event::Sleep,
        "wakes up" => Event::Wake,
        text => {
            let id = text
                .strip_prefix("Guard #")
                .and_then(|rest| rest.strip_suffix(" begins shift"))
                .ok_or(RecordError::Syntax)?;
            if id.is_empty() || !id.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(RecordError::Syntax);
            }
            Event::Begin(NonZeroU32::new(id.parse()?).ok_or(RecordError::Range)?)
        }
    };
    if hour != 0 && !matches!(event, Event::Begin(_)) {
        return Err(RecordError::Range);
    }
    Ok(Record {
        time: Timestamp {
            date: (year, month, day),
            hour,
            minute,
        },
        event,
    })
}

#[cfg(test)]
mod tests {
    use super::{ParseError, RecordError, parse};

    #[test]
    fn empty_input_is_rejected() {
        assert_eq!(
            parse(""),
            Err(ParseError::Records(aoc_core::RecordsError::Empty)),
            "no records"
        );
    }

    #[test]
    fn lines_keep_their_order() {
        let puzzle = parse("[1518-01-01 00:09] wakes up\n[1518-01-01 00:00] Guard #7 begins shift\n[1518-01-01 00:08] falls asleep").unwrap();
        assert_eq!(
            puzzle.sleeps[0].guard.get(),
            7,
            "chronology overrides input order"
        );
        assert_eq!(
            puzzle.sleeps[0].minutes,
            8..9,
            "sleep includes start, excludes wake"
        );
    }

    #[test]
    fn lines_borrow_the_input_and_strip_crlf() {
        let input = String::from(
            "[1518-01-01 23:58] Guard #7 begins shift\r\n[1518-01-02 00:00] falls asleep\r\n[1518-01-02 00:59] wakes up\r\n",
        );
        let puzzle = parse(&input).unwrap();
        assert_eq!(
            puzzle.sleeps[0].minutes,
            0..59,
            "CRLF and prior-evening shift"
        );
        assert_eq!(
            puzzle.sleeps[0].guard.get(),
            7,
            "guard survives date boundary"
        );
        assert_eq!(
            parse("\n"),
            Err(ParseError::Records(aoc_core::RecordsError::Line {
                line: 1,
                source: RecordError::Syntax
            })),
            "blank records are invalid domain input"
        );
    }

    #[test]
    fn invalid_records_and_dates_are_rejected() {
        for stamp in [
            "1518-02-29 00:00",
            "1500-02-29 00:00",
            "1518-04-31 00:00",
            "0000-01-01 00:00",
            "1518-00-01 00:00",
            "1518-01-00 00:00",
            "1518-01-01 24:00",
            "1518-01-01 00:60",
            "1518-1-01 00:00",
        ] {
            assert!(
                matches!(
                    parse(&format!("[{stamp}] Guard #7 begins shift")),
                    Err(ParseError::Records(_))
                ),
                "{stamp}"
            );
        }
        for text in [
            "[1518-01-01 00:00] Guard #0 begins shift",
            "[1518-01-01 00:00] Guard #+1 begins shift",
            "[1518-01-01 00:00] Guard #4294967296 begins shift",
            "[1518-01-01 00:00] wakes up extra",
            "[1518-01-01 01:00] falls asleep",
            "[1518-01-01 00:00] falls asleep\n\n",
        ] {
            assert!(matches!(parse(text), Err(ParseError::Records(_))), "{text}");
        }
        for year in [1516, 1600, 2000] {
            assert_eq!(
                parse(&format!("[{year}-02-29 23:59] Guard #7 begins shift"))
                    .unwrap()
                    .sleeps,
                [],
                "valid leap date {year}"
            );
        }
    }

    #[test]
    fn impossible_transitions_are_rejected() {
        for events in [
            "00:01] wakes up",
            "00:01] falls asleep\n[1518-01-01 00:02] falls asleep",
            "00:01] falls asleep\n[1518-01-01 00:02] Guard #8 begins shift",
        ] {
            let text = format!("[1518-01-01 00:00] Guard #7 begins shift\n[1518-01-01 {events}");
            assert_eq!(parse(&text), Err(ParseError::Transition), "{events}");
        }
        assert_eq!(
            parse("[1518-01-01 00:01] falls asleep"),
            Err(ParseError::Transition),
            "sleep requires a guard"
        );
        assert_eq!(
            parse("[1518-01-01 00:00] Guard #7 begins shift\n[1518-01-01 00:01] falls asleep"),
            Err(ParseError::Interval),
            "unclosed sleep"
        );
        assert_eq!(
            parse(
                "[1518-01-01 00:00] Guard #7 begins shift\n[1518-01-01 00:01] falls asleep\n[1518-01-02 00:02] wakes up"
            ),
            Err(ParseError::Interval),
            "cross-date sleep"
        );
        assert_eq!(
            parse("[1518-01-01 00:00] Guard #7 begins shift\n[1518-01-01 00:00] falls asleep"),
            Err(ParseError::DuplicateTime),
            "duplicate timestamps have no defined order"
        );
    }
}
