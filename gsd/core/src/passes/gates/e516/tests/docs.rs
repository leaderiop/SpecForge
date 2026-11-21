//! The bound a gsd/E516 tail applies, as every doc that states it states it.

use super::*;

/// The English words for the counts a doc may state as a bound, by value.
const WORDS: [&str; 11] = [
    "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
];

/// `text` with every run of whitespace one space, so a statement reads the same wherever its lines break.
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `word` with its first letter upper case, as a heading starts it.
fn capital(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// Every statement of the bound the tail applies gives [`NAMED`] in words, wherever its lines break: the pass's module
/// doc; the gsd/E516 row of the rules; the paragraph (not a table) of the verify spec and of the README that names
/// gsd/E516; ADR 0379's pointer to ADR 0383; and ADR 0383's D2 (its heading, its rule and its reason), Limits and
/// Consequences. A change of the bound that leaves one of them stale fails here.
#[test]
fn the_docs_state_the_bound_the_tail_applies() {
    assert!(NAMED < WORDS.len(), "no word for {NAMED}");
    let (word, heading) = (WORDS[NAMED], capital(WORDS[NAMED]));
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
    let module_doc = read("src/passes/gates/e516.rs")
        .lines()
        .filter_map(|line| line.strip_prefix("//!"))
        .collect::<Vec<_>>()
        .join("\n");
    let rules = read("spec/rules.md");
    let row = rules
        .lines()
        .find(|l| l.starts_with("| gsd/E516 |"))
        .expect("the gsd/E516 row")
        .to_string();
    let adr = |number: &str| -> String {
        let dir = format!("{root}/../../docs/adr");
        let name = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{dir}: {e}"))
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .find(|name| name.starts_with(&format!("{number}-")))
            .unwrap_or_else(|| panic!("no ADR {number} in {dir}"));
        read(&format!("../../docs/adr/{name}"))
    };
    let per_holder = format!("at most {word} per holder and judge");
    let places = [
        (
            "the module doc of gsd/E516",
            module_doc,
            vec![per_holder.clone()],
        ),
        (
            "spec/rules.md, the gsd/E516 row",
            row,
            vec![per_holder.clone()],
        ),
        (
            "spec/verify.md",
            paragraph(&read("spec/verify.md")),
            vec![per_holder.clone()],
        ),
        (
            "README.md",
            paragraph(&read("README.md")),
            vec![per_holder.clone()],
        ),
        ("ADR 0379", adr("0379"), vec![per_holder]),
        (
            "ADR 0383",
            adr("0383"),
            vec![
                format!("## D2. {heading} names per holder and judge"),
                format!("A group names at most {word} entities"),
                format!("{word} keep the help readable"),
                format!("Past {word}, a group's entities are counted"),
                format!("each by at most {word} ids"),
                format!("grows by at most {word} ids per holder and judge"),
                format!("D2 ({word} names)"),
            ],
        ),
    ];
    for (place, text, statements) in places {
        let text = flat(&text);
        for statement in statements {
            assert!(
                text.contains(&statement),
                "{place} does not say `{statement}`: {text}"
            );
        }
    }
}
