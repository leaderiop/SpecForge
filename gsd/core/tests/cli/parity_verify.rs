//! The taskboard's SpecForge twin of phase 1's UAT against what gsd-tools 1.18.0 says about `01-UAT.md` (parity
//! suite): `audit-uat`, the UAT's own Summary and frontmatter, and the phase's verification, whose differences from
//! `01-VERIFICATION.md` are named below until r40 records the E501 verdicts. The goldens come from `record.sh`.

use std::path::Path;

use serde_json::Value;

use crate::harness::taskboard;

const SESSION: &str = "v1.0/01/01-UAT";

fn golden(name: &str) -> Value {
    let full = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../parity/goldens/taskboard")
        .join(format!("{name}.json"));
    let text =
        std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("read {}: {e}", full.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{name} is JSON: {e}"))
}

#[test]
fn the_uat_twin_reports_what_audit_uat_lists() {
    let p = taskboard();
    let audit = golden("audit-uat");
    let items = audit["results"][0]["items"].as_array().expect("items");
    let issues: Vec<&Value> = items.iter().filter(|i| i["category"] == "issue").collect();
    // audit-uat lists the one test whose result is `issue`, by number and name.
    assert_eq!(issues.len(), 1);
    let test = p.fields_of(&format!("{SESSION}/U{}", issues[0]["test"]));
    assert_eq!(test["result"], "issue");
    let tests = p.nodes_of("uat_test");
    let node = tests
        .iter()
        .find(|n| n["id"].as_str().is_some_and(|i| i.ends_with("/U2")))
        .expect("U2");
    assert_eq!(issues[0]["name"], node["title"]);
    assert_eq!(issues[0]["expected"], test["expected"]);
    // ... and the gap it found, with the reporter's words as its reason.
    let gap = p.fields_of(&format!("{SESSION}/U2/G1"));
    let found = items
        .iter()
        .find(|i| i["category"] == "unknown")
        .expect("a gap item");
    assert_eq!(found["name"], gap["truth"]);
    assert_eq!(found["reason"], gap["reason"]);
    assert_eq!(audit["summary"]["by_category"]["issue"], 1);
}

const UAT_MD: &str = ".planning/phases/01-core/01-UAT.md";
const VERIFICATION_MD: &str = ".planning/phases/01-core/01-VERIFICATION.md";

/// The `key: value` lines of a markdown file's frontmatter (`---` block) or of the block under a `## <heading>`.
fn pairs(text: &str, heading: Option<&str>) -> Vec<(String, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let body: Vec<&str> = match heading {
        None => lines
            .iter()
            .skip(1)
            .take_while(|l| **l != "---")
            .copied()
            .collect(),
        Some(h) => lines
            .iter()
            .skip_while(|l| **l != h)
            .skip(1)
            .skip_while(|l| l.is_empty())
            .take_while(|l| !l.is_empty())
            .copied()
            .collect(),
    };
    body.iter()
        .filter_map(|l| l.split_once(": "))
        .map(|(k, v)| (k.to_string(), v.trim().to_string()))
        .collect()
}

fn get(pairs: &[(String, String)], key: &str) -> String {
    pairs
        .iter()
        .find(|(k, _)| k == key)
        .unwrap_or_else(|| panic!("{key} is in the file"))
        .1
        .clone()
}

#[test]
fn the_uat_twin_totals_are_the_uat_summary() {
    let p = taskboard();
    // The totals are the `## Summary` block of `01-UAT.md`, parsed: total, passed, issues, pending, skipped, blocked.
    let summary = pairs(&p.read(UAT_MD), Some("## Summary"));
    let expected: serde_json::Map<String, Value> =
        ["total", "passed", "issues", "pending", "skipped", "blocked"]
            .iter()
            .map(|k| {
                (
                    k.to_string(),
                    serde_json::json!(get(&summary, k).parse::<u64>().expect("a count")),
                )
            })
            .collect();
    assert_eq!(p.fields_of(SESSION)["totals"], Value::Object(expected));
    assert_eq!(p.fields_of(&format!("{SESSION}/U2/G1"))["status"], "open");
}

/// A named divergence. `01-UAT.md` and the golden say `complete`, which the file's writer set when the run finished
/// ("testing complete"). The twin's status is computed (`uat-predicate.cts:106-129`): its one issue has a gap that is
/// still open and has no root cause, so the session is `partial`. It becomes `complete` when a gap-closure plan
/// resolves the gap; the twin does not model one.
#[test]
fn the_uat_twins_status_is_partial_where_the_file_says_complete() {
    let p = taskboard();
    let frontmatter = pairs(&p.read(UAT_MD), None);
    assert_eq!(get(&frontmatter, "status"), "complete");
    assert_eq!(golden("audit-uat")["results"][0]["status"], "complete");
    assert_eq!(p.fields_of(SESSION)["status"], "partial");
}

/// A named divergence, until r40 records the E501 verdicts: `01-VERIFICATION.md` is `passed`, `2/2 must-haves
/// verified`, and the twin's verification is `pending` with no verdict on its three truths (the twin also models more
/// truths than the file's two must-haves). r40 turns this into an equality.
#[test]
fn the_twins_verification_is_pending_until_r40() {
    let p = taskboard();
    let file = pairs(&p.read(VERIFICATION_MD), None);
    assert_eq!(get(&file, "status"), "passed");
    assert!(get(&file, "score").starts_with("2/2"), "{file:?}");
    let twin = p.fields_of("v1.0/01/01-VERIFICATION");
    assert_eq!(twin["status"], "pending", "{twin}");
    assert_eq!(twin["score"], "0/3", "{twin}");
}
