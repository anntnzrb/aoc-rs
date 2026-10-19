//! Scaffolding plans for year crates, primary solutions and explorations.
//!
//! Planning is pure: it reads existing files through a [`Tree`] and returns every write,
//! so collisions are refused before anything touches the disk.

use std::path::{Path, PathBuf};

use aoc_core::{Day, Year};

use crate::cli::{Exploration, Variant};
use crate::error::OpsError;

const CARGO_TEMPLATE: &str = include_str!("../../../templates/Cargo.toml.template");
const LIB_TEMPLATE: &str = include_str!("../../../templates/lib.rs.template");
const DAY_TEMPLATE: &str = include_str!("../../../templates/day.rs.template");
const PRIMARY_TEMPLATE: &str = include_str!("../../../templates/primary.rs.template");
const EXPLORATION_TEMPLATE: &str = include_str!("../../../templates/exploration.rs.template");
const BIN_TEMPLATE: &str = include_str!("../../../templates/bin.rs.template");

/// One file the scaffold writes, relative to the workspace root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Write {
    pub(crate) path: PathBuf,
    pub(crate) text: String,
}

/// Read access to the workspace for planning.
pub(crate) trait Tree {
    /// Returns a file's content, `Some("")` for a directory, or `None` when nothing exists.
    fn read(&self, path: &Path) -> Result<Option<String>, OpsError>;
}

/// The paths of one day inside its year crate.
struct DayPaths {
    krate: PathBuf,
    module: PathBuf,
    dir: PathBuf,
    bin: PathBuf,
}

impl DayPaths {
    fn new(year: Year, day: Day, variant: &Variant) -> Self {
        let krate = PathBuf::from("crates").join(year.crate_name());
        let src = krate.join("src");
        Self {
            module: src.join(format!("day{day}.rs")),
            dir: src.join(format!("day{day}")),
            bin: src.join("bin").join(format!("{}.rs", variant.binary(day))),
            krate,
        }
    }
}

fn render(template: &str, year: Year, day: Day, variant: &Variant) -> String {
    let (module, label) = match variant {
        Variant::Primary => ("primary", "primary solution's"),
        Variant::Exploration(name) => (name.as_str(), "exploration's"),
    };
    template
        .replace("{{CRATE}}", &year.crate_name())
        .replace("{{YEAR}}", &year.to_string())
        .replace("{{DAYN}}", &day.get().to_string())
        .replace("{{DAY}}", &day.to_string())
        .replace("{{MODULE}}", module)
        .replace("{{LABEL}}", label)
        .replace("{{EXPLORATION}}", variant.label())
}

/// Adds `pub mod <name>;` to the sorted run of `pub mod` lines, or after the header when none exist.
fn add_module(text: &str, name: &str) -> String {
    let line = format!("pub mod {name};");
    let mut lines: Vec<&str> = text.lines().collect();
    let is_module = |l: &&str| l.starts_with("pub mod ") && l.ends_with(';');
    let last = lines.iter().rposition(is_module);
    let at = lines
        .iter()
        .position(|l| is_module(l) && *l > line.as_str())
        .or_else(|| last.map(|index| index.saturating_add(1)));
    if let Some(index) = at {
        lines.insert(index, &line);
    } else {
        while lines.last().is_some_and(|l| l.is_empty()) {
            lines.pop();
        }
        lines.extend(["", &line]);
    }
    let mut joined = lines.join("\n");
    joined.push('\n');
    joined
}

fn refuse_existing(tree: &impl Tree, paths: &[&Path]) -> Result<(), OpsError> {
    for path in paths {
        if tree.read(path)?.is_some() {
            return Err(OpsError::Exists(path.to_path_buf()));
        }
    }
    Ok(())
}

/// Plans a primary solution (creating the year crate on first use) or a named exploration.
pub(crate) fn plan(
    tree: &impl Tree,
    year: Year,
    day: Day,
    variant: &Variant,
) -> Result<Vec<Write>, OpsError> {
    let paths = DayPaths::new(year, day, variant);
    match variant {
        Variant::Primary => plan_primary(tree, year, day, &paths),
        Variant::Exploration(name) => plan_exploration(tree, year, day, name, &paths),
    }
}

fn plan_primary(
    tree: &impl Tree,
    year: Year,
    day: Day,
    paths: &DayPaths,
) -> Result<Vec<Write>, OpsError> {
    let primary = Variant::Primary;
    refuse_existing(tree, &[&paths.module, &paths.dir, &paths.bin])?;
    let manifest = paths.krate.join("Cargo.toml");
    let lib = paths.krate.join("src/lib.rs");
    let mut writes = Vec::new();
    let lib_text = match (tree.read(&manifest)?, tree.read(&lib)?) {
        (Some(_), Some(text)) => text,
        (None, None) => {
            refuse_existing(tree, &[&paths.krate])?;
            writes.push(Write {
                path: manifest,
                text: render(CARGO_TEMPLATE, year, day, &primary),
            });
            render(LIB_TEMPLATE, year, day, &primary)
        }
        (Some(_), None) | (None, Some(_)) => {
            return Err(OpsError::Scaffold(format!(
                "{} is incomplete: expected Cargo.toml and src/lib.rs",
                paths.krate.display()
            )));
        }
    };
    writes.extend([
        Write {
            path: lib,
            text: add_module(&lib_text, &format!("day{day}")),
        },
        Write {
            path: paths.module.clone(),
            text: render(DAY_TEMPLATE, year, day, &primary),
        },
        Write {
            path: paths.dir.join("primary.rs"),
            text: render(PRIMARY_TEMPLATE, year, day, &primary),
        },
        Write {
            path: paths.bin.clone(),
            text: render(BIN_TEMPLATE, year, day, &primary),
        },
    ]);
    Ok(writes)
}

fn plan_exploration(
    tree: &impl Tree,
    year: Year,
    day: Day,
    name: &Exploration,
    paths: &DayPaths,
) -> Result<Vec<Write>, OpsError> {
    let variant = Variant::Exploration(name.clone());
    let solution = paths.dir.join(format!("{}.rs", name.as_str()));
    refuse_existing(tree, &[&solution, &paths.bin])?;
    let (Some(day_text), Some(_)) = (
        tree.read(&paths.module)?,
        tree.read(&paths.dir.join("primary.rs"))?,
    ) else {
        return Err(OpsError::Scaffold(format!(
            "an exploration needs the primary solution of {year} day {day} first"
        )));
    };
    Ok(vec![
        Write {
            path: paths.module.clone(),
            text: add_module(&day_text, name.as_str()),
        },
        Write {
            path: solution,
            text: render(EXPLORATION_TEMPLATE, year, day, &variant),
        },
        Write {
            path: paths.bin.clone(),
            text: render(BIN_TEMPLATE, year, day, &variant),
        },
    ])
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    use aoc_core::{Day, Year};

    use super::{Tree, Write, add_module, plan};
    use crate::cli::{Exploration, Variant};
    use crate::error::OpsError;

    #[derive(Default)]
    struct Memory(BTreeMap<PathBuf, String>);

    impl Tree for Memory {
        fn read(&self, path: &Path) -> Result<Option<String>, OpsError> {
            let directory = self
                .0
                .keys()
                .any(|key| key.starts_with(path) && key != path);
            Ok(self
                .0
                .get(path)
                .cloned()
                .or_else(|| directory.then(String::new)))
        }
    }

    impl Memory {
        fn apply(&mut self, writes: Vec<Write>) {
            self.0.extend(writes.into_iter().map(|w| (w.path, w.text)));
        }

        fn get(&self, path: &str) -> &str {
            self.0.get(Path::new(path)).map_or("", String::as_str)
        }
    }

    fn date(day: u8) -> (Year, Day) {
        let year = Year::new(2025).unwrap();
        (year, Day::new(year, day).unwrap())
    }

    fn exploration(name: &str) -> Variant {
        Variant::Exploration(Exploration::parse(name).unwrap())
    }

    #[test]
    fn first_primary_creates_the_year_crate() {
        let (year, day) = date(3);
        let mut tree = Memory::default();
        tree.apply(plan(&tree, year, day, &Variant::Primary).unwrap());
        let paths: Vec<&str> = tree.0.keys().filter_map(|p| p.to_str()).collect();
        assert_eq!(
            paths,
            [
                "crates/aoc25/Cargo.toml",
                "crates/aoc25/src/bin/day03.rs",
                "crates/aoc25/src/day03/primary.rs",
                "crates/aoc25/src/day03.rs",
                "crates/aoc25/src/lib.rs",
            ],
            "crate, module, primary and binary"
        );
        assert!(
            tree.get("crates/aoc25/Cargo.toml")
                .contains("name = \"aoc25\""),
            "crate name"
        );
        assert!(
            tree.get("crates/aoc25/src/lib.rs")
                .ends_with("\n\npub mod day03;\n"),
            "module"
        );
        assert!(
            tree.get("crates/aoc25/src/bin/day03.rs")
                .contains("aoc_core::run(aoc25::day03::primary::solve)"),
            "binary calls the primary"
        );
        assert!(
            tree.get("crates/aoc25/src/day03/primary.rs")
                .starts_with("// approach: "),
            "header tags lead the solution"
        );
        let unrendered = tree.0.values().any(|text| text.contains("{{"));
        assert!(!unrendered, "every placeholder is rendered");
    }

    #[test]
    fn later_days_extend_the_sorted_module_list() {
        let mut tree = Memory::default();
        for day in [5, 2, 9] {
            let (year, day) = date(day);
            tree.apply(plan(&tree, year, day, &Variant::Primary).unwrap());
        }
        assert!(
            tree.get("crates/aoc25/src/lib.rs")
                .ends_with("\n\npub mod day02;\npub mod day05;\npub mod day09;\n"),
            "modules stay sorted"
        );
    }

    #[test]
    fn existing_paths_are_never_overwritten() {
        let (year, day) = date(1);
        let mut tree = Memory::default();
        tree.apply(plan(&tree, year, day, &Variant::Primary).unwrap());
        assert!(
            matches!(
                plan(&tree, year, day, &Variant::Primary),
                Err(OpsError::Exists(_))
            ),
            "the day exists"
        );
        let mut partial = Memory::default();
        partial
            .0
            .insert("crates/aoc25/notes.txt".into(), String::new());
        assert!(
            matches!(
                plan(&partial, year, day, &Variant::Primary),
                Err(OpsError::Exists(_))
            ),
            "an unexpected crate directory is refused"
        );
        let mut broken = Memory::default();
        broken
            .0
            .insert("crates/aoc25/Cargo.toml".into(), String::new());
        assert!(
            matches!(
                plan(&broken, year, day, &Variant::Primary),
                Err(OpsError::Scaffold(_))
            ),
            "a crate without lib.rs is refused"
        );
    }

    #[test]
    fn explorations_need_a_primary_and_register_beside_it() {
        let (year, day) = date(4);
        let mut tree = Memory::default();
        assert!(
            matches!(
                plan(&tree, year, day, &exploration("fold")),
                Err(OpsError::Scaffold(_))
            ),
            "no primary yet"
        );
        tree.apply(plan(&tree, year, day, &Variant::Primary).unwrap());
        tree.apply(plan(&tree, year, day, &exploration("fold")).unwrap());
        let module = tree.get("crates/aoc25/src/day04.rs");
        assert!(
            module.contains("pub mod fold;\npub mod primary;\n"),
            "sorted registration"
        );
        assert!(
            tree.get("crates/aoc25/src/bin/day04-fold.rs")
                .contains("aoc_core::run(aoc25::day04::fold::solve)"),
            "exploration binary"
        );
        assert!(
            tree.get("crates/aoc25/src/day04/fold.rs")
                .contains("`fold` exploration"),
            "exploration template"
        );
        assert!(
            matches!(
                plan(&tree, year, day, &exploration("fold")),
                Err(OpsError::Exists(_))
            ),
            "explorations are not overwritten"
        );
    }

    #[test]
    fn add_module_handles_headers_without_modules() {
        assert_eq!(
            add_module("//! Doc.\n\n#![forbid(unsafe_code)]\n\n\n", "a"),
            "//! Doc.\n\n#![forbid(unsafe_code)]\n\npub mod a;\n",
            "one blank line separates the header"
        );
        assert_eq!(
            add_module("pub mod b;\n", "a"),
            "pub mod a;\npub mod b;\n",
            "front"
        );
        assert_eq!(
            add_module("pub mod a;\n", "b"),
            "pub mod a;\npub mod b;\n",
            "back"
        );
        assert_eq!(
            add_module("pub mod a;\npub mod c;\n", "b"),
            "pub mod a;\npub mod b;\npub mod c;\n",
            "middle"
        );
        assert_eq!(
            add_module("//! d\npub mod x\nuse y;\n", "a"),
            "//! d\npub mod x\nuse y;\n\npub mod a;\n",
            "only complete `pub mod` lines count as modules"
        );
    }
}
