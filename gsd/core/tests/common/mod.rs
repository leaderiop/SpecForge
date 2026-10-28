//! Helpers shared by the integration tests: `spec/rules.md` as data, and golden files.

use std::path::Path;

/// One rule row of `spec/rules.md`'s code tables.
#[derive(Debug)]
pub struct RuleRow {
    pub code: String,
    /// The letter under `gsd-compat`, `—` when the rule is off.
    pub compat: String,
    /// The strict column: `E`, `W`, `I`, `—` or `E (plus)`.
    pub strict: String,
}

const NOT_CODE_TABLES: &[&str] = &[
    "Judged rules' inputs",
    "Setting-gated rules",
    "Retired and kernel-dependent",
];

/// The rows of the tables that list the codes (six cells, the first `gsd/` and a letter and three digits).
pub fn rules_md() -> Vec<RuleRow> {
    let mut section = String::new();
    let mut rows = Vec::new();
    for line in include_str!("../../spec/rules.md").lines() {
        if let Some(title) = line.strip_prefix("## ") {
            section = title.to_string();
        }
        if NOT_CODE_TABLES.contains(&section.as_str()) {
            continue;
        }
        let cells: Vec<&str> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        let [code, _, _, compat, strict, _] = cells[..] else {
            continue;
        };
        let Some(local) = code.strip_prefix("gsd/") else {
            continue;
        };
        if local.len() == 4 && local[1..].bytes().all(|b| b.is_ascii_digit()) {
            rows.push(RuleRow {
                code: local.to_string(),
                compat: compat.to_string(),
                strict: strict.to_string(),
            });
        }
    }
    rows
}

/// Compares `actual` with `tests/golden/<name>`; `GSD_BLESS=1` writes it instead.
#[allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]
pub fn golden(name: &str, actual: &serde_json::Value) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    let text = serde_json::to_string_pretty(actual).expect("a JSON value prints") + "\n";
    if std::env::var_os("GSD_BLESS").is_some() {
        std::fs::write(&path, &text).expect("write the golden");
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "{} is missing: GSD_BLESS=1 cargo test writes it",
            path.display()
        )
    });
    assert!(
        expected == text,
        "{name} differs: review the change, then GSD_BLESS=1 cargo test"
    );
}
