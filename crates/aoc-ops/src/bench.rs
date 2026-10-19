//! Pure benchmark statistics, measurement parsing and canonical CSV rows.

/// The shared CSV schema used by every `AoC` repository.
pub(crate) const CSV_HEADER: &str = "date,commit,host,os,cpu,compiler,std,year,day,solution,runs,work_med,work_min,work_max,instr_med,wall_min_us,wall_med_us,cpu_med_us,rss_kib";

/// Median, minimum and maximum of a sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Summary {
    pub(crate) median: i128,
    pub(crate) minimum: i128,
    pub(crate) maximum: i128,
}

impl Summary {
    /// Summarizes a non-empty sample; an even count averages the middle pair, rounding down.
    pub(crate) fn of(values: &[i128]) -> Option<Self> {
        let mut sorted = values.to_vec();
        sorted.sort_unstable();
        let minimum = *sorted.first()?;
        let maximum = *sorted.last()?;
        let half = sorted.len() / 2;
        let upper = *sorted.get(half)?;
        let median = if sorted.len() % 2 == 1 {
            upper
        } else {
            let lower = *sorted.get(half.checked_sub(1)?)?;
            lower.checked_add(upper)?.div_euclid(2)
        };
        Some(Self {
            median,
            minimum,
            maximum,
        })
    }

    pub(crate) fn render(self) -> String {
        format!("{} [{}, {}]", self.median, self.minimum, self.maximum)
    }
}

/// Startup-subtracted work for full/empty instruction-count pairs; negative noise is kept.
pub(crate) fn work(pairs: &[(i128, i128)]) -> Option<Summary> {
    let deltas: Option<Vec<i128>> = pairs
        .iter()
        .map(|(full, empty)| full.checked_sub(*empty))
        .collect();
    Summary::of(&deltas?)
}

/// Sums `instructions:u` counters from `perf stat -x,` output, including hybrid PMU rows.
pub(crate) fn instructions(text: &str) -> Option<i128> {
    let counts: Vec<i128> = text
        .lines()
        .filter_map(|line| {
            let mut fields = line.split(',');
            let value = fields.next()?.trim();
            let event = fields.nth(1)?.trim();
            let counted = event == "instructions:u" || event.ends_with("/instructions/u");
            counted.then(|| value.parse::<i128>().ok()).flatten()
        })
        .filter(|count| *count >= 0)
        .collect();
    if counts.is_empty() {
        None
    } else {
        counts.into_iter().try_fold(0_i128, i128::checked_add)
    }
}

/// Parses GNU time's `%U,%S,%M` into CPU (user + system) µs and peak RSS KiB.
pub(crate) fn resources(text: &str) -> Option<(i128, i128)> {
    let line = text.lines().rev().find(|line| !line.trim().is_empty())?;
    let fields: Vec<&str> = line.trim().split(',').collect();
    let [user, system, rss] = fields.as_slice() else {
        return None;
    };
    let cpu = micros(user)?.checked_add(micros(system)?)?;
    let rss = rss.parse::<i128>().ok().filter(|kib| *kib >= 0)?;
    Some((cpu, rss))
}

/// Converts a non-negative decimal seconds string to whole microseconds without floats.
fn micros(seconds: &str) -> Option<i128> {
    let (whole, fraction) = seconds.split_once('.').unwrap_or((seconds, ""));
    if whole.is_empty()
        || fraction.len() > 6
        || !(whole.bytes().chain(fraction.bytes())).all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let padded = format!("{fraction:0<6}");
    whole
        .parse::<i128>()
        .ok()?
        .checked_mul(1_000_000)?
        .checked_add(padded.parse::<i128>().ok()?)
}

/// Quotes a CSV text field, doubling embedded quotes.
pub(crate) fn quote(text: &str) -> String {
    format!("\"{}\"", text.replace('"', "\"\""))
}

/// Host and revision metadata captured once per benchmark.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Metadata {
    pub(crate) date: String,
    pub(crate) commit: String,
    pub(crate) host: String,
    pub(crate) os: String,
    pub(crate) cpu: String,
    pub(crate) compiler: String,
    pub(crate) standard: String,
}

/// One solution's whole-process measurements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Measurement {
    pub(crate) solution: String,
    pub(crate) runs: usize,
    pub(crate) work: Option<Summary>,
    pub(crate) instructions: Option<Summary>,
    pub(crate) wall: Summary,
    pub(crate) cpu: Summary,
    pub(crate) rss_kib: i128,
}

/// Serializes one canonical row: text fields quoted, unavailable metrics empty.
pub(crate) fn csv_row(meta: &Metadata, year: u16, day: u8, m: &Measurement) -> String {
    let optional = |value: Option<i128>| value.map_or_else(String::new, |n| n.to_string());
    [
        quote(&meta.date),
        quote(&meta.commit),
        quote(&meta.host),
        quote(&meta.os),
        quote(&meta.cpu),
        quote(&meta.compiler),
        quote(&meta.standard),
        year.to_string(),
        day.to_string(),
        quote(&m.solution),
        m.runs.to_string(),
        optional(m.work.map(|s| s.median)),
        optional(m.work.map(|s| s.minimum)),
        optional(m.work.map(|s| s.maximum)),
        optional(m.instructions.map(|s| s.median)),
        m.wall.minimum.to_string(),
        m.wall.median.to_string(),
        m.cpu.median.to_string(),
        m.rss_kib.to_string(),
    ]
    .join(",")
}

/// Canonical records require the main branch and a clean tracked tree.
pub(crate) fn canonical(branch: &str, status: &str) -> Result<(), &'static str> {
    if branch.trim() != "main" {
        Err("record requires the main branch")
    } else if !status.trim().is_empty() {
        Err("record requires a clean tracked tree")
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CSV_HEADER, Measurement, Metadata, Summary, canonical, csv_row, instructions, quote,
        resources, work,
    };

    #[test]
    fn summaries_report_median_and_range() {
        assert_eq!(
            Summary::of(&[5, 1, 3]),
            Some(Summary {
                median: 3,
                minimum: 1,
                maximum: 5
            }),
            "odd count"
        );
        assert_eq!(
            Summary::of(&[4, 1, 2, 9]).map(|s| s.median),
            Some(3),
            "even count"
        );
        assert_eq!(
            Summary::of(&[1, 2]).map(|s| s.median),
            Some(1),
            "rounds down"
        );
        assert_eq!(Summary::of(&[]), None, "empty sample");
        assert_eq!(
            Summary::of(&[2, 1, 3]).map(Summary::render),
            Some("2 [1, 3]".to_owned()),
            "render"
        );
    }

    #[test]
    fn work_subtracts_the_empty_baseline() {
        assert_eq!(
            work(&[(110, 100), (130, 100), (95, 100)]),
            Some(Summary {
                median: 10,
                minimum: -5,
                maximum: 30
            }),
            "negative noise is kept"
        );
    }

    #[test]
    fn perf_counters_are_summed_across_hybrid_pmus() {
        let text = "1000,,cpu_core/instructions/u,1,100.00,,\n\
                    234,,cpu_atom/instructions/u,1,100.00,,\n\
                    <not counted>,,cpu_x/instructions/u,0,0.00,,\n\
                    9,,cycles:u,1,100.00,,\n";
        assert_eq!(instructions(text), Some(1234), "hybrid rows add");
        assert_eq!(
            instructions("77,,instructions:u,1,100.00,,\n"),
            Some(77),
            "plain row"
        );
        assert_eq!(instructions("9,,cycles:u,1,100.00,,\n"), None, "no counter");
    }

    #[test]
    fn gnu_time_resources_are_parsed_exactly() {
        assert_eq!(
            resources("noise\n0.01,0.02,2048\n"),
            Some((30_000, 2048)),
            "user+system, RSS"
        );
        assert_eq!(resources("1,0.000001,1"), Some((1_000_001, 1)), "units");
        for bad in [
            "",
            "1,2",
            "1,2,-1",
            "a,0,1",
            "0.1234567,0,1",
            ".5,0,1",
            "1,2,3,4",
        ] {
            assert_eq!(resources(bad), None, "{bad:?} is rejected");
        }
    }

    #[test]
    fn csv_rows_follow_the_shared_schema() {
        let meta = Metadata {
            date: "2026-01-01T00:00:00Z".to_owned(),
            commit: "abc".to_owned(),
            host: "h".to_owned(),
            os: "Linux \"x\"".to_owned(),
            cpu: "c".to_owned(),
            compiler: "rustc 1".to_owned(),
            standard: "Rust 2024".to_owned(),
        };
        let one = Summary {
            median: 2,
            minimum: 1,
            maximum: 3,
        };
        let m = Measurement {
            solution: "primary".to_owned(),
            runs: 10,
            work: None,
            instructions: Some(one),
            wall: one,
            cpu: one,
            rss_kib: 9,
        };
        let row = csv_row(&meta, 2018, 1, &m);
        assert_eq!(
            row,
            "\"2026-01-01T00:00:00Z\",\"abc\",\"h\",\"Linux \"\"x\"\"\",\"c\",\"rustc 1\",\
             \"Rust 2024\",2018,1,\"primary\",10,,,,2,1,2,2,9",
            "unavailable work stays empty"
        );
        assert_eq!(
            row.matches(',').count(),
            CSV_HEADER.matches(',').count(),
            "one field per column"
        );
        assert_eq!(quote("a,b"), "\"a,b\"", "delimiters are quoted");
    }

    #[test]
    fn records_require_clean_main() {
        assert_eq!(canonical("main\n", ""), Ok(()), "clean main");
        assert!(canonical("work/x", "").is_err(), "other branch");
        assert!(canonical("main", " M a.rs\n").is_err(), "dirty tree");
    }
}
