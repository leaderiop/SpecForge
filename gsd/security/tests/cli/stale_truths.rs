//! A check since the judged commit says what changed under each stale truth of the vault (`gsd/I527`), and names
//! exactly the truths `re-verify --stale-only` lists. The twin's two truths share `src/vault.js` and each cites one
//! file of its own (`test/vault.test.js`, `src/cli.js`), so one edit names one truth or both, each with that file
//! alone.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::fresh_vault::{COMMAND_LINE, SEALED, judged_phase, touch};
use super::harness::Project;

const I527: &str = "gsd/I527";
const PHASE: &str = "v1.0/01";

/// The vault judged and committed, then `file` edited: each `gsd/I527` of a check since that commit, by its truth. The
/// truths it names are exactly those `re-verify --stale-only` lists, and the check finds its base (no `W021`).
fn named_after(file: &str) -> (Project, BTreeMap<String, Value>) {
    let p = judged_phase();
    p.commit();
    touch(&p, file);
    let codes: Vec<String> = p
        .checked_since("HEAD")
        .iter()
        .filter_map(|d| d["code"].as_str().map(String::from))
        .collect();
    assert!(!codes.iter().any(|c| c == "W021"), "{file}: {codes:?}");
    let advisories = p.advisories(I527, "HEAD");
    let named: BTreeSet<String> = advisories.keys().cloned().collect();
    assert_eq!(named, p.stale_truths(PHASE), "{file}");
    (p, advisories)
}

/// Asserts the advisory on `truth` is Info, names the commit and `file` as the one file it cites that changed, and
/// does not name `unchanged`, the other file the truth cites.
fn names_one_file(
    p: &Project,
    advisories: &BTreeMap<String, Value>,
    truth: &str,
    file: &str,
    unchanged: &str,
) {
    let advisory = advisories
        .get(truth)
        .unwrap_or_else(|| panic!("{I527} on {truth}: {advisories:#?}"));
    assert_eq!(advisory["severity"], "Info", "{advisory}");
    let message = advisory["message"].as_str().expect("a message");
    let base: String = p.head().chars().take(12).collect();
    assert!(message.contains(&format!("since {base}")), "{message}");
    assert!(
        message.contains(&format!("only the files it cites changed: '{file}'")),
        "{message}"
    );
    assert!(!message.contains(unchanged), "{message}");
}

/// The truths of `advisories`, sorted.
fn truths(advisories: &BTreeMap<String, Value>) -> Vec<&str> {
    advisories.keys().map(String::as_str).collect()
}

#[test]
fn an_edit_to_the_vault_names_both_truths_with_that_file() {
    let (p, advisories) = named_after("src/vault.js");
    assert_eq!(truths(&advisories), [SEALED, COMMAND_LINE], "{I527}");
    names_one_file(
        &p,
        &advisories,
        SEALED,
        "src/vault.js",
        "test/vault.test.js",
    );
    names_one_file(&p, &advisories, COMMAND_LINE, "src/vault.js", "src/cli.js");
}

#[test]
fn an_edit_to_the_command_line_names_its_truth_only() {
    let (p, advisories) = named_after("src/cli.js");
    assert_eq!(truths(&advisories), [COMMAND_LINE], "{I527}");
    names_one_file(&p, &advisories, COMMAND_LINE, "src/cli.js", "src/vault.js");
}

#[test]
fn an_edit_to_the_tests_names_the_sealed_store_truth_only() {
    let (p, advisories) = named_after("test/vault.test.js");
    assert_eq!(truths(&advisories), [SEALED], "{I527}");
    names_one_file(
        &p,
        &advisories,
        SEALED,
        "test/vault.test.js",
        "src/vault.js",
    );
}
