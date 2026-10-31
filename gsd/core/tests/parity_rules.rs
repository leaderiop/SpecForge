//! The parity rules check (D-r44-3, D-r44-22): the code book and the gate-cutover verdicts are accounted in one place.
//!
//! `evidence.rs` names, for each of the code book's 63 codes, a test that makes it fire and a test that keeps it
//! silent; `cutover.rs` classifies each of the 171 recorded verdicts of gsd-core's `gate-cutover` fixtures. The
//! black-box half, which runs the twin projects, is `tests/cli/parity_gate_cutover.rs`.

#[path = "parity_rules/cutover.rs"]
mod cutover;
#[path = "parity_rules/evidence.rs"]
mod evidence;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use cutover::{Class, GATE_CUTOVER, SHAPES};
use evidence::{Evidence, RULE_EVIDENCE};

const CODEBOOK: &str = include_str!("golden/codebook.json");

fn core_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
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

/// `(name, body)` of every `#[test]` fn under `tests/` and `src/`. A body runs from the `fn` line to the first line
/// that is a lone `}` at the fn's own indentation.
fn test_fns() -> Vec<(String, String)> {
    let mut files = Vec::new();
    for dir in ["tests", "src"] {
        rust_files(&core_dir().join(dir), &mut files);
    }
    let mut found = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).expect("a source file");
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.trim() != "#[test]" {
                continue;
            }
            let Some(start) = (i + 1..lines.len().min(i + 6)).find(|&j| lines[j].contains("fn "))
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

fn codes() -> Vec<String> {
    let book: serde_json::Value = serde_json::from_str(CODEBOOK).expect("the code book");
    book["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|i| i["code"].as_str().expect("a code").to_string())
        .collect()
}

/// Whether some test fn called `name` mentions `code` in its name or its body.
fn names_code(fns: &[(String, String)], name: &str, code: &str) -> bool {
    let word = |text: &str| {
        text.match_indices(code).any(|(at, _)| {
            let after = text[at + code.len()..].chars().next();
            let before = text[..at].chars().next_back();
            !before.is_some_and(|c| c.is_alphanumeric())
                && !after.is_some_and(|c| c.is_alphanumeric())
        })
    };
    fns.iter().any(|(n, body)| {
        n == name && (word(body) || n.to_lowercase().contains(&code.to_lowercase()))
    })
}

#[test]
fn every_code_fires_and_stays_silent() {
    let fns = test_fns();
    let book = codes();
    assert_eq!(book.len(), 63, "the code book has 63 codes");
    let listed: Vec<&str> = RULE_EVIDENCE.iter().map(|(c, _)| *c).collect();
    assert_eq!(listed, book, "RULE_EVIDENCE lists the code book, in order");
    let mut bad = Vec::new();
    for (code, evidence) in RULE_EVIDENCE {
        match evidence {
            Evidence::Tests { fires, silent } => {
                for (what, name) in [("fires", fires), ("silent", silent)] {
                    if !names_code(&fns, name, code) {
                        bad.push(format!("{code} {what}: no test `{name}` that names {code}"));
                    }
                }
            }
            Evidence::NotBuilt(reason) => {
                assert!(reason.len() > 40, "{code}: say why: {reason}");
            }
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
    // The codes with no rule are exactly the ones the plan names.
    let unbuilt: Vec<&str> = RULE_EVIDENCE
        .iter()
        .filter(|(_, e)| matches!(e, Evidence::NotBuilt(_)))
        .map(|(c, _)| *c)
        .collect();
    assert_eq!(unbuilt, ["E515"]);
}

fn snapshot_names() -> BTreeSet<String> {
    let dir = core_dir().join("../parity/upstream/v1.18.0/gate-cutover");
    std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|e| {
            e.expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter_map(|n| n.strip_suffix(".json").map(String::from))
        .collect()
}

#[test]
fn the_cutover_table_matches_the_snapshot() {
    let snapshot = snapshot_names();
    assert_eq!(snapshot.len(), 171, "the vendored verdicts");
    let table: Vec<&str> = GATE_CUTOVER.iter().map(|(n, _)| *n).collect();
    let unique: BTreeSet<&str> = table.iter().copied().collect();
    assert_eq!(unique.len(), table.len(), "each verdict is classified once");
    let listed: BTreeSet<String> = table.iter().map(|n| n.to_string()).collect();
    assert_eq!(
        listed, snapshot,
        "the table and the snapshot name the same verdicts"
    );
    // A reproduced row names a test that exists and that reads the row's recorded verdict.
    let fns = test_fns();
    for (name, class) in GATE_CUTOVER {
        if let Class::Reproduced(test) = class {
            let quoted = format!("\"{name}\"");
            assert!(
                fns.iter()
                    .any(|(n, body)| n == test && body.contains(&quoted)),
                "{name}: no test `{test}` that reads it"
            );
        }
    }
    // Every class says something.
    for (name, class) in GATE_CUTOVER {
        let text = match class {
            Class::Reproduced(t)
            | Class::Owned(t)
            | Class::ByConstruction(t)
            | Class::NotApplicable(t)
            | Class::NotReproduced(t)
            | Class::Diverges(t) => t,
        };
        assert!(!text.is_empty(), "{name}: an empty class");
    }
}

#[test]
fn every_by_construction_row_names_its_refusal() {
    for (name, class) in GATE_CUTOVER {
        if let Class::ByConstruction(shape) = class {
            assert!(
                SHAPES.contains(&shape),
                "{name}: `{shape}` is not one of the shape classes {SHAPES:?}"
            );
        }
    }
}

/// REVIEW-r44 M9: a verdict a twin reaches and answers differently is a divergence, not an unreached verdict.
#[test]
fn the_divergences_and_the_unreached_verdicts_are_counted_apart() {
    let names = |f: fn(&Class) -> bool| -> Vec<&str> {
        GATE_CUTOVER
            .iter()
            .filter(|(_, c)| f(c))
            .map(|(n, _)| *n)
            .collect()
    };
    assert_eq!(
        names(|c| matches!(c, Class::Diverges(_))),
        [
            "prohibition-enforcement-request-file-judgment-mode",
            "tdd-review-checkpoint-mixed-violation",
            "tdd-review-checkpoint-red-only"
        ]
    );
    assert_eq!(names(|c| matches!(c, Class::NotReproduced(_))).len(), 11);
}
