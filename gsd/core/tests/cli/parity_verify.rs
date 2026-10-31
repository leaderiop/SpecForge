//! The taskboard's SpecForge twin of phase 1's UAT against what gsd-tools 1.19.0 says about `01-UAT.md` (parity
//! suite): `audit-uat`, the UAT's own Summary and frontmatter, and the phase's verification, whose differences from
//! `01-VERIFICATION.md` are named below. The goldens come from `record.sh`.

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

/// `01-VERIFICATION.md` is `passed`, `3/3 must-haves verified` (D-r44-21: the verifier merges the roadmap's success
/// criteria with the plans' must-haves). The twin carries its verifier's verdicts, so it says `passed` and `3/3` with
/// nothing recorded, and the file's three rows pair with the twin's three truths.
#[test]
fn verification_is_three_of_three_on_both_sides() {
    let p = taskboard();
    let file = pairs(&p.read(VERIFICATION_MD), None);
    assert_eq!(get(&file, "status"), "passed");
    assert!(get(&file, "score").starts_with("3/3"), "{file:?}");
    let twin = p.fields_of("v1.0/01/01-VERIFICATION");
    assert_eq!(twin["status"], get(&file, "status"), "{twin}");
    assert_eq!(twin["score"], "3/3", "{twin}");
    // The file's rows, each paired with the twin truth that says it.
    let rows: Vec<String> = p
        .read(VERIFICATION_MD)
        .lines()
        .filter(|l| l.starts_with("| ") && l.contains("VERIFIED"))
        .map(|l| l.split('|').nth(2).unwrap_or_default().trim().to_string())
        .collect();
    assert_eq!(rows.len(), 3, "{rows:?}");
    let truths = p.nodes_of("truth");
    let phase_one: Vec<String> = truths
        .iter()
        .filter(|n| n["id"].as_str().is_some_and(|i| i.starts_with("v1.0/01/")))
        .map(|n| {
            n["fields"]["text"]
                .as_str()
                .unwrap_or_default()
                .to_lowercase()
        })
        .collect();
    assert_eq!(phase_one.len(), 3, "{phase_one:?}");
    let pairing = [
        ("add a task with a title", "next integer id"),
        ("mark a task complete by its id", "completed by its id"),
        ("does not exist fails", "does not exist fails"),
    ];
    for (row, (said, twin_says)) in rows.iter().zip(pairing) {
        assert!(row.contains(said), "{row} says {said}");
        assert!(
            phase_one.iter().any(|t| t.contains(twin_says)),
            "{twin_says} in {phase_one:?}"
        );
    }
    // Stripped of its verdicts the verification is pending.
    let q = taskboard();
    q.strip_verdicts();
    let before = q.fields_of("v1.0/01/01-VERIFICATION");
    assert_eq!(
        (before["status"].clone(), before["score"].clone()),
        ("pending".into(), "0/3".into())
    );
}

/// P7 (D-r44-21): the verifier's seven verdicts on phase 01 are committed into the twin's `spec/` and its harness
/// ledger, and they are fresh in any copy of it. `GSD_BLESS=1` records them on a copy and writes the files back.
#[test]
fn the_twin_holds_its_verifier_verdicts() {
    if std::env::var_os("GSD_BLESS").is_some() {
        let p = taskboard();
        let recorded = p.verify_phase("v1.0/01");
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../parity/fixtures/taskboard");
        let mut files: Vec<String> = std::fs::read_dir(fixture.join("spec"))
            .expect("the spec dir")
            .map(|e| {
                format!(
                    "spec/{}",
                    e.expect("an entry").file_name().to_string_lossy()
                )
            })
            .collect();
        files.push("specforge-harness".to_string());
        for file in files {
            std::fs::write(fixture.join(&file), p.read(&file)).expect("the fixture file");
        }
        eprintln!("recorded {recorded} verdicts into the twin");
    }
    let fresh = taskboard();
    assert_eq!(
        fresh.verify_phase("v1.0/01"),
        0,
        "no E501 judgement is pending in phase 01: GSD_BLESS=1 records them"
    );
}
