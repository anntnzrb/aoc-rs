//! Argument parsing into validated operations, before any filesystem or network access.

use aoc_core::{Day, Year};

use crate::error::OpsError;

pub(crate) const USAGE: &str = "usage: aoc-ops <command>
  fetch YEAR DAY [--force]      cache inputs/YEAR/dayDD.txt
  puzzle YEAR DAY               refresh puzzles/YEAR/DD.txt
  submit YEAR DAY PART ANSWER   post an answer; exit 0 right, 1 wrong, 2 wait/done, 3 unknown
  new YEAR DAY [EXPLORATION]    scaffold a primary solution or a named exploration
  run YEAR DAY [EXPLORATION]    run a release build on the cached input
  test [YEAR [DAY]]             check every solution against its fixtures
  bench YEAR DAY                measure every solution of a day
  record YEAR DAY               measure and append canonical rows to bench/YEAR.csv
  doc                           check solution header tags";

/// Longest accepted exploration name, keeping binary names readable.
const MAX_EXPLORATION_CHARS: usize = 32;

/// Names that would collide with Rust keywords, the test module or the primary label.
const RESERVED: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "crate",
    "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl",
    "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "primary", "priv",
    "pub", "ref", "return", "self", "static", "struct", "super", "tests", "trait", "true", "try",
    "type", "typeof", "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

/// Whether `fetch` reuses the cached input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refresh {
    Cached,
    Force,
}

/// The puzzle part being submitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Part {
    One,
    Two,
}

impl Part {
    pub(crate) const fn level(self) -> &'static str {
        match self {
            Self::One => "1",
            Self::Two => "2",
        }
    }
}

/// A non-empty answer token of ASCII letters, digits, commas and hyphens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Answer(String);

impl Answer {
    fn parse(text: &str) -> Result<Self, OpsError> {
        if !text.is_empty()
            && text
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b',' || byte == b'-')
        {
            Ok(Self(text.to_owned()))
        } else {
            Err(OpsError::Usage(
                "answer must be a non-empty token of ASCII letters, digits, commas and hyphens"
                    .to_owned(),
            ))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// A lowercase snake-case exploration name, usable as a module and binary suffix.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Exploration(String);

impl Exploration {
    pub(crate) fn parse(text: &str) -> Result<Self, OpsError> {
        let mut bytes = text.bytes();
        let valid = bytes.next().is_some_and(|first| first.is_ascii_lowercase())
            && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
            && text.len() <= MAX_EXPLORATION_CHARS
            && !RESERVED.contains(&text);
        if valid {
            Ok(Self(text.to_owned()))
        } else {
            Err(OpsError::Usage(format!(
                "exploration must be a lowercase snake_case name of at most \
                 {MAX_EXPLORATION_CHARS} characters, not a Rust keyword, `primary` or `tests`"
            )))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// A day's primary solution or one of its explorations.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Variant {
    Primary,
    Exploration(Exploration),
}

impl Variant {
    /// Returns the Cargo binary name: `dayDD` or `dayDD-<exploration>`.
    pub(crate) fn binary(&self, day: Day) -> String {
        match self {
            Self::Primary => format!("day{day}"),
            Self::Exploration(name) => format!("day{day}-{}", name.as_str()),
        }
    }

    /// Returns the label used in reports and benchmark records.
    pub(crate) fn label(&self) -> &str {
        match self {
            Self::Primary => "primary",
            Self::Exploration(name) => name.as_str(),
        }
    }
}

/// Which days the fixture runner covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    All,
    Year(Year),
    Day(Year, Day),
}

/// Whether a benchmark only reports or also appends canonical records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Record {
    Report,
    Append,
}

/// A validated operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    Fetch(Year, Day, Refresh),
    Puzzle(Year, Day),
    Submit(Year, Day, Part, Answer),
    New(Year, Day, Variant),
    Run(Year, Day, Variant),
    Test(Scope),
    Bench(Year, Day, Record),
    Doc,
}

fn year(text: &str) -> Result<Year, OpsError> {
    let number = text
        .parse::<u16>()
        .map_err(|source| OpsError::Usage(format!("invalid YEAR {text:?}: {source}")))?;
    Ok(Year::new(number)?)
}

fn date(year_text: &str, day_text: &str) -> Result<(Year, Day), OpsError> {
    let year = year(year_text)?;
    let number = day_text
        .parse::<u8>()
        .map_err(|source| OpsError::Usage(format!("invalid DAY {day_text:?}: {source}")))?;
    Ok((year, Day::new(year, number)?))
}

fn variant(name: Option<&str>) -> Result<Variant, OpsError> {
    name.map_or(Ok(Variant::Primary), |text| {
        Exploration::parse(text).map(Variant::Exploration)
    })
}

/// Parses command-line arguments, excluding the program name.
pub(crate) fn parse(args: &[String]) -> Result<Command, OpsError> {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["fetch", y, d] => date(y, d).map(|(y, d)| Command::Fetch(y, d, Refresh::Cached)),
        ["fetch", y, d, "--force"] => date(y, d).map(|(y, d)| Command::Fetch(y, d, Refresh::Force)),
        ["puzzle", y, d] => date(y, d).map(|(y, d)| Command::Puzzle(y, d)),
        ["submit", y, d, part, answer] => {
            let (y, d) = date(y, d)?;
            let part = match *part {
                "1" => Part::One,
                "2" => Part::Two,
                _ => return Err(OpsError::Usage("PART must be 1 or 2".to_owned())),
            };
            Ok(Command::Submit(y, d, part, Answer::parse(answer)?))
        }
        ["new", y, d, rest @ ..] if rest.len() <= 1 => {
            let (y, d) = date(y, d)?;
            Ok(Command::New(y, d, variant(rest.first().copied())?))
        }
        ["run", y, d, rest @ ..] if rest.len() <= 1 => {
            let (y, d) = date(y, d)?;
            Ok(Command::Run(y, d, variant(rest.first().copied())?))
        }
        ["test"] => Ok(Command::Test(Scope::All)),
        ["test", y] => year(y).map(|y| Command::Test(Scope::Year(y))),
        ["test", y, d] => date(y, d).map(|(y, d)| Command::Test(Scope::Day(y, d))),
        ["bench", y, d] => date(y, d).map(|(y, d)| Command::Bench(y, d, Record::Report)),
        ["record", y, d] => date(y, d).map(|(y, d)| Command::Bench(y, d, Record::Append)),
        ["doc"] => Ok(Command::Doc),
        _ => Err(OpsError::Usage(USAGE.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use aoc_core::{Day, Year};

    use super::{Command, Exploration, Part, Record, Refresh, Scope, Variant, parse};

    fn words(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    fn date(year: u16, day: u8) -> (Year, Day) {
        let year = Year::new(year).unwrap();
        (year, Day::new(year, day).unwrap())
    }

    #[test]
    fn parses_every_operation() {
        let (y, d) = date(2018, 3);
        let cases = [
            ("fetch 2018 3", Command::Fetch(y, d, Refresh::Cached)),
            (
                "fetch 2018 03 --force",
                Command::Fetch(y, d, Refresh::Force),
            ),
            ("puzzle 2018 3", Command::Puzzle(y, d)),
            ("new 2018 3", Command::New(y, d, Variant::Primary)),
            ("run 2018 3", Command::Run(y, d, Variant::Primary)),
            ("test", Command::Test(Scope::All)),
            ("test 2018", Command::Test(Scope::Year(y))),
            ("test 2018 3", Command::Test(Scope::Day(y, d))),
            ("bench 2018 3", Command::Bench(y, d, Record::Report)),
            ("record 2018 3", Command::Bench(y, d, Record::Append)),
            ("doc", Command::Doc),
        ];
        for (text, expected) in cases {
            assert_eq!(parse(&words(text)).unwrap(), expected, "{text}");
        }
    }

    #[test]
    fn submit_validates_part_and_answer_before_network() {
        let (y, d) = date(2018, 3);
        let Command::Submit(year, day, Part::Two, answer) =
            parse(&words("submit 2018 3 2 ab-1,C")).unwrap()
        else {
            panic!("submit parses into Command::Submit");
        };
        assert_eq!((year, day), (y, d), "date is kept");
        assert_eq!(answer.as_str(), "ab-1,C", "answer is kept verbatim");
        assert_eq!(Part::One.level(), "1", "part 1 posts level 1");
        assert_eq!(Part::Two.level(), "2", "part 2 posts level 2");
        for bad in [
            "submit 2018 3 3 1",
            "submit 2018 3 1 a.b",
            "submit 2018 3 1 a=b",
        ] {
            assert!(parse(&words(bad)).is_err(), "{bad} is rejected");
        }
        assert!(
            parse(&["submit", "2018", "3", "1", ""].map(str::to_owned)).is_err(),
            "empty answer is rejected"
        );
    }

    #[test]
    fn rejects_dates_outside_the_calendar() {
        for bad in [
            "fetch 2014 1",
            "fetch 2100 1",
            "fetch 2018 0",
            "fetch 2018 26",
            "fetch 2025 13",
            "fetch 2018 x",
            "fetch 2018 1 --forced",
            "test 2018 1 2",
            "new 2018 1 a b",
            "run 2018 1 a b",
            "frobnicate",
        ] {
            assert!(parse(&words(bad)).is_err(), "{bad} is rejected");
        }
        assert!(
            parse(&words("fetch 2025 12")).is_ok(),
            "2025 ends on day 12"
        );
    }

    #[test]
    fn explorations_name_modules_and_binaries() {
        let (_, d) = date(2018, 3);
        let name = Exploration::parse("two_pass2").unwrap();
        let exploration = Variant::Exploration(name);
        assert_eq!(exploration.binary(d), "day03-two_pass2", "binary suffix");
        assert_eq!(exploration.label(), "two_pass2", "report label");
        assert_eq!(Variant::Primary.binary(d), "day03", "primary binary");
        assert_eq!(Variant::Primary.label(), "primary", "primary label");
        assert_eq!(
            parse(&words("new 2018 3 two_pass2")).unwrap(),
            Command::New(y_of(2018), d, exploration.clone()),
            "new accepts an exploration"
        );
        assert_eq!(
            parse(&words("run 2018 3 two_pass2")).unwrap(),
            Command::Run(y_of(2018), d, exploration),
            "run accepts an exploration"
        );
    }

    fn y_of(year: u16) -> Year {
        Year::new(year).unwrap()
    }

    #[test]
    fn exploration_names_are_restricted() {
        for bad in [
            "",
            "Recursive",
            "2fast",
            "two-pass",
            "primary",
            "tests",
            "mod",
            "a b",
        ] {
            assert!(Exploration::parse(bad).is_err(), "{bad:?} is rejected");
        }
        assert!(
            Exploration::parse(&"a".repeat(32)).is_ok(),
            "32 characters fit"
        );
        assert!(
            Exploration::parse(&"a".repeat(33)).is_err(),
            "33 characters do not"
        );
        assert!(Exploration::parse("z").is_ok(), "one letter is enough");
    }
}
