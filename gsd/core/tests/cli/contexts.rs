//! The context routes `@gsd/core` declares (ADR 0356, ADR 0386), read with `specforge context` on the parity fixtures
//! and on a real-size one: each serves a subject of its kind, its core whole and its enrich sections within its
//! allowance; the checkers' packs digest as the inputs their judged rules read; the plan check reads the project's
//! decisions and those its phase's research resolves, never an unrelated phase's; the executor's keeps every task of
//! its plan and no file, and a real-size plan starts; the phase context holds what its discussion settled.

use crate::cmd_execute::{PLANTED, SUMMARY};
use crate::commands_support::plant;
use crate::harness::{Project, ledger, project, taskboard, vault};
use serde_json::Value;
use std::collections::BTreeMap;

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

/// A milestone audit of `v1.0` (of a fixture's first milestone, replaced): no fixture holds one, and the integration
/// checker reads nothing else.
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

/// The real-size fixture: the 100 project decisions of a real repository and one phase planned at the size a live run
/// met (one plan of 3 tasks, 11 truths, 7 artifacts, 3 key links and 5 prohibitions), beside a research and contexts.
fn real_size() -> Project {
    project("real-size")
}

/// Its plan and phase of that size.
const REAL_PLAN: &str = "v1.0.0/03/03-01";
const REAL_PHASE: &str = "v1.0.0/03";

/// The most a route's core may cost in a fixture: a role reads its core whole, so a core past this is a route to trim.
const ROLE_CEILING: u64 = 100_000;

/// Each route's allowance and its core sections, as `@gsd/core` declares them.
fn declared() -> BTreeMap<String, (u64, Vec<String>)> {
    let host = specforge_extension_sdk::testing::MockHost::new(gsd_core::build());
    let contexts: Value =
        serde_json::from_str(&host.describe_json("contexts").expect("contexts")).expect("JSON");
    contexts["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|route| {
            let core = route["sections"]
                .as_array()
                .expect("sections")
                .iter()
                .filter(|s| s["tier"] != "enrich")
                .map(|s| s["name"].as_str().expect("a name").to_string())
                .collect();
            let name = route["name"].as_str().expect("a name").to_string();
            (name, (route["budget"].as_u64().expect("a budget"), core))
        })
        .collect()
}

/// A default read of `route` measured: every core section served and none truncated, the cap the pack was fitted to
/// its core and the route's allowance, the pack within that cap, and the core under the role ceiling. The core's cost.
fn measured(pack: &Value, route: &str, (allowance, core): &(u64, Vec<String>)) -> u64 {
    let budget = &pack["budget"];
    let cap = budget["budget_tokens"].as_u64().expect("a cap");
    let estimated = budget["estimated_tokens"].as_u64().expect("an estimate");
    let floor = cap - allowance;
    assert!(floor <= estimated && estimated <= cap, "{route}: {budget}");
    assert!(floor <= ROLE_CEILING, "{route}: a core of {floor} tokens");
    let served: Vec<&str> = pack["sections"]
        .as_array()
        .expect("sections")
        .iter()
        .map(|s| s["name"].as_str().expect("a name"))
        .collect();
    let truncated = budget["truncated_sections"].as_array().expect("a list");
    for section in core {
        assert!(
            served.contains(&section.as_str()),
            "{route}: {section} not served"
        );
        assert!(
            !truncated.contains(&Value::from(section.as_str())),
            "{route}: {section} cut"
        );
    }
    floor
}

#[test]
fn every_route_serves_its_core_whole_within_its_allowance_in_each_fixture() {
    let declared = declared();
    let mut served = 0;
    for p in [taskboard(), vault(), ledger(), real_size()] {
        let milestone = p.nodes_of("milestone")[0]["id"]
            .as_str()
            .expect("an id")
            .to_string();
        p.write("spec/audit.spec", &AUDIT.replace("v1.0", &milestone));
        for (route, kind) in ROUTES {
            let nodes = p.nodes_of(kind);
            let subject = nodes[0]["id"].as_str().expect("an id");
            let pack = pack(&p, route, subject);
            assert_eq!(pack["route"], format!("gsd/{route}"));
            assert!(!digests(&pack).is_empty(), "{route}: {pack}");
            measured(&pack, route, &declared[route]);
            served += 1;
        }
    }
    assert_eq!(served, 56);
}

#[test]
fn the_plan_checker_serves_a_real_size_plan_whole_where_a_fixed_budget_refused_it() {
    let p = real_size();
    let pack = pack(&p, "plan-checker", REAL_PLAN);
    let floor = measured(&pack, "plan-checker", &declared()["plan-checker"]);
    // The old route's fixed budget was 8000 tokens: the project's decisions alone cost more.
    assert!(floor > 8000, "a core of {floor} tokens");
    assert_eq!(members(&pack, "project_decisions").len(), 100);
    assert_eq!(digests(&pack), input_digests(&p, REAL_PLAN, "W311"));
    // Only a cap the read states below the core is refused.
    let capped = p.run(&[
        "context",
        "gsd/plan-checker",
        REAL_PLAN,
        "--max-tokens",
        "8000",
    ]);
    assert_ne!(capped.code, 0, "{}", capped.all());
    assert!(capped.all().contains("E062"), "{}", capped.all());
}

#[test]
fn the_plan_check_reads_the_project_decisions_and_those_its_phase_research_resolves() {
    let p = real_size();
    let pack = pack(&p, "plan-checker", REAL_PLAN);
    let mut resolved = members(&pack, "resolved_decisions");
    resolved.sort();
    assert_eq!(resolved, ["v1.0.0/02/D-01", "v1.0.0/03/D-02"]);
    let phase: Vec<String> = members(&pack, "phase_decisions");
    assert_eq!(phase.len(), 4, "{phase:?}");
    assert!(members(&pack, "quick_decisions").is_empty());
    // A decision of another phase that answers no question of this one is read by no section, and moving it stales
    // no plan check of this phase.
    let read = |pack: &Value| -> Vec<String> {
        [
            "phase_decisions",
            "quick_decisions",
            "project_decisions",
            "resolved_decisions",
        ]
        .iter()
        .flat_map(|section| members(pack, section))
        .collect()
    };
    assert!(!read(&pack).contains(&"v1.0.0/02/D-02".to_string()));
    let before = input_digests(&p, REAL_PLAN, "W311");
    p.edit(
        "spec/discuss.spec",
        "Nodes are sorted by id and edges by source, label and target",
        "Nodes are sorted by id, edges by source, label and target",
    );
    assert_eq!(input_digests(&p, REAL_PLAN, "W311"), before);
}

#[test]
fn start_plan_records_a_real_size_plans_executor_pack_and_only_a_stated_cap_refuses_it() {
    let p = real_size();
    let read = pack(&p, "executor", REAL_PLAN);
    let floor = measured(&read, "executor", &declared()["executor"]);
    // The old route's fixed budget was 6000 tokens, below this plan's core: the record was refused.
    assert!(floor > 6000, "a core of {floor} tokens");
    let run = p.run(&["gsd", "start-plan", REAL_PLAN]);
    assert_eq!(run.code, 0, "{}", run.all());
    let record = &p.fields_of(REAL_PLAN)["context_pack"];
    assert_eq!(record["digest"], read["digest"], "{record}");
    // A read that states no cap records none: the record is read again under the route's allowance.
    assert!(record.get("max_tokens").is_none(), "{record}");
    assert!(
        !p.codes().contains(&"gsd/W524".to_string()),
        "{:?}",
        p.codes()
    );
    let refused = p.run(&["gsd", "start-plan", REAL_PLAN, "--max-tokens", "100"]);
    assert_ne!(refused.code, 0, "{}", refused.all());
    assert!(
        refused.all().contains("record_refused"),
        "{}",
        refused.all()
    );
}

#[test]
fn the_real_size_phase_is_served_whole_to_the_roles_that_read_it() {
    let p = real_size();
    let declared = declared();
    for route in [
        "planner",
        "verifier",
        "phase-researcher",
        "nyquist-auditor",
        "phase-context",
    ] {
        let pack = pack(&p, route, REAL_PHASE);
        measured(&pack, route, &declared[route]);
    }
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

/// The plan the executor tests open, and a line of one of its tasks.
const PLAN: &str = "v1.0/02/02-01";
const DONE: &str = "done        \"The store tests pass\"";

/// `start-plan` on `plan`, which records its executor pack.
fn start(p: &Project, plan: &str) {
    let run = p.run(&["gsd", "start-plan", plan]);
    assert_eq!(run.code, 0, "{}", run.all());
}

/// The message of `gsd/W524`, when the check raises it.
fn w524(p: &Project) -> Option<String> {
    p.report()
        .into_iter()
        .find(|(code, _)| code == "gsd/W524")
        .map(|(_, message)| message)
}

#[test]
fn a_plan_whose_executor_pack_moved_is_w524_while_it_is_open() {
    let p = taskboard();
    start(&p, PLAN);
    p.edit(
        "spec/plans.spec",
        DONE,
        "done        \"The store tests pass twice\"",
    );
    let message = w524(&p).unwrap_or_else(|| panic!("no W524: {:?}", p.codes()));
    assert!(
        message.contains(PLAN) && message.contains("gsd/executor"),
        "{message}"
    );
    // The continuation re-records the pack, and the plan is silent again.
    start(&p, PLAN);
    assert_eq!(w524(&p), None);
    // Moved again, then completed: a closed plan's record is not checked.
    p.edit(
        "spec/plans.spec",
        "done        \"The store tests pass twice\"",
        DONE,
    );
    assert!(w524(&p).is_some(), "{:?}", p.codes());
    plant(&p, "spec/plans.spec", PLANTED);
    let run = p.run(&["gsd", "complete-plan", SUMMARY]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(w524(&p), None, "no W524 once the plan is complete");
}

#[test]
fn a_recorded_executor_pack_in_step_with_its_plan_is_silent() {
    let p = taskboard();
    start(&p, PLAN);
    assert_eq!(w524(&p), None, "no W524: {:?}", p.codes());
    // Recording, and opening the plan, move nothing the executor reads.
    assert_eq!(
        p.fields_of(PLAN)["context_pack"]["digest"],
        pack(&p, "executor", PLAN)["digest"]
    );
}
