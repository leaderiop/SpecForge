//! `NN-VERIFICATION.md` and `NN-UAT.md` through the command in process, against the taskboard twin's `verify.spec`
//! (its verdict blocks excluded: verdicts are never imported, ADR 0391 D5).
//!
//! Known differences from the twin: the session's and the verification's titles are the import's (the file stem, the
//! document's title); the twin's are hand-made.

mod common;

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use common::{
    creates, entries, fact, feed, find, fixture, has, import, import_fixture, paths, planning_facts,
};

const VERIFICATION: &str = ".planning/phases/01-core/01-VERIFICATION.md";
const UAT: &str = ".planning/phases/01-core/01-UAT.md";

#[test]
fn taskboard_verification_is_its_phase_and_nothing_the_verifier_recorded() {
    let out = import_fixture("taskboard");
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    assert_eq!(paths(&all, "verification"), ["v1.0/01/01-VERIFICATION"]);
    let v = find(&all, "verification", "v1.0/01/01-VERIFICATION");
    assert_eq!(v.fields, json!({ "phase": "v1.0/01" }));
    assert_eq!(v.title, "Phase 1: Core Verification Report");
    assert_eq!(v.file, "spec/gsd/phases/01.spec");
    let reported = entries(&out, VERIFICATION);
    for (line, outcome, says) in [
        (2, "redundant", "the file name gives the phase"),
        (3, "computed", "verification.verified is computed"),
        (4, "computed", "verification.status is computed"),
        (5, "computed", "verification.score is computed"),
        // A block value is located at its first item.
        (7, "computed", "verification.covered_files is computed"),
        (15, "computed", "verification.covered_digest is computed"),
        (16, "unmapped", "no field of a verification holds it"),
        (19, "mapped", "verification v1.0/01/01-VERIFICATION"),
        (21, "redundant", "ROADMAP.md gives the phase goal"),
        (22, "computed", "verification.verified is computed"),
        (23, "computed", "verification.status is computed"),
        (25, "ignored", "a heading over the verifier's tables"),
        (27, "unmapped", "verdicts, never imported (ADR 0391 D5)"),
        (37, "unmapped", "verdicts, never imported (ADR 0391 D5)"),
        (46, "redundant", "requirements-completed give it"),
        (53, "ignored", "an empty placeholder"),
        (58, "unmapped", "prose"),
        (59, "unmapped", "prose"),
    ] {
        assert!(
            has(&reported, line, outcome, says),
            "{line} {says}: {reported:#?}"
        );
    }
}

#[test]
fn taskboard_uat_matches_the_twin() {
    let out = import_fixture("taskboard");
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    let session = find(&all, "uat_session", "v1.0/01/01-UAT");
    assert_eq!(
        session.fields,
        json!({
            "phase": "v1.0/01",
            "source": ["v1.0/01/01-01/01-01-SUMMARY", "v1.0/01/01-02/01-02-SUMMARY"],
            "started": "2026-03-03T10:30:00Z",
        })
    );
    assert_eq!(
        paths(&all, "uat_test"),
        [
            "v1.0/01/01-UAT/U1",
            "v1.0/01/01-UAT/U2",
            "v1.0/01/01-UAT/U3"
        ]
    );
    for (id, title, fields) in [
        (
            "U1",
            "Add a task",
            json!({
                "expected": "`node src/cli.js add \"Write the roadmap\"` prints `added 1: Write the roadmap`",
                "result": "pass",
            }),
        ),
        (
            "U2",
            "Complete a task that does not exist",
            json!({
                "expected": "`node src/cli.js done 7` fails and names the id 7",
                "result": "issue",
                "reported": "it prints a stack trace before the message, which is noisy",
                "severity": "cosmetic",
            }),
        ),
        (
            "U3",
            "Usage text",
            json!({
                "expected": "`node src/cli.js` with no command prints the usage line and exits 2",
                "result": "pass",
            }),
        ),
    ] {
        let test = find(&all, "uat_test", &format!("v1.0/01/01-UAT/{id}"));
        let mut want = fields;
        want["session"] = json!("v1.0/01/01-UAT");
        assert_eq!(test.fields, want, "{id}");
        assert_eq!(test.title, title);
    }
    let gaps: Vec<_> = all.iter().filter(|c| c.kind == "uat_gap").collect();
    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0].path, "v1.0/01/01-UAT/U2/G1");
    assert_eq!(
        gaps[0].fields,
        json!({
            "test": "v1.0/01/01-UAT/U2",
            "truth": "Completing an unknown id fails with a one-line message",
            "reason": "User reported: it prints a stack trace before the message, which is noisy",
            "severity": "cosmetic",
        })
    );
    let reported = entries(&out, UAT);
    for (line, outcome, says) in [
        (2, "computed", "uat_session.status is computed"),
        (3, "redundant", "the file name gives the phase"),
        (6, "unmapped", "the session's last edit"),
        (9, "unmapped", "a runtime state"),
        (13, "ignored", "a heading over the tests"),
        (15, "mapped", "uat_test v1.0/01/01-UAT/U1"),
        (29, "computed", "uat_session.totals is computed"),
        (38, "mapped", "each item is a uat_gap"),
        (40, "mapped", "uat_gap v1.0/01/01-UAT/U2/G1"),
        (41, "computed", "uat_gap.status is computed"),
    ] {
        assert!(
            has(&reported, line, outcome, says),
            "{line} {says}: {reported:#?}"
        );
    }
}

/// The taskboard imported with its `01-UAT.md` replaced by `uat`.
fn with_uat(uat: &str) -> CommandOutput {
    let facts: Vec<Value> = planning_facts(&fixture("taskboard"))
        .into_iter()
        .map(|f| if f["path"] == UAT { fact(UAT, uat) } else { f })
        .collect();
    let manifest =
        std::fs::read_to_string(fixture("taskboard").join("specforge.json")).expect("a manifest");
    import(
        &CommandGraph::default(),
        &feed(facts, false, Some(&manifest)),
    )
}

#[test]
fn a_hostile_uat_reports_what_it_cannot_write() {
    let uat = "---\nphase: 01-core\nsource: [01-01-SUMMARY.md, 07-01-SUMMARY.md]\nstarted: yesterday\n---\n\n\
        ## Tests\n\n\
        ### 1. Odd result\nexpected: |\n  two\n  lines\nresult: maybe\nseverity: cosmetic\n\n\
        ### Not a test\nexpected: x\n\n\
        ## Gaps\n\n\
        - truth: \"A gap of no test\"\n  test: 9\n- truth: \"A gap of test 1\"\n  test: 1\n  root_cause: \"\"\n  \
        artifacts: []\n  debug_session: \"dbg-1\"\n\n## Notes\n";
    let out = with_uat(uat);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    assert_eq!(
        find(&all, "uat_session", "v1.0/01/01-UAT").fields,
        json!({ "phase": "v1.0/01", "source": ["v1.0/01/01-01/01-01-SUMMARY"] })
    );
    // The test keeps every field but its result.
    assert_eq!(
        find(&all, "uat_test", "v1.0/01/01-UAT/U1").fields,
        json!({ "session": "v1.0/01/01-UAT", "expected": "two\nlines", "severity": "cosmetic" })
    );
    assert_eq!(paths(&all, "uat_test"), ["v1.0/01/01-UAT/U1"]);
    assert_eq!(paths(&all, "uat_gap"), ["v1.0/01/01-UAT/U1/G2"]);
    assert_eq!(
        find(&all, "uat_gap", "v1.0/01/01-UAT/U1/G2").fields,
        json!({ "test": "v1.0/01/01-UAT/U1", "truth": "A gap of test 1" })
    );
    let reported = entries(&out, UAT);
    for (line, outcome, says) in [
        (
            3,
            "unmapped",
            "`07-01-SUMMARY.md` names no NN-MM-SUMMARY.md of a roadmap phase",
        ),
        (4, "unmapped", "not an RFC 3339 timestamp"),
        (
            13,
            "unmapped",
            "`result: maybe` is none of pending, pass, issue, skipped, blocked",
        ),
        (16, "unmapped", "a test heading is `### N. Title`"),
        (
            22,
            "unmapped",
            "the gap names no test of this file (`test: 9`)",
        ),
        (27, "unmapped", "`uat_gap.debug`"),
        (
            29,
            "unmapped",
            "a section no row of the NN-UAT.md table names",
        ),
    ] {
        assert!(
            has(&reported, line, outcome, says),
            "{line} {says}: {reported:#?}"
        );
    }
}

/// GSD's diagnosed gap (gsd-core templates/UAT.md): `artifacts` as `path`/`issue` maps.
#[test]
fn a_diagnosed_gap_keeps_its_artifacts() {
    let uat = "---\nphase: 01-core\nsource: [01-01-SUMMARY.md]\n---\n\n## Tests\n\n### 1. One\nexpected: e\n\n\
        ## Gaps\n\n\
        - truth: \"A gap\"\n  test: 1\n  artifacts:\n    - path: \"src/a.js\"\n      issue: \"never saves\"\n    \
        - \"src/b.js\"\n    - path: \"src/c.js\"\n      note: \"n\"\n    - note: \"only\"\n  missing:\n    - \"add w\"\n    \
        - [x]\n\
        - truth: \"Another\"\n  test: 1\n  artifacts: src/x.js\n  missing: {a: b}\n  reason: [x]\n";
    let out = with_uat(uat);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    assert_eq!(
        find(&all, "uat_gap", "v1.0/01/01-UAT/U1/G1").fields,
        json!({
            "test": "v1.0/01/01-UAT/U1",
            "truth": "A gap",
            "artifacts": [
                {"path": "src/a.js", "issue": "never saves"},
                {"path": "src/b.js"},
                {"path": "src/c.js"}
            ],
            "missing": ["add w"]
        })
    );
    assert_eq!(
        find(&all, "uat_gap", "v1.0/01/01-UAT/U1/G2").fields,
        json!({ "test": "v1.0/01/01-UAT/U1", "truth": "Another" })
    );
    let reported = entries(&out, UAT);
    for (line, says) in [
        (
            20,
            "an artifact member `note` no row of the ## Gaps table names",
        ),
        (21, "an artifact member `note` no row"),
        (21, "an artifact with neither a path nor an issue"),
        (24, "a `missing` item that is not one text"),
        (27, "`artifacts` is not a list"),
        (28, "`missing` is not a list"),
        (29, "`reason` is not one text"),
    ] {
        assert!(
            has(&reported, line, "unmapped", says),
            "{line} {says}: {reported:#?}"
        );
    }
}

#[test]
fn a_uat_testing_no_imported_summary_is_no_session() {
    let uat = "---\nphase: 01-core\nsource: [07-01-SUMMARY.md]\n---\n\n## Tests\n\n### 1. One\nexpected: e\n";
    let out = with_uat(uat);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    assert!(paths(&all, "uat_session").is_empty());
    assert!(paths(&all, "uat_test").is_empty());
    let reported = entries(&out, UAT);
    for (line, says) in [
        (
            1,
            "uat_session v1.0/01/01-UAT: no item of its source is imported",
        ),
        (
            8,
            "uat_test v1.0/01/01-UAT/U1: parent not imported (v1.0/01/01-UAT)",
        ),
    ] {
        assert!(
            has(&reported, line, "unmapped", says),
            "{line} {says}: {reported:#?}"
        );
    }
}

#[test]
fn a_uat_of_two_hundred_tests_imports_each() {
    let tests: String = (1..=200)
        .map(|n| format!("### {n}. Test {n}\nexpected: e{n}\nresult: pass\n\n"))
        .collect();
    let uat =
        format!("---\nphase: 01-core\nsource: [01-01-SUMMARY.md]\n---\n\n## Tests\n\n{tests}");
    let out = with_uat(&uat);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    assert_eq!(paths(&all, "uat_test").len(), 200);
    assert_eq!(
        find(&all, "uat_test", "v1.0/01/01-UAT/U200").fields,
        json!({ "session": "v1.0/01/01-UAT", "expected": "e200", "result": "pass" })
    );
}
