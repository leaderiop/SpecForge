use std::collections::BTreeSet;
use std::path::Path;

use super::{PLANNING_FILES, SHOWN, planning_names};
use crate::markdown::{inline_code, tables};

fn readme() -> String {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../core/spec/README.md");
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The rows `text` names, `.planning` left out.
fn rows_named(text: &str) -> Vec<&'static str> {
    planning_names(text)
        .into_iter()
        .map(|(_, row)| row)
        .filter(|row| *row != ".planning")
        .collect()
}

#[test]
fn a_name_matches_its_row() {
    for (text, row) in [
        ("03-CONTEXT.md", "NN-CONTEXT.md"),
        ("{phase}-{plan}-SUMMARY.md", "NN-MM-SUMMARY.md"),
        ("${PADDED_PHASE}-UAT.md", "NN-UAT.md"),
        ("*-PLAN.md", "NN-MM-PLAN.md"),
        ("v1.0-ROADMAP.md", "ROADMAP.md"),
        (
            "v{version}-MILESTONE-AUDIT.md",
            "v<version>-MILESTONE-AUDIT.md",
        ),
        (".continue-here*.md", "*continue-here*.md"),
        ("quick/260101-abc/PLAN.md", "quick/*"),
        ("03-UI-SPEC.md", "NN-UI-SPEC.md"),
        ("onboarding/SUMMARY.md", "onboarding/SUMMARY.md"),
        ("Write `STATE.md`.", "STATE.md"),
    ] {
        assert_eq!(rows_named(text), [row], "{text}");
    }
    let found = planning_names(".planning/");
    assert_eq!(found, [(0..9, ".planning")]);
    for text in [
        "ULTRAPLAN.md",
        "config.json",
        "README.md",
        "gsd-core/templates/UAT.md",
        "https://github.com/open-gsd/gsd-core/blob/v1.19.0/CONTEXT.md",
        "docs/research/notes.md",
        "STATE.md.bak",
    ] {
        assert!(planning_names(text).is_empty(), "{text}");
    }
}

/// The names of the README's table, in order, with their kinds: a backticked file name of the first column (a code
/// span holding a `.`, a `/` or a `*`), `config.json` left out.
fn readme_rows() -> Vec<(String, String)> {
    let text = readme();
    let table = tables(&text)
        .into_iter()
        .find(|t| t.heading == "Every GSD file maps to a kind")
        .expect("the README's file table");
    assert_eq!(table.header, ["GSD file", "Kinds", "Extension"]);
    table
        .rows
        .iter()
        .flat_map(|row| {
            inline_code(&row[0])
                .into_iter()
                .map(|(_, name)| name)
                .filter(|name| name.contains(['.', '/', '*']) && name != "config.json")
                .map(|name| (name, row[1].clone()))
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn every_readme_name_is_a_row_with_its_kinds() {
    let readme = readme_rows();
    assert!(readme.len() >= 40, "{} names", readme.len());
    let ours: Vec<(String, String)> = PLANNING_FILES[..readme.len()]
        .iter()
        .map(|(name, kinds)| (name.to_string(), kinds.to_string()))
        .collect();
    assert_eq!(ours, readme);
}

/// Every `.planning/<path>` literal of the vendored release's prompts: `gsd-core`, `commands` and `agents`.
fn snapshot_literals() -> BTreeSet<String> {
    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
        for entry in entries {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else {
                out.push(path);
            }
        }
    }
    let mut files = Vec::new();
    for dir in ["gsd-core", "commands", "agents"] {
        walk(&Path::new(crate::snapshot!()).join(dir), &mut files);
    }
    let name = |c: char| c.is_ascii_alphanumeric() || "_{}$*<>.-/".contains(c);
    let mut out = BTreeSet::new();
    for file in files {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        for (at, _) in text.match_indices(".planning/") {
            let run: String = text[at..].chars().take_while(|c| name(*c)).collect();
            let run = run.trim_end_matches('.');
            if run != ".planning/" {
                out.insert(run.to_string());
            }
        }
    }
    assert!(out.len() >= 150, "{} literals", out.len());
    out
}

/// A last segment upstream's text keeps under `.planning/` that names no file GSD keeps state in, and why.
const NOT_STATE: &[(&str, &str)] = &[
    ("*.md", "every Markdown file of a folder, not one file"),
    ("*", "every entry of a folder"),
    ("*-*", "a glob of phase directories"),
    ("config.json", "settings, not a state-file pattern"),
    ("phases", "the folder of phase directories, not a file"),
    ("XX-name", "a placeholder phase directory"),
    ("XX-current", "a placeholder phase directory"),
    ("03-auth", "an example phase directory"),
    ("999*", "a glob of backlog phase directories"),
    ("My", "a path cut at its space, in a comment about that cut"),
    ("{spikes", "a brace list cut at its comma"),
    ("ROADMAP.md_", "a pattern fragment in a comment"),
    (
        "STATE.md.bak",
        "a name the PR filter must not take for STATE.md",
    ),
    (
        "STATEX.md",
        "a name the PR filter must not take for STATE.md",
    ),
];

/// The row each literal matches, whole or by its last segment.
fn snapshot_rows() -> BTreeSet<&'static str> {
    let mut used = BTreeSet::new();
    for literal in snapshot_literals() {
        used.extend(rows_named(&literal));
        if let Some(last) = literal.split('/').rev().find(|s| !s.is_empty()) {
            used.extend(rows_named(last));
        }
    }
    used
}

#[test]
fn every_gsd_name_the_snapshot_uses_matches_a_row() {
    let mut unmatched = Vec::new();
    let mut not_state = BTreeSet::new();
    for literal in snapshot_literals() {
        if !rows_named(&literal).is_empty() {
            continue;
        }
        let last = literal
            .split('/')
            .rev()
            .find(|s| !s.is_empty())
            .unwrap_or_default();
        match NOT_STATE.iter().find(|(name, _)| *name == last) {
            Some((name, _)) => {
                not_state.insert(*name);
            }
            None => unmatched.push(literal),
        }
    }
    assert!(
        unmatched.is_empty(),
        "names under .planning/ that match no PLANNING_FILES row:\n{}",
        unmatched.join("\n")
    );
    let stale: Vec<&str> = NOT_STATE
        .iter()
        .map(|(name, _)| *name)
        .filter(|name| !not_state.contains(name))
        .collect();
    assert!(
        stale.is_empty(),
        "NOT_STATE entries no literal needs: {stale:?}"
    );
}

#[test]
fn every_row_is_the_readmes_or_the_snapshots() {
    let text = readme();
    let from = text
        .find("Written nowhere, on purpose")
        .expect("the README's written-nowhere list");
    let to = from
        + text[from..]
            .find("\n\nNot modelled")
            .expect("the paragraph after it");
    let shown: BTreeSet<String> = inline_code(&text[from..to])
        .into_iter()
        .map(|(_, name)| name)
        .collect();
    let used = snapshot_rows();
    let readme = readme_rows().len();
    let mut invented = Vec::new();
    for (name, kinds) in &PLANNING_FILES[readme..] {
        if shown.contains(*name) {
            assert_eq!(*kinds, SHOWN, "{name} is written nowhere");
        } else if !used.contains(name) {
            invented.push(*name);
        }
    }
    assert!(
        invented.is_empty(),
        "rows neither the README nor upstream's `.planning/` names give: {invented:?}"
    );
    let mut names = BTreeSet::new();
    for (name, _) in PLANNING_FILES {
        assert!(names.insert(*name), "{name} is a row twice");
    }
}
