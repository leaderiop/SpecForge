//! The verifier's verdicts (`gsd/E501`) on the `verify` project's truths, artifacts and key links: what recording one
//! does to the check and to the verification's status and score.

use super::harness::{Project, judged, project};

const VERIFICATION: &str = "v1.0/01/01-VERIFICATION";
const PLAN_1: &str = "v1.0/01/01-01";
const PLAN_2: &str = "v1.0/01/01-02";
const TRUTH: &str = "v1.0/01/01-01/TR1";

/// The phase's truths, artifacts and key links (two plans, one of each kind each).
const ITEMS: [&str; 6] = [
    "v1.0/01/01-01/TR1",
    "v1.0/01/01-01/AR1",
    "v1.0/01/01-01/KL1",
    "v1.0/01/01-02/TR1",
    "v1.0/01/01-02/AR1",
    "v1.0/01/01-02/KL1",
];

fn verify() -> Project {
    project("verify")
}

fn verification(p: &Project) -> serde_json::Value {
    p.fields_of(VERIFICATION)
}

/// The fixture's override, gap and human item removed: nothing is accepted by a person, nothing is open.
fn bare() -> Project {
    let p = verify();
    let text = p.read("spec/verify.spec");
    let at = text.find("verify_gap VG1").expect("the gap");
    p.write("spec/verify.spec", &format!("{}\n", text[..at].trim_end()));
    p
}

/// The entities of phase 01 that gsd/E501 reports (the project has other phases' items too).
fn e501(p: &Project) -> Vec<String> {
    judged(&p.report_all())
        .into_iter()
        .filter(|(c, id)| c == "gsd/E501" && id.starts_with("v1.0/01/"))
        .map(|(_, id)| id)
        .collect()
}

fn verified(p: &Project, entity: &str) {
    let run = p.record(entity, "E501", "verified", &[]);
    assert_eq!(run.code, 0, "{entity}: {}", run.all());
}

#[test]
fn a_verified_truth_records() {
    let p = bare();
    let mut all = ITEMS.map(String::from).to_vec();
    all.sort();
    assert_eq!(e501(&p), all, "every item has no verdict");
    verified(&p, "v1.0/01/01-01/TR1");
    // Only that truth's E501 is gone; the harness line and the member both exist.
    let mut rest = all.clone();
    rest.retain(|id| id != "v1.0/01/01-01/TR1");
    assert_eq!(e501(&p), rest);
    assert!(
        p.read("specforge-harness")
            .contains("v1.0/01/01-01/TR1 verdict:gsd/E501 ")
    );
    assert!(
        p.read("spec/plans.spec")
            .contains("verdict gsd/E501 verified {")
    );
    assert!(p.report_all().iter().all(|(c, _)| c != "E083"));
}

#[test]
fn a_key_link_with_a_pattern_is_not_judged() {
    let p = bare();
    p.edit(
        "spec/plans.spec",
        "  from \"test/tasks.test.js\"\n  to   \"src/tasks.js\"\n  via  \"require\"\n",
        "  from \"test/tasks.test.js\"\n  to   \"src/tasks.js\"\n  via  \"require\"\n  pattern \"require\"\n",
    );
    let judged_ids = e501(&p);
    assert!(
        !judged_ids.contains(&"v1.0/01/01-01/KL1".to_string()),
        "{judged_ids:?}"
    );
    // The other key link, without a pattern, still is.
    assert!(
        judged_ids.contains(&"v1.0/01/01-02/KL1".to_string()),
        "{judged_ids:?}"
    );
    assert_eq!(judged_ids.len(), 5, "{judged_ids:?}");
    // The host agrees: it reports no verdict state for the link with a pattern, and `missing` for the other.
    assert!(
        p.inputs("v1.0/01/01-01/KL1", "E501")
            .get("verdict")
            .is_none()
    );
    assert_eq!(
        p.inputs("v1.0/01/01-02/KL1", "E501")["verdict"]["state"],
        "missing"
    );
}

#[test]
fn every_item_verified_makes_passed() {
    let p = bare();
    let before = verification(&p);
    assert_eq!(before["status"], "pending", "{before}");
    for (n, item) in ITEMS.iter().enumerate() {
        verified(&p, item);
        let found = verification(&p);
        if n + 1 < ITEMS.len() {
            assert_eq!(found["status"], "pending", "after {item}: {found}");
        }
    }
    let found = verification(&p);
    assert_eq!(found["status"], "passed", "{found}");
    assert_eq!(found["score"], "2/2", "{found}");
    assert_eq!(e501(&p), Vec::<String>::new());
    // `covered_files` lists the files the verdicts cited: none here.
    assert!(
        found.get("covered_files").is_none() || found["covered_files"] == serde_json::json!([])
    );
}

#[test]
fn a_coincidental_verdict_verifies_too() {
    let p = bare();
    for item in ITEMS {
        let outcome = if item.ends_with("AR1") {
            "verified_coincidental"
        } else {
            "verified"
        };
        let run = p.record(item, "E501", outcome, &[]);
        assert_eq!(run.code, 0, "{item}: {}", run.all());
    }
    assert_eq!(verification(&p)["status"], "passed");
}

#[test]
fn a_stale_verdict_is_not_a_pass() {
    let p = bare();
    for item in ITEMS {
        verified(&p, item);
    }
    assert_eq!(verification(&p)["status"], "passed");
    p.edit(
        "spec/plans.spec",
        "A user can complete a task",
        "A user can finish a task",
    );
    let found = verification(&p);
    assert_eq!(found["status"], "pending", "{found}");
    assert_eq!(found["score"], "1/2", "{found}");
    assert_eq!(e501(&p), vec!["v1.0/01/01-02/TR1".to_string()]);
}

#[test]
fn an_override_still_verifies() {
    let p = verify();
    // The fixture's override accepts 01-01/TR1; its gap and human item are removed so only verdicts are left.
    let text = p.read("spec/verify.spec");
    let gap = text.find("verify_gap VG1").expect("the gap");
    let ov = text.find("override OV1").expect("the override");
    let human = text.find("human_item HI1").expect("the human item");
    p.write(
        "spec/verify.spec",
        &format!(
            "{}\n{}\n",
            text[..gap].trim_end(),
            text[ov..human].trim_end()
        ),
    );
    for item in &ITEMS[1..] {
        verified(&p, item);
    }
    let found = verification(&p);
    assert_eq!(found["status"], "passed", "{found}");
    assert_eq!(found["score"], "2/2", "{found}");
}

/// A failing verdict on E501 (an Error) is refused whole by the host (E080, D-r40-1), so these two wait for the
/// kernel. The derivation itself is tested on synthetic judgements (`passes::derive::verify`).
#[test]
#[ignore = "kernel: a verdict's own judged state is not a new error (K17)"]
fn a_failed_truth_makes_gaps_found() {
    let p = bare();
    let run = p.record("v1.0/01/01-01/TR1", "E501", "failed", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(verification(&p)["status"], "gaps_found");
}

#[test]
#[ignore = "kernel: a verdict's own judged state is not a new error (K17)"]
fn a_backstop_truth_is_human_needed() {
    let p = bare();
    for item in &ITEMS[1..] {
        verified(&p, item);
    }
    let run = p.record(
        "v1.0/01/01-01/TR1",
        "E501",
        "present_behavior_unverified",
        &[],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(verification(&p)["status"], "human_needed");
}

#[test]
fn a_failed_truth_is_refused_whole_for_now() {
    let p = bare();
    let spec = p.read("spec/plans.spec");
    for outcome in ["failed", "uncertain", "present_behavior_unverified"] {
        let run = p.record("v1.0/01/01-01/TR1", "E501", outcome, &[]);
        assert_ne!(run.code, 0, "{outcome}: {}", run.all());
        assert!(run.all().contains("E080"), "{outcome}: {}", run.all());
    }
    assert_eq!(p.read("spec/plans.spec"), spec);
    assert_eq!(verification(&p)["status"], "pending");
}

#[test]
fn the_plans_of_the_phase_are_the_ones_judged() {
    // Pins the item list above to the fixture.
    let p = bare();
    let mut ids: Vec<String> = ["truth", "artifact", "key_link"]
        .iter()
        .flat_map(|k| p.nodes_of(k))
        .map(|n| n["id"].as_str().expect("an id").to_string())
        .filter(|id| id.starts_with(PLAN_1) || id.starts_with(PLAN_2))
        .collect();
    ids.sort();
    let mut want = ITEMS.map(String::from).to_vec();
    want.sort();
    assert_eq!(ids, want);
}

#[test]
fn covered_digest_changes_with_a_covered_file() {
    let p = bare();
    let read = |p: &Project| {
        let run = p.run(&[
            "query",
            TRUTH,
            "--inputs",
            "gsd/E501",
            "--cite-file",
            "src/tasks.js",
            "--format",
            "json",
        ]);
        serde_json::from_str::<serde_json::Value>(&run.stdout).expect("inputs")["basis"].to_string()
    };
    let basis = read(&p);
    let cited = r#"[{"file":"src/tasks.js"}]"#;
    let run = p.record_over(TRUTH, "E501", "verified", &basis, &["--cited", cited]);
    assert_eq!(run.code, 0, "{}", run.all());
    let before = verification(&p);
    let digest = before["covered_digest"]
        .as_str()
        .expect("a digest")
        .to_string();
    assert!(digest.starts_with("fnv1a64:"), "{before}");
    assert_eq!(before["covered_files"], serde_json::json!(["src/tasks.js"]));
    p.write(
        "src/tasks.js",
        "export const add = (l, t) => [...l, t];\n// edited\n",
    );
    let after = verification(&p);
    assert_ne!(
        after["covered_digest"].as_str(),
        Some(digest.as_str()),
        "{after}"
    );
}

const AUDIT: &str = "v1.0/A1";

#[test]
fn e518_inputs() {
    let p = verify();
    let inputs = p.inputs(AUDIT, "E518");
    let names: Vec<&str> = inputs["inputs"]
        .as_array()
        .expect("inputs")
        .iter()
        .filter_map(|i| i["name"].as_str())
        .collect();
    assert_eq!(
        names,
        ["audit", "phases", "verifications", "requirements"],
        "{inputs}"
    );
    assert_eq!(inputs["verdict"]["state"], "missing", "{inputs}");
}

#[test]
fn an_audit_without_a_verdict_reports_e518() {
    let p = verify();
    let found: Vec<_> = judged(&p.report_all())
        .into_iter()
        .filter(|(c, _)| c == "gsd/E518")
        .collect();
    assert_eq!(found, [("gsd/E518".to_string(), AUDIT.to_string())]);
}

#[test]
fn a_passed_integration_sets_the_field() {
    let p = verify();
    assert_eq!(p.fields_of(AUDIT).get("integration"), None);
    let run = p.record(AUDIT, "E518", "passed", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.fields_of(AUDIT)["integration"], "passed");
    assert!(
        !judged(&p.report_all()).iter().any(|(c, _)| c == "gsd/E518"),
        "{:?}",
        p.report_all()
    );
    // A hand-written value is E082.
    let spec = p.read("spec/audit.spec");
    p.write(
        "spec/audit.spec",
        &spec.replace(
            "  milestone v1.0\n",
            "  milestone v1.0\n  integration issues\n",
        ),
    );
    assert!(p.codes().contains(&"E082".to_string()), "{:?}", p.codes());
}
