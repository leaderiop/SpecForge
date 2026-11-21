//! The bound a gsd/E516 tail applies, as the docs state it.

use super::*;

/// The English words for the counts a doc may state as a bound, by value.
const WORDS: [&str; 11] = [
    "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
];

/// The bound the tail applies is the one the docs state: `at most <NAMED in words>` in the gsd/E516 row of the rules,
/// in the paragraph (not a table) of the verify spec and of the README that names gsd/E516, and in ADR 0383's D2.
#[test]
fn the_docs_state_the_bound_the_tail_applies() {
    assert!(NAMED < WORDS.len(), "no word for {NAMED}");
    let bound = format!("at most {}", WORDS[NAMED]);
    let root = env!("CARGO_MANIFEST_DIR");
    let read = |path: &str| -> String {
        let path = format!("{root}/{path}");
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
    };
    let paragraph = |text: &str| -> String {
        text.split("\n\n")
            .find(|p| p.contains("gsd/E516") && !p.trim_start().starts_with('|'))
            .unwrap_or_else(|| panic!("no paragraph names gsd/E516"))
            .to_string()
    };
    let rules = read("spec/rules.md");
    let row = rules
        .lines()
        .find(|l| l.starts_with("| gsd/E516 |"))
        .expect("the gsd/E516 row");
    let adr = read(
        "../../docs/adr/0383-a-refusal-names-the-entities-owing-the-verdicts-that-block-its-runs.md",
    );
    let (_, d2) = adr.split_once("\n## D2.").expect("ADR 0383 has a D2");
    let d2 = d2.split("\n## ").next().unwrap_or_default();
    for (place, text) in [
        ("spec/rules.md, the gsd/E516 row", row.to_string()),
        ("spec/verify.md", paragraph(&read("spec/verify.md"))),
        ("README.md", paragraph(&read("README.md"))),
        ("ADR 0383, D2", d2.to_string()),
    ] {
        assert!(
            text.contains(&bound),
            "{place} does not say `{bound}`: {text}"
        );
    }
}
