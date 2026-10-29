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
    "Retired and kernel-provided",
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

#[allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]
pub fn describe(category: &str) -> serde_json::Value {
    let host = specforge_extension_sdk::testing::MockHost::new(gsd_core::build());
    let text = host
        .describe_json(category)
        .unwrap_or_else(|| panic!("no category {category}"));
    serde_json::from_str(&text).expect("describe answers JSON")
}

/// The names of the fields a kind declares, in order.
#[allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]
pub fn fields_of(kind: &str) -> Vec<String> {
    let entities = describe("entities");
    let entity = entities["items"]
        .as_array()
        .and_then(|kinds| kinds.iter().find(|k| k["name"] == kind))
        .unwrap_or_else(|| panic!("no kind {kind}"));
    entity["fields"]
        .as_array()
        .expect("fields")
        .iter()
        .map(|f| f["name"].as_str().expect("a name").to_string())
        .collect()
}

#[allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]
pub fn field_of(kind: &str, field: &str) -> serde_json::Value {
    let entities = describe("entities");
    let entity = entities["items"]
        .as_array()
        .and_then(|kinds| kinds.iter().find(|k| k["name"] == kind))
        .unwrap_or_else(|| panic!("no kind {kind}"));
    entity["fields"]
        .as_array()
        .and_then(|fields| fields.iter().find(|f| f["name"] == field))
        .unwrap_or_else(|| panic!("no field {kind}.{field}"))
        .clone()
}

/// One row of `spec/rules.md`'s "Judged rules' inputs" table.
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]
pub struct JudgedRow {
    /// The local codes: `["W311", "W313", ...]`.
    pub codes: Vec<String>,
    pub target: String,
    /// `(name, selector)`, in the order the row lists them.
    pub inputs: Vec<(String, String)>,
    /// The field a row's `unless_set` names, when the row says it.
    pub unless_set: Option<String>,
    pub binds: String,
}

/// The selector a cell piece (`` `name` selector (note) ``) names, cut at its note.
fn input_of(piece: &str) -> (String, String) {
    let rest = piece.trim().strip_prefix('`').expect("a quoted input name");
    let (name, rest) = rest.split_once('`').expect("a closing quote");
    let rest = rest.trim().replace("\\|", "|");
    let end = [" (", ";"]
        .iter()
        .filter_map(|cut| rest.find(cut))
        .min()
        .unwrap_or(rest.len());
    (name.to_string(), rest[..end].trim().to_string())
}

#[allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]
pub fn judged_rows() -> Vec<JudgedRow> {
    let text = include_str!("../../spec/rules.md");
    let section = text
        .split("## Judged rules' inputs")
        .nth(1)
        .and_then(|rest| rest.split("\n## ").next())
        .expect("the judged inputs section");
    let mut rows = Vec::new();
    for line in section.lines().filter(|l| l.starts_with("| gsd/")) {
        let cells: Vec<&str> = line.trim().trim_matches('|').split(" | ").collect();
        let [rules, target, inputs, binds] = cells[..] else {
            panic!("a judged row has four cells: {line}");
        };
        let head = rules.split(" (").next().unwrap_or(rules);
        let codes = head
            .split(',')
            .map(|c| c.trim().trim_start_matches("gsd/").to_string())
            .collect();
        let unless_set = inputs
            .split("`unless_set ")
            .nth(1)
            .and_then(|r| r.split(|c: char| !c.is_alphanumeric() && c != '_').next())
            .map(String::from);
        rows.push(JudgedRow {
            codes,
            target: target.trim_matches('`').to_string(),
            inputs: inputs.split(" · ").map(input_of).collect(),
            unless_set,
            binds: binds.to_string(),
        });
    }
    rows
}

/// The judged descriptor `describe` prints for a local code.
#[allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]
pub fn judged_descriptor(code: &str) -> Option<serde_json::Value> {
    describe("validation_rules")["items"]
        .as_array()
        .expect("rules")
        .iter()
        .find(|r| r["code"] == code && r["check"] == "judged")
        .map(|r| r["judged"].clone())
}

/// Asserts that the descriptor of `code` reads exactly what `rules.md` lists for it: each target with its inputs in
/// order, and its `unless_set`.
#[allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]
pub fn assert_inputs_match_rules_md(code: &str) {
    let rows: Vec<JudgedRow> = judged_rows()
        .into_iter()
        .filter(|r| r.codes.iter().any(|c| c == code))
        .collect();
    assert!(!rows.is_empty(), "rules.md lists no judged row for {code}");
    let descriptor = judged_descriptor(code).unwrap_or_else(|| panic!("{code} is not judged"));
    let targets = descriptor["targets"].as_array().expect("targets");
    assert_eq!(targets.len(), rows.len(), "{code}: targets");
    for row in rows {
        let target = targets
            .iter()
            .find(|t| t["kind"] == row.target.as_str())
            .unwrap_or_else(|| panic!("{code}: no target {}", row.target));
        let inputs: Vec<(String, String)> = target["inputs"]
            .as_array()
            .expect("inputs")
            .iter()
            .map(|i| {
                (
                    i["name"].as_str().expect("name").to_string(),
                    i["selector"].as_str().expect("selector").to_string(),
                )
            })
            .collect();
        assert!(!inputs.is_empty(), "{code} {}: no inputs", row.target);
        assert_eq!(inputs, row.inputs, "{code} {}", row.target);
        let unless: Vec<String> = target["unless_set"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        assert_eq!(
            unless,
            row.unless_set.iter().cloned().collect::<Vec<_>>(),
            "{code} {}",
            row.target
        );
    }
}
