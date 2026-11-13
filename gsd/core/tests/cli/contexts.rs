//! The context routes `@gsd/core` declares (ADR 0356), read with `specforge context` on the parity fixtures: each
//! serves a subject of its kind; the checkers' packs digest as the inputs their judged rules read; the executor's keeps
//! every task of its plan and no file; the phase context holds what its discussion settled.

use crate::harness::{Project, ledger, taskboard, vault};
use serde_json::Value;

/// Every route and its subject kind.
const ROUTES: [(&str, &str); 14] = [
    ("executor", "plan"),
    ("planner", "phase"),
    ("plan-checker", "plan"),
    ("verifier", "phase"),
    ("integration-checker", "milestone_audit"),
    ("advisor-researcher", "phase"),
    ("assumptions-analyzer", "phase"),
    ("project-researcher", "project"),
    ("research-synthesizer", "project"),
    ("roadmapper", "project"),
    ("phase-researcher", "phase"),
    ("nyquist-auditor", "phase"),
    ("phase-context", "phase"),
    ("quick-context", "plan"),
];

/// A milestone audit of `v1.0`: no fixture holds one, and the integration checker reads nothing else.
const AUDIT: &str = "milestone_audit A1 \"Audit\" {\n  milestone v1.0\n}\n";

/// `specforge context gsd/<route> <subject>`, its pack.
fn pack(p: &Project, route: &str, subject: &str) -> Value {
    let run = p.run(&["context", &format!("gsd/{route}"), subject]);
    assert_eq!(run.code, 0, "{route} {subject}: {}", run.all());
    serde_json::from_str(&run.stdout).unwrap_or_else(|e| panic!("a pack ({e}): {}", run.all()))
}

/// Each served section's `(name, digest)`, in order.
fn digests(pack: &Value) -> Vec<(String, String)> {
    pack["sections"]
        .as_array()
        .expect("sections")
        .iter()
        .map(|s| {
            (
                s["name"].as_str().expect("a string").into(),
                s["digest"].as_str().expect("a string").into(),
            )
        })
        .collect()
}

/// The members of the section `name`, by path.
fn members(pack: &Value, name: &str) -> Vec<String> {
    let section = pack["sections"]
        .as_array()
        .expect("sections")
        .iter()
        .find(|s| s["name"] == name)
        .unwrap_or_else(|| panic!("no section {name}: {pack}"));
    section["members"]
        .as_array()
        .map(|m| {
            m.iter()
                .map(|m| m.as_str().expect("a string").to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// Each input's `(name, digest)` of `gsd/<code>` on `entity`, in order.
fn input_digests(p: &Project, entity: &str, code: &str) -> Vec<(String, String)> {
    let inputs = p.inputs(entity, code);
    inputs["inputs"]
        .as_array()
        .unwrap_or_else(|| panic!("inputs: {inputs}"))
        .iter()
        .map(|i| {
            (
                i["name"].as_str().expect("a string").into(),
                i["digest"].as_str().expect("a string").into(),
            )
        })
        .collect()
}

#[test]
fn every_route_serves_a_subject_of_its_kind_in_each_fixture() {
    let mut served = 0;
    for p in [taskboard(), vault(), ledger()] {
        p.write("spec/audit.spec", AUDIT);
        for (route, kind) in ROUTES {
            let nodes = p.nodes_of(kind);
            let subject = nodes[0]["id"].as_str().expect("an id");
            let pack = pack(&p, route, subject);
            assert_eq!(pack["route"], format!("gsd/{route}"));
            assert!(!digests(&pack).is_empty(), "{route}: {pack}");
            served += 1;
        }
    }
    assert_eq!(served, 42);
}

#[test]
fn the_plan_checker_pack_digests_as_the_plan_check_inputs() {
    let p = taskboard();
    let plan = "v1.0/02/02-01";
    let pack = pack(&p, "plan-checker", plan);
    assert_eq!(digests(&pack), input_digests(&p, plan, "W311"));
}

#[test]
fn the_integration_checker_pack_digests_as_the_e518_inputs() {
    let p = taskboard();
    p.write("spec/audit.spec", AUDIT);
    let audit = p.nodes_of("milestone_audit")[0]["id"]
        .as_str()
        .expect("an id")
        .to_string();
    let pack = pack(&p, "integration-checker", &audit);
    assert_eq!(digests(&pack), input_digests(&p, &audit, "E518"));
}

#[test]
fn the_executor_pack_keeps_every_task_of_its_plan_and_serves_no_file_it_edits() {
    let p = taskboard();
    let plan = "v1.0/02/02-01";
    let pack = pack(&p, "executor", plan);
    let tasks: Vec<String> = p
        .nodes_of("task")
        .iter()
        .filter(|t| t["fields"]["plan"] == plan)
        .map(|t| t["id"].as_str().expect("a string").to_string())
        .collect();
    assert!(!tasks.is_empty());
    let body = members(&pack, "body");
    for task in &tasks {
        assert!(body.contains(task), "{task} not in {body:?}");
    }
    let sections = pack["sections"].as_array().expect("sections");
    // The instructions are the project's, never the plan's `files_modified`.
    for section in sections.iter().filter(|s| s["kind"] == "files") {
        assert_eq!(section["name"], "instructions", "{section}");
    }
    assert!(pack["budget"]["truncated_sections"].as_array().is_some());
}

#[test]
fn the_phase_context_pack_holds_the_context_its_discussed_decisions_and_deferred_ideas() {
    let p = taskboard();
    let phase = "v1.0/02";
    let decisions = |p: &Project, origin: &str| -> Vec<String> {
        p.nodes_of("decision")
            .iter()
            .filter(|d| d["fields"]["phase"] == phase && d["fields"]["origin"] == origin)
            .map(|d| d["id"].as_str().expect("a string").to_string())
            .collect()
    };
    let discussed = decisions(&p, "context");
    assert!(!discussed.is_empty(), "phase 02 discussed decisions");
    let first = pack(&p, "phase-context", phase);
    assert_eq!(members(&first, "decisions"), discussed);
    let of_kind = |kind: &str| -> Vec<String> {
        p.nodes_of(kind)
            .iter()
            .filter(|n| n["fields"]["phase"] == phase)
            .map(|n| n["id"].as_str().expect("a string").to_string())
            .collect()
    };
    assert_eq!(members(&first, "context"), of_kind("context"));
    assert_eq!(members(&first, "deferred"), of_kind("deferred_idea"));

    // A decision a summary records is no member.
    p.write(
        "spec/summary-decision.spec",
        "decision D-SUM \"From a summary\" {\n  phase v1.0/02\n  origin summary\n  text \"Kept\"\n}\n",
    );
    assert_eq!(decisions(&p, "summary").len(), 1);
    let again = pack(&p, "phase-context", phase);
    assert_eq!(members(&again, "decisions"), discussed);
}
