//! Judged rules, the review of K7 (ADR 0290): stale verdicts that counted, and reads outside the project.

use serde_json::json;
use specforge_project::judged::Cited;
use specforge_project::testing::{TestProject, gsd_judged_project};
use specforge_registry::PlainDisk;
use specforge_test::prelude::*;

use crate::judged::{PLAN, W311, compile, digest_of, judgement, record, record_citing};

const TRUTH: &str = "v1.0/03/03-02/TR1";
const E501: &str = "gsd/E501";

fn report(tests: &[(&str, &str)]) -> String {
    let tests: Vec<_> = tests
        .iter()
        .map(|(name, status)| json!({"name": name, "status": status, "duration_ms": 1}))
        .collect();
    json!({"results": {TRUTH: {"tests": tests}}}).to_string()
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a verdict recorded without a report is stale once a report exists"
)]
fn a_recorded_none_goes_stale_once_a_report_exists() {
    let project = gsd_judged_project("");
    record(&project, TRUTH, E501, "passed");
    assert_eq!(judgement(&compile(&project), TRUTH, E501).0, "fresh");
    project.write("specforge-report.json", report(&[("survives", "passed")]));
    let (state, changed) = judgement(&compile(&project), TRUTH, E501);
    assert_eq!(
        (state.as_str(), changed.as_slice()),
        ("stale", ["tests".to_string()].as_slice())
    );
}

#[specforge_test(
    behavior = "digest_judged_inputs",
    verify = "a missing report hides no change to a cited entity or file"
)]
fn a_missing_report_hides_no_change_to_what_a_verdict_cites() {
    let project = gsd_judged_project("").file("docs/design.md", "one\n");
    let cited = Cited {
        entities: vec!["D-00".into()],
        files: vec!["docs/design.md".into()],
        tests: vec!["survives".into()],
    };
    let cites = "    cited [D-00, \"docs/design.md\"]\n    cited_tests [\"survives\"]\n";
    record_citing(&project, TRUTH, E501, "passed", &cited, cites);
    assert_eq!(judgement(&compile(&project), TRUTH, E501).0, "fresh");
    project.write("docs/design.md", "two two\n");
    let (state, changed) = judgement(&compile(&project), TRUTH, E501);
    assert_eq!(
        (state.as_str(), changed.as_slice()),
        ("stale", ["cited".to_string()].as_slice())
    );
    // A cited entity that changes is as visible.
    project.write("docs/design.md", "one\n");
    assert_eq!(judgement(&compile(&project), TRUTH, E501).0, "fresh");
    project.write(
        "a.spec",
        project
            .read("a.spec")
            .replace("A project decision", "Another decision"),
    );
    assert_eq!(judgement(&compile(&project), TRUTH, E501).1, ["cited"]);
}

#[specforge_test(
    behavior = "digest_judged_inputs",
    verify = "cited tests are an input of their own, judged once a report exists"
)]
fn cited_tests_are_judged_once_a_report_exists() {
    let project = gsd_judged_project("");
    let cited = Cited {
        tests: vec!["survives".into()],
        ..Cited::default()
    };
    let cites = "    cited_tests [\"survives\"]\n";
    record_citing(&project, TRUTH, E501, "passed", &cited, cites);
    project.write("specforge-report.json", report(&[("survives", "passed")]));
    let changed = judgement(&compile(&project), TRUTH, E501).1;
    assert!(changed.contains(&"cited_tests".to_string()), "{changed:?}");
}

#[specforge_test(
    behavior = "digest_judged_inputs",
    verify = "the tests of a report digest sorted, whatever order they ran in"
)]
fn a_report_that_reorders_its_tests_stales_nothing() {
    let project = gsd_judged_project("");
    project.write(
        "specforge-report.json",
        report(&[("a", "passed"), ("b", "passed")]),
    );
    record(&project, TRUTH, E501, "passed");
    project.write(
        "specforge-report.json",
        report(&[("b", "passed"), ("a", "passed")]),
    );
    assert_eq!(judgement(&compile(&project), TRUTH, E501).0, "fresh");
}

const ELSEWHERE: &str =
    "\nphase 04 \"Four\" {\n  milestone v1.0\n}\n\ndecision D-09 \"Elsewhere\" {\n  phase 04\n}\n";

#[specforge_test(
    behavior = "digest_judged_inputs",
    verify = "a cited entity is digested by the resolution that accepted it"
)]
fn a_cited_entity_outside_the_holders_chain_is_still_watched() {
    let project = gsd_judged_project(ELSEWHERE);
    let cited = Cited {
        entities: vec!["D-09".into()],
        ..Cited::default()
    };
    record_citing(&project, PLAN, W311, "passed", &cited, "    cited [D-09]\n");
    assert_eq!(judgement(&compile(&project), PLAN, W311).0, "fresh");
    project.write(
        "a.spec",
        project.read("a.spec").replace("Elsewhere", "Somewhere"),
    );
    let (state, changed) = judgement(&compile(&project), PLAN, W311);
    assert_eq!(
        (state.as_str(), changed.as_slice()),
        ("stale", ["cited".to_string()].as_slice())
    );
}

#[specforge_test(
    behavior = "digest_judged_inputs",
    verify = "a spec source that declares nothing digests as its bytes"
)]
fn a_spec_file_declaring_nothing_digests_as_its_bytes() {
    let project = gsd_judged_project("").file("empty.spec", "// one\n");
    project.write(
        "a.spec",
        project
            .read("a.spec")
            .replace("[\"src/auth/session.ts\"]", "[\"empty.spec\"]"),
    );
    record(&project, TRUTH, E501, "passed");
    assert_eq!(judgement(&compile(&project), TRUTH, E501).0, "fresh");
    project.write("empty.spec", "// two two\n");
    assert_eq!(judgement(&compile(&project), TRUTH, E501).0, "stale");
}

#[specforge_test(
    behavior = "digest_judged_inputs",
    verify = "a cited file outside the project is not read"
)]
fn a_cited_file_outside_the_project_is_not_read() {
    let project = gsd_judged_project("");
    let other = TestProject::new();
    let outside = other.root().join("specforge.json");
    let climbing = format!(
        "../{}/specforge.json",
        other.root().file_name().unwrap().to_string_lossy()
    );
    let compiled = compile(&project);
    let judged = compiled
        .environment()
        .registries()
        .judged_rule(W311)
        .expect("a rule");
    for file in [outside.to_string_lossy().to_string(), climbing] {
        let cited = Cited {
            files: vec![file.clone()],
            ..Cited::default()
        };
        let now = specforge_project::judged::inputs(&compiled, &PlainDisk, PLAN, judged, &cited)
            .expect("the rule judges the plan");
        let read = now
            .files
            .iter()
            .find(|f| f.path == file)
            .expect("the cited file");
        assert_eq!(read.digest, "unreadable", "{file}");
    }
}
