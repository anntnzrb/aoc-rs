//! Validated Advent of Code puzzle years and days.

use std::fmt;

/// First Advent of Code event.
const FIRST_YEAR: u16 = 2015;
/// Last year whose two-digit crate suffix (`aocYY`) stays unique.
const LAST_YEAR: u16 = 2099;
/// Last event with 25 puzzle days; later events have 12.
const LAST_25_DAY_YEAR: u16 = 2024;

/// A puzzle date argument outside the Advent of Code calendar.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CalendarError {
    /// The year precedes the first event or exceeds the supported range.
    #[error("year {year} is outside {FIRST_YEAR}..={LAST_YEAR}")]
    Year {
        /// The rejected year.
        year: u16,
    },
    /// The day is not a puzzle day of its year.
    #[error("day {day} is outside 1..={days} for {year}")]
    Day {
        /// The year whose calendar was checked.
        year: u16,
        /// The rejected day.
        day: u8,
        /// The number of puzzle days in that year.
        days: u8,
    },
}

/// An Advent of Code event year between 2015 and 2099.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Year(u16);

impl Year {
    /// Validates an event year.
    ///
    /// # Errors
    ///
    /// Returns [`CalendarError::Year`] outside 2015 through 2099.
    pub const fn new(year: u16) -> Result<Self, CalendarError> {
        if year >= FIRST_YEAR && year <= LAST_YEAR {
            Ok(Self(year))
        } else {
            Err(CalendarError::Year { year })
        }
    }

    /// Returns the full year.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }

    /// Returns the number of puzzle days: 25 through 2024, 12 from 2025.
    #[must_use]
    pub const fn days(self) -> u8 {
        if self.0 <= LAST_25_DAY_YEAR { 25 } else { 12 }
    }

    /// Returns the year's crate name, such as `aoc18` for 2018.
    #[must_use]
    pub fn crate_name(self) -> String {
        format!("aoc{:02}", self.0.wrapping_rem(100))
    }
}

impl fmt::Display for Year {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A puzzle day within its year's calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Day(u8);

impl Day {
    /// Validates a puzzle day for `year`.
    ///
    /// # Errors
    ///
    /// Returns [`CalendarError::Day`] when `day` is outside `1..=year.days()`.
    pub const fn new(year: Year, day: u8) -> Result<Self, CalendarError> {
        let days = year.days();
        if day >= 1 && day <= days {
            Ok(Self(day))
        } else {
            Err(CalendarError::Day {
                year: year.get(),
                day,
                days,
            })
        }
    }

    /// Returns the one-based day number.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// Renders the day zero-padded to two digits, as in paths such as `day01`.
impl fmt::Display for Day {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{CalendarError, Day, Year};

    #[test]
    fn years_are_bounded_by_the_first_event_and_unique_crate_suffixes() {
        assert_eq!(
            Year::new(2014),
            Err(CalendarError::Year { year: 2014 }),
            "no event before 2015"
        );
        assert_eq!(Year::new(2015).map(Year::get), Ok(2015), "first event");
        assert_eq!(Year::new(2099).map(Year::get), Ok(2099), "last suffix");
        assert_eq!(
            Year::new(2100),
            Err(CalendarError::Year { year: 2100 }),
            "2100 would reuse aoc00"
        );
    }

    #[test]
    fn events_from_2025_have_twelve_days() {
        assert_eq!(Year::new(2024).map(Year::days), Ok(25), "2024 has 25 days");
        assert_eq!(Year::new(2025).map(Year::days), Ok(12), "2025 has 12 days");
    }

    #[test]
    fn days_must_fall_inside_the_year() {
        let year = Year::new(2025).unwrap();
        assert_eq!(Day::new(year, 1).map(Day::get), Ok(1), "first day");
        assert_eq!(Day::new(year, 12).map(Day::get), Ok(12), "last day");
        assert_eq!(
            Day::new(year, 0),
            Err(CalendarError::Day {
                year: 2025,
                day: 0,
                days: 12
            }),
            "days are one-based"
        );
        assert_eq!(
            Day::new(year, 13),
            Err(CalendarError::Day {
                year: 2025,
                day: 13,
                days: 12
            }),
            "2025 ends on day 12"
        );
        assert_eq!(
            Day::new(Year::new(2024).unwrap(), 25).map(Day::get),
            Ok(25),
            "2024 ends on day 25"
        );
    }

    #[test]
    fn names_use_padded_numbers() {
        let year = Year::new(2018).unwrap();
        assert_eq!(year.crate_name(), "aoc18", "two-digit crate suffix");
        assert_eq!(
            Year::new(2099).unwrap().crate_name(),
            "aoc99",
            "last suffix"
        );
        assert_eq!(
            Year::new(2015).unwrap().crate_name(),
            "aoc15",
            "first suffix"
        );
        assert_eq!(year.to_string(), "2018", "full year display");
        assert_eq!(Day::new(year, 7).unwrap().to_string(), "07", "padded day");
    }
}
