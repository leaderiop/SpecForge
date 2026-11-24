//! `specforge gsd import` on the taskboard fixture through the binary (ADR 0391 D2, D5, D8, D10): what it writes,
//! the host's `--dry-run` and `--format json`, the refused second import, and the same bytes from the same folder.

use std::collections::BTreeMap;

use crate::harness::{Project, taskboard};
use crate::{imported, tree};

/// The project paths whose bytes differ between `before` and `after`, or that only one of them holds.
fn changed(before: &BTreeMap<String, Vec<u8>>, after: &BTreeMap<String, Vec<u8>>) -> Vec<String> {
    let mut paths: Vec<String> = before
        .keys()
        .chain(after.keys())
        .filter(|path| before.get(*path) != after.get(*path))
        .cloned()
        .collect();
    paths.sort();
    paths.dedup();
    paths
}

/// A path the import may write: under `spec/gsd/`, or the host's harness ledger with the `.gitattributes` line that
/// merges it.
fn importable(path: &str) -> bool {
    path.starts_with("spec/gsd/") || matches!(path, "specforge-harness" | ".gitattributes")
}

/// `spec/gsd/**` of `project`, path by path.
fn spec_gsd(project: &Project) -> BTreeMap<String, Vec<u8>> {
    tree(project)
        .into_iter()
        .filter(|(path, _)| path.starts_with("spec/gsd/"))
        .collect()
}

#[test]
fn import_writes_spec_gsd_and_the_report() {
    let p = imported(taskboard());
    let before = tree(&p);
    let run = p.run(&["gsd", "import"]);
    assert_eq!(run.code, 0, "import: {}", run.all());
    let written = changed(&before, &tree(&p));
    assert!(
        written.iter().any(|path| path.starts_with("spec/gsd/")),
        "nothing under spec/gsd/: {written:?}"
    );
    assert!(
        written.iter().all(|path| importable(path)),
        "a write outside spec/gsd/ and the ledger: {written:?}"
    );
    let ledger = p.read("specforge-harness");
    let results: Vec<&str> = ledger
        .lines()
        .filter(|line| line.split(' ').nth(1) == Some("result"))
        .collect();
    assert_eq!(results.len(), 3, "the three uat_test results: {ledger}");
    assert!(
        ledger
            .lines()
            .skip(1)
            .all(|line| line.contains("/01-UAT/U")),
        "only the UAT results are harness values: {ledger}"
    );
    assert!(
        run.stdout.contains(".planning/STATE.md"),
        "no entry for STATE.md: {}",
        run.stdout
    );
    assert!(
        run.stdout.lines().any(|l| l.starts_with("import: ")),
        "no count line: {}",
        run.stdout
    );
    let spec_files = written
        .iter()
        .filter(|p| p.starts_with("spec/gsd/"))
        .count();
    let diffed = run
        .stdout
        .lines()
        .filter(|l| l.starts_with("--- spec/gsd/"))
        .count();
    assert_eq!(
        diffed, spec_files,
        "one diff per written file: {}",
        run.stdout
    );
    assert_eq!(p.nodes_of("requirement").len(), 8);
}

#[test]
fn a_dry_run_writes_nothing() {
    let p = imported(taskboard());
    let before = tree(&p);
    let run = p.run(&["gsd", "import", "--dry-run"]);
    assert_eq!(run.code, 0, "dry run: {}", run.all());
    assert!(
        run.stdout.lines().any(|l| l.starts_with("import: ")),
        "no report: {}",
        run.stdout
    );
    assert_eq!(changed(&before, &tree(&p)), Vec::<String>::new());
}

#[test]
fn json_carries_the_report() {
    let p = imported(taskboard());
    let run = p.run(&["gsd", "import", "--format", "json"]);
    assert_eq!(run.code, 0, "json: {}", run.all());
    let answer: serde_json::Value = serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("the answer is JSON ({e}): {}", run.all()));
    assert_eq!(answer["output"]["format"], 1, "{answer}");
    assert!(
        answer["output"]["entries"]
            .as_array()
            .is_some_and(|entries| !entries.is_empty()),
        "no entries: {answer}"
    );
    let files = answer["files_written"].as_array().expect("files_written");
    assert!(!files.is_empty(), "{answer}");
    for file in files {
        let path = file.as_str().expect("a path");
        assert!(importable(path), "{path} is written: {answer}");
    }
}

#[test]
fn a_second_import_is_refused() {
    let p = imported(taskboard());
    let first = p.run(&["gsd", "import"]);
    assert_eq!(first.code, 0, "first import: {}", first.all());
    let before = tree(&p);
    let second = p.run(&["gsd", "import"]);
    assert_eq!(second.code, 1, "second import: {}", second.all());
    let all = second.all();
    let named = all
        .split("such as ")
        .nth(1)
        .unwrap_or_else(|| panic!("no entity named: {all}"));
    let words: Vec<&str> = named.split_whitespace().collect();
    assert!(
        words.len() >= 4 && words[2] == "in" && words[3].starts_with("spec/gsd/"),
        "kind, path and file: {all}"
    );
    assert_eq!(changed(&before, &tree(&p)), Vec::<String>::new());
}

#[test]
fn two_imports_write_the_same_bytes() {
    let (a, b) = (imported(taskboard()), imported(taskboard()));
    let (run_a, run_b) = (a.run(&["gsd", "import"]), b.run(&["gsd", "import"]));
    assert_eq!(run_a.code, 0, "{}", run_a.all());
    assert_eq!(run_b.code, 0, "{}", run_b.all());
    assert_eq!(run_a.stdout, run_b.stdout);
    let (spec_a, spec_b) = (spec_gsd(&a), spec_gsd(&b));
    assert!(!spec_a.is_empty());
    assert_eq!(
        spec_a.keys().collect::<Vec<_>>(),
        spec_b.keys().collect::<Vec<_>>()
    );
    for (path, bytes) in &spec_a {
        assert!(spec_b[path] == *bytes, "{path} differs");
    }
}
