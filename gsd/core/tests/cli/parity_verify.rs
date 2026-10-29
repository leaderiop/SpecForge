//! The taskboard's SpecForge twin of phase 1's UAT against what gsd-tools 1.18.0 says about `01-UAT.md` (parity
//! suite): `audit-uat` and the UAT's own Summary. The goldens come from `record.sh`. The phase's verification
//! (`01-VERIFICATION.md`, `verification fingerprint`) is not compared here: its verdicts are E501 judgements, r40's.

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
    assert_eq!(audit["results"][0]["status"], "complete");
}

#[test]
fn the_uat_twin_totals_are_the_uat_summary() {
    let p = taskboard();
    // `01-UAT.md`: total 3, passed 2, issues 1, pending 0, skipped 0, blocked 0.
    assert_eq!(
        p.fields_of(SESSION)["totals"],
        serde_json::json!({ "total": 3, "passed": 2, "issues": 1, "pending": 0, "skipped": 0, "blocked": 0 })
    );
    assert_eq!(p.fields_of(&format!("{SESSION}/U2/G1"))["status"], "open");
}
