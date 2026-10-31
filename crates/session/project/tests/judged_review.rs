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
    verify = "a reference digests as the graph links it"
)]
fn a_reference_digests_as_the_graph_links_it() {
    // `phase 03` and `phase v1.0/03` are one value.
    let project = gsd_judged_project("");
    let before = digest_of(&compile(&project), "v1.0/03/D-01");
    project.write(
        "a.spec",
        project.read("a.spec").replace(
            "\"Use tokens\" {\n  phase 03",
            "\"Use tokens\" {\n  phase v1.0/03",
        ),
    );
    assert_eq!(digest_of(&compile(&project), "v1.0/03/D-01"), before);
    // A nearer entity of the wrong kind is not what the field names.
    let project = gsd_judged_project("\ntask AUTH-07 \"Same name\" {\n  plan 03-02\n}\n");
    let compiled = compile(&project);
    let node = compiled.graph().node(PLAN).expect("the plan");
    let forms = compiled.environment().registries().field_forms();
    let requirements =
        specforge_graph::wire::resolved(compiled.graph(), forms, node, "requirements")
            .expect("the field");
    assert_eq!(requirements.as_value(), &json!(["AUTH-07"]));
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

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a baseline whose files git could not read is neither cached nor judged"
)]
fn a_failed_blob_read_poisons_no_baseline() {
    use specforge_project::baseline::Baseline;
    use specforge_project::history::{History, HistoryError, Located, Sha};
    use specforge_project::testing::FakeHistory;
    use std::path::PathBuf;
    let project = gsd_judged_project("");
    record(&project, PLAN, W311, "passed");
    let compiled = compile(&project);
    let id = Sha::new("b".repeat(40));
    let files = vec![
        (PathBuf::from("a.spec"), project.read("a.spec").into_bytes()),
        (
            PathBuf::from("specforge-harness"),
            project.read("specforge-harness").into_bytes(),
        ),
    ];
    let mut fake = FakeHistory::repo(project.root(), id.as_str(), files);
    fake.blobs_fail = Some(HistoryError("git went away".into()));
    let Located::Repo(place) = History::locate(&fake, project.root()) else {
        panic!("a repository")
    };
    let env = compiled.environment();
    let why = Baseline::at(env, &fake, &place, &id).expect_err("an unreadable baseline");
    assert!(why.contains("git went away"), "{why}");
    let cached = std::fs::read_dir(project.root().join(".specforge"))
        .map(|dir| {
            dir.flatten()
                .any(|e| e.file_name().to_string_lossy().ends_with(".json"))
        })
        .unwrap_or(false);
    assert!(!cached, "a failed read was cached");
    // Git answering again gives the baseline the verdict holds in.
    fake.blobs_fail = None;
    fake.blobs = vec![
        (PathBuf::from("CLAUDE.md"), b"# Instructions\n".to_vec()),
        (
            PathBuf::from("src/auth/session.ts"),
            b"export const session = 1;\n".to_vec(),
        ),
    ];
    let baseline = Baseline::at(env, &fake, &place, &id).expect("a baseline");
    assert!(
        baseline.entities[PLAN]
            .judgements
            .iter()
            .any(|j| j.rule == W311 && j.state == "fresh"),
        "{:?}",
        baseline.entities[PLAN].judgements
    );
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a fresh verdict's message counts its findings"
)]
fn a_judged_message_counts_its_findings() {
    let message = |findings: &str| {
        let project = gsd_judged_project("");
        let cites = if findings.is_empty() {
            String::new()
        } else {
            format!("    findings [{findings}]\n")
        };
        record_citing(&project, PLAN, W311, "issues", &Cited::default(), &cites);
        compile(&project)
            .diagnostics()
            .into_iter()
            .find(|d| d.code == W311 && d.message.contains(PLAN) && d.message.contains("'issues'"))
            .unwrap_or_else(|| panic!("a finding on {PLAN}"))
            .message
    };
    let one = r#"{ severity "error" message "no wiring" }"#;
    let two = format!(r#"{one}, {{ severity "warning" message "thin" }}"#);
    assert!(
        message(&two).contains("is 'issues' (2 findings)"),
        "{}",
        message(&two)
    );
    assert!(
        message(one).contains("is 'issues' (1 finding)"),
        "{}",
        message(one)
    );
    assert!(
        message("").contains("is 'issues' (0 findings)"),
        "{}",
        message("")
    );
}

#[specforge_test(
    behavior = "answer_judged_inputs",
    verify = "a pass sees the tests a verdict cites"
)]
fn a_pass_sees_the_tests_a_verdict_cites() {
    let project = gsd_judged_project("");
    let cited = Cited {
        tests: vec!["t one".into()],
        ..Cited::default()
    };
    record_citing(
        &project,
        TRUTH,
        E501,
        "passed",
        &cited,
        "    cited_tests [\"t one\"]\n",
    );
    let compiled = compile(&project);
    let input = specforge_project::passes::pass_input(&specforge_project::passes::PassContext {
        judgements: Some(compiled.judgements()),
        ..specforge_project::passes::PassContext::of(compiled.entities())
    });
    let truth = input.entities.iter().find(|e| e.id == TRUTH).unwrap();
    let judgement = truth.judgements.iter().find(|j| j.rule == E501).unwrap();
    let wire = serde_json::to_value(judgement).unwrap();
    assert_eq!(wire["cited_tests"], json!(["t one"]), "{wire}");
}
