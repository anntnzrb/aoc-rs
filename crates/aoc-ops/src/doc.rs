//! Solution header checks: the first three lines carry non-empty `approach:`, `cost:` and `trade:`.

/// Header tags every solution file opens with, in order.
const TAGS: [&str; 3] = ["approach", "cost", "trade"];

/// Reports `path:line: reason` for each missing or empty header tag.
pub(crate) fn check(path: &str, text: &str) -> Vec<String> {
    let mut lines = text.lines();
    TAGS.iter()
        .enumerate()
        .filter_map(|(index, tag)| {
            let prefix = format!("// {tag}:");
            let ok = lines
                .next()
                .and_then(|line| line.strip_prefix(&prefix))
                .is_some_and(|rest| !rest.trim().is_empty());
            (!ok).then(|| {
                format!(
                    "{path}:{}: expected `{prefix}` with text",
                    index.saturating_add(1)
                )
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::check;

    #[test]
    fn requires_three_ordered_non_empty_tags() {
        let good = "// approach: a\n// cost: b\n// trade: c\n\n//! Doc.\n";
        assert!(check("f.rs", good).is_empty(), "complete header");
        assert_eq!(
            check("f.rs", "// approach: a\n// cost:  \n// trade: c\n"),
            ["f.rs:2: expected `// cost:` with text"],
            "empty tag"
        );
        assert_eq!(
            check("f.rs", "// cost: b\n// approach: a\n// trade: c\n"),
            [
                "f.rs:1: expected `// approach:` with text",
                "f.rs:2: expected `// cost:` with text"
            ],
            "order matters"
        );
        assert_eq!(check("f.rs", "").len(), 3, "every tag is missing");
    }
}
