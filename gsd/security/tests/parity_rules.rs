//! The parity rules check for `@gsd/security` (plan 16 T25): the code book is the codes table of `spec/README.md`,
//! every code fires on a named project and stays silent on its twin, and every divergence tag the crate names is listed
//! under "Known divergences" in ACCEPTANCE.

#[path = "parity/evidence.rs"]
mod evidence;

use std::path::{Path, PathBuf};

use evidence::RULE_EVIDENCE;
use gsd_security::codes::CODES;

const SPEC: &str = include_str!("../spec/README.md");
const ACCEPTANCE: &str = include_str!("../../parity/ACCEPTANCE.md");

/// One row of the codes table of `spec/README.md`.
struct Row {
    local: String,
    severity: String,
    gated_by: String,
}

/// The rows of the "Codes" section: six cells, the first `gsd.security/` and a letter and three digits.
fn codes_table() -> Vec<Row> {
    let section = SPEC
        .split("\n## Codes\n")
        .nth(1)
        .and_then(|s| s.split("\n## ").next())
        .expect("a Codes section");
    section
        .lines()
        .filter_map(|line| {
            let cells: Vec<&str> = line
                .trim()
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect();
            let [code, _, severity, gated_by, _, _] = cells[..] else {
                return None;
            };
            let local = code.trim_matches('`').strip_prefix("gsd.security/")?;
            (local.len() == 4 && local[1..].bytes().all(|b| b.is_ascii_digit())).then(|| Row {
                local: local.to_string(),
                severity: severity.to_string(),
                gated_by: gated_by.to_string(),
            })
        })
        .collect()
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// `(name, body)` of every `#[test]` fn under `tests/`: a body runs from the `fn` line to the first lone `}` at the
/// fn's own indentation.
fn test_fns() -> Vec<(String, String)> {
    let mut files = Vec::new();
    rust_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests"),
        &mut files,
    );
    let mut found = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).expect("a source file");
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.trim() != "#[test]" {
                continue;
            }
            let Some(start) = (i + 1..lines.len().min(i + 4)).find(|&j| lines[j].contains("fn "))
            else {
                continue;
            };
            let head = lines[start];
            let indent = head.len() - head.trim_start().len();
            let name = head
                .split("fn ")
                .nth(1)
                .and_then(|r| r.split('(').next())
                .unwrap_or_default()
                .trim()
                .to_string();
            let close = format!("{}}}", " ".repeat(indent));
            let end = (start..lines.len())
                .find(|&j| lines[j] == close)
                .unwrap_or(lines.len() - 1);
            found.push((name, lines[start..=end].join("\n")));
        }
    }
    found
}

/// Whether the test fn `name` mentions `code` (any case) as a word in its name or its body.
fn names_code(fns: &[(String, String)], name: &str, code: &str) -> bool {
    let code = code.to_lowercase();
    fns.iter().any(|(n, body)| {
        n == name
            && body.to_lowercase().match_indices(&code).any(|(at, _)| {
                let body = body.to_lowercase();
                let before = body[..at].chars().next_back();
                let after = body[at + code.len()..].chars().next();
                !before.is_some_and(char::is_alphanumeric)
                    && !after.is_some_and(char::is_alphanumeric)
            })
    })
}

#[test]
fn the_code_book_is_the_codes_table() {
    let rows = codes_table();
    let table: Vec<&str> = rows.iter().map(|r| r.local.as_str()).collect();
    let book: Vec<&str> = CODES.iter().map(|(code, _, _)| *code).collect();
    assert_eq!(
        table, book,
        "spec/README.md's Codes table lists the code book, in order"
    );
    for row in &rows {
        let severity = match row.local.as_bytes()[0] {
            b'E' => "error",
            b'W' => "warning",
            other => panic!("{}: letter {}", row.local, other as char),
        };
        assert_eq!(row.severity, severity, "{}", row.local);
    }
    // E101 applies whatever the hook says; W301 is switched off with the enforcement
    let gated = |code: &str| {
        rows.iter()
            .find(|r| r.local == code)
            .expect("a row")
            .gated_by
            .clone()
    };
    assert!(gated("E101").starts_with("none"));
    assert_eq!(gated("W301"), "`workflow.security_enforcement`");
}

#[test]
fn every_code_fires_and_stays_silent() {
    let fns = test_fns();
    let listed: Vec<&str> = RULE_EVIDENCE.iter().map(|(c, _, _)| *c).collect();
    let book: Vec<&str> = CODES.iter().map(|(code, _, _)| *code).collect();
    assert_eq!(listed, book, "RULE_EVIDENCE lists the code book, in order");
    let mut bad = Vec::new();
    for (code, fires, silent) in RULE_EVIDENCE {
        for (what, name) in [("fires", fires), ("silent", silent)] {
            if !names_code(&fns, name, code) {
                bad.push(format!("{code} {what}: no test `{name}` that names {code}"));
            }
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}

/// The divergence tags the crate names: `[accept-risk-human]` in a source, a test or the model.
fn tags_named() -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for dir in ["src", "tests"] {
        rust_files(&root.join(dir), &mut files);
    }
    // this file spells the notation out
    files.retain(|f| !f.ends_with("parity_rules.rs"));
    let mut texts: Vec<String> = files
        .iter()
        .map(|f| std::fs::read_to_string(f).expect("a source file"))
        .collect();
    texts.push(SPEC.to_string());
    let rx = regex::Regex::new(r"\[([a-z]+(?:-[a-z]+)+)\]").expect("a regex");
    let mut tags: Vec<String> = texts
        .iter()
        .flat_map(|t| {
            rx.captures_iter(t)
                .map(|c| c[1].to_string())
                .collect::<Vec<_>>()
        })
        .collect();
    tags.sort();
    tags.dedup();
    tags
}

/// The "Known divergences" section of ACCEPTANCE.
fn divergences() -> &'static str {
    ACCEPTANCE
        .split("## Known divergences")
        .nth(1)
        .and_then(|s| s.split("\n## ").next())
        .expect("ACCEPTANCE.md has a Known divergences section")
}

#[test]
fn every_divergence_tag_the_crate_names_is_listed_in_acceptance() {
    let tags = tags_named();
    assert!(tags.len() >= 7, "{tags:?}");
    for tag in &tags {
        // a tag in a regex or a doc about the notation is not a divergence; those are not kebab-case words of the book
        assert!(
            divergences().contains(&format!("[{tag}]")),
            "[{tag}] is not under Known divergences"
        );
    }
    // and the tags the plan names, whether or not a source cites them
    for tag in gsd_divergences() {
        assert!(
            divergences().contains(&format!("[{tag}]")),
            "[{tag}] is not under Known divergences"
        );
    }
}

/// The divergences of `@gsd/security` from upstream 1.18.0 (plan 16's inventory, and the ones the build found).
pub fn gsd_divergences() -> [&'static str; 14] {
    [
        "blocking-needs-security",
        "fragment-repointed",
        "audit-advisory-every-check",
        "accept-risk-human",
        "accepted-risk-agent-creatable",
        "no-superseded-plans",
        "stride-enum",
        "retro-threat-in-plan",
        "threats-open-live",
        "security-absent-off",
        "verify-post-produces",
        "skipped-hooks-empty",
        "injection-key-explicit",
        "surface-peer-setting",
    ]
}
