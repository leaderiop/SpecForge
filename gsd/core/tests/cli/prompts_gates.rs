//! The gates of the lifecycle prompts. A refused prompt answers a `gate_failed` JSON that names the
//! failing code and lists the gates, and it writes nothing: the tree is byte-identical after the refusal. Each gate
//! has a pair: the project that trips it, and the same project once the cause is gone.

use serde_json::{Value, json};

use super::cmd_milestone::ready;
use super::commands_support::{bytes, plus, strict, twin, unchanged};
use super::harness::{Project, Run, project};
use super::prompts_support::{payload, prompt};

/// The `gate_failed` JSON a refusal printed.
pub(super) fn gate_failed(run: &Run) -> Value {
    assert_eq!(run.code, 1, "{}", run.all());
    let all = run.all();
    let start = all
        .find("{\"code\"")
        .unwrap_or_else(|| panic!("no JSON in: {all}"));
    let found: Value = serde_json::from_str(&all[start..]).unwrap_or_else(|e| panic!("{e}: {all}"));
    assert_eq!(found["code"], "gate_failed", "{all}");
    found
}

/// The prompt is not refused by `code`: it renders, or its refusal names other codes only. This is how a test shows
/// that a remedy clears one refusal while another gate may still refuse the prompt.
pub(super) fn not_refused_by(p: &Project, id: &str, args: &[&str], code: &str) {
    let run = prompt(p, id, args, "claude-code");
    if run.code == 0 {
        return;
    }
    let full = format!("gsd/{code}");
    let found = gate_failed(&run);
    let failing: Vec<&str> = found["data"]["failing"]
        .as_array()
        .expect("failing")
        .iter()
        .filter_map(|d| d["code"].as_str())
        .collect();
    assert!(
        !failing.contains(&full.as_str()),
        "{id} {args:?}: {failing:?}"
    );
}

/// The prompt is refused with `code` among its failing diagnostics and the gate marked as not passing, and nothing is
/// written.
pub(super) fn refused_by(p: &Project, id: &str, args: &[&str], code: &str) {
    let before = bytes(p);
    let found = gate_failed(&prompt(p, id, args, "claude-code"));
    let full = format!("gsd/{code}");
    let failing: Vec<&str> = found["data"]["failing"]
        .as_array()
        .expect("failing")
        .iter()
        .filter_map(|d| d["code"].as_str())
        .collect();
    assert!(failing.contains(&full.as_str()), "{id}: {failing:?}");
    let gate = found["data"]["gates"]
        .as_array()
        .expect("gates")
        .iter()
        .find(|g| g["code"] == full.as_str())
        .unwrap_or_else(|| panic!("{id}: {code} is not a gate: {found}"));
    assert_eq!(gate["passes"], false, "{id}: {gate}");
    unchanged(p, &before);
}

/// The prompt renders.
pub(super) fn renders(p: &Project, id: &str, args: &[&str]) {
    let rendered = payload(&prompt(p, id, args, "claude-code"));
    assert!(
        rendered["payload"]["prompt"]
            .as_str()
            .is_some_and(|s| s.ends_with(id)),
        "{id}"
    );
}

pub(super) const PHASE: &str = "v1.0/02";
const PLANNING: [&str; 3] = ["plan_phase", "plan_review_convergence", "ultraplan_phase"];

fn discuss_file(file: &str) -> String {
    let path = format!(
        "{}/tests/projects/discuss/spec/{file}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The verdicts of the `discuss` project's phase 02 that gsd-strict's verdict gate owes at every planning prompt, as
/// their judges record them: `spec_phase`'s on the spec's ambiguity scores (gsd/E202) and on each locked requirement
/// (gsd/W203), and the researcher's on each package it audited (gsd/W205).
fn judge_discussion(p: &Project) {
    let scores =
        r#"{"scores":{"goal":90,"boundary":80,"constraint":70,"acceptance":80,"ambiguity":10}}"#;
    let run = p.record("v1.0/02/02-SPEC", "E202", "passed", &["--values", scores]);
    assert_eq!(run.code, 0, "{}", run.all());
    for (kind, code, outcome) in [
        ("locked_req", "W203", "passed"),
        ("package_audit", "W205", "legitimate"),
    ] {
        for node in p.nodes_of(kind) {
            let run = p.record(node["id"].as_str().expect("an id"), code, outcome, &[]);
            assert_eq!(run.code, 0, "{}", run.all());
        }
    }
}

#[test]
fn e201_a_context_without_canonical_refs_refuses_the_planning_prompts() {
    let p = project("discuss");
    let spec = discuss_file("discuss.spec");
    let field = "  canonical_refs [\n    { topic \"Storage\" paths [\"docs/storage.md\"] }\n  ]\n";
    assert!(spec.contains(field));
    for id in PLANNING {
        renders(&p, id, &[PHASE]);
    }
    p.write("spec/discuss.spec", &spec.replace(field, ""));
    for id in PLANNING {
        refused_by(&p, id, &[PHASE], "E201");
    }
    p.write("spec/discuss.spec", &spec);
    for id in PLANNING {
        renders(&p, id, &[PHASE]);
    }
}

#[test]
fn w204_an_open_research_question_refuses_every_planning_prompt_under_strict() {
    let p = project("discuss");
    let spec = discuss_file("research.spec");
    let resolved = "  resolved_by D-02\n";
    assert!(spec.contains(resolved));
    p.write("spec/research.spec", &spec.replace(resolved, ""));
    for id in PLANNING {
        renders(&p, id, &[PHASE]);
    }
    strict(&p);
    for id in PLANNING {
        refused_by(&p, id, &[PHASE], "W204");
    }
    p.write("spec/research.spec", &spec);
    judge_discussion(&p);
    for id in PLANNING {
        renders(&p, id, &[PHASE]);
    }
}

#[test]
fn e206_a_second_research_in_the_phase_refuses_the_planning_prompts() {
    let p = project("discuss");
    for id in PLANNING {
        renders(&p, id, &[PHASE]);
    }
    let research = discuss_file("research.spec");
    let second = "research 02-RESEARCH-B \"A second research\" {\n  phase v1.0/02\n  domain \"Locking\"\n  researched 2026-10-28\n  confidence high\n  summary \"Two researches in one phase\"\n}\n";
    p.write("spec/research_b.spec", second);
    for id in PLANNING {
        refused_by(&p, id, &[PHASE], "E206");
    }
    p.write("spec/research_b.spec", "");
    assert_eq!(p.read("spec/research.spec"), research);
    for id in PLANNING {
        renders(&p, id, &[PHASE]);
    }
}

/// The research of phase 02 recorded against its context, then a decision of the context edited.
pub(super) fn research_against_a_stale_context(p: &Project) {
    research_in_step_with_its_context(p);
    let decisions = p.read("spec/discuss.spec");
    p.write(
        "spec/discuss.spec",
        &decisions.replace(
            "Tasks are kept in a single JSON file",
            "Tasks are kept in SQLite",
        ),
    );
}

/// The research of phase 02 recorded by `record-research`: it holds the pack of its context as it is.
pub(super) fn research_in_step_with_its_context(p: &Project) {
    let run = p.run(&["gsd", "record-research", "v1.0/02/02-RESEARCH"]);
    assert_eq!(run.code, 0, "{}", run.all());
}

#[test]
fn w406_a_drifted_context_refuses_the_planning_prompts_when_the_action_is_block() {
    let p = project("discuss");
    research_against_a_stale_context(&p);
    // a warning by default: the gate lets the prompt through
    for id in PLANNING {
        renders(&p, id, &[PHASE]);
    }
    p.settings(json!({ "workflow.context_drift_action": "block" }));
    for id in PLANNING {
        refused_by(&p, id, &[PHASE], "W406");
    }
    p.settings(json!({ "workflow.context_drift_precheck": false, "workflow.context_drift_action": "block" }));
    for id in PLANNING {
        renders(&p, id, &[PHASE]);
    }
}

/// Stale plan-check verdicts on `plan`: recorded, then their input (the instructions) edited. They count only while
/// the plan is open: an executed plan's plan check is history (its owner state), and the host owes it no verdict.
fn stale_plan_check(p: &Project, plan: &str) {
    p.write("CLAUDE.md", "# Rules\n\nNo task is longer than an hour.\n");
    for code in ["W311", "W313", "W315", "W316"] {
        let run = p.record(plan, code, "passed", &[]);
        assert_eq!(run.code, 0, "{code}: {}", run.all());
    }
    p.write(
        "CLAUDE.md",
        "# Rules\n\nNo task is longer than half an hour.\n",
    );
}

/// The state of `plan`'s W311 verdict, `None` when no verdict is owed.
fn plan_check_state(p: &Project, plan: &str) -> Option<String> {
    p.inputs(plan, "W311")
        .get("verdict")
        .map(|v| v["state"].as_str().unwrap_or_default().to_string())
}

/// The codes of the gate diagnostics that refuse `id`.
pub(super) fn failing(p: &Project, id: &str, args: &[&str]) -> Vec<String> {
    gate_failed(&prompt(p, id, args, "claude-code"))["data"]["failing"]
        .as_array()
        .expect("failing")
        .iter()
        .filter_map(|d| Some(d["code"].as_str()?.to_string()))
        .collect()
}

/// gsd-strict turns the verdict gate on (gsd-plus presets the same gate, as `profile_gates.rs` shows).
#[test]
fn e516_a_stale_verdict_refuses_complete_milestone_and_ship_under_strict() {
    // complete_milestone: the one-phase milestone whose audit passes, beside a planned milestone whose open plan's
    // plan check is stale (a stale verdict anywhere counts)
    let p = ready();
    p.write(
        "spec/next.spec",
        "milestone v2.0 \"N\" {\n  name \"N\"\n}\n\nphase 01 \"Next\" {\n  milestone v2.0\n  name \"Next\"\n  slug \"next\"\n  goal \"Do more\"\n  success_criteria [\"More is done\"]\n}\n\nplan 01-01 \"Q\" {\n  phase v2.0/01\n  type execute\n  objective \"Do more\"\n}\n",
    );
    stale_plan_check(&p, "v2.0/01/01-01");
    assert_eq!(
        plan_check_state(&p, "v2.0/01/01-01").as_deref(),
        Some("stale")
    );
    renders(&p, "complete_milestone", &["v1.0"]);
    strict(&p);
    refused_by(&p, "complete_milestone", &["v1.0"], "E516");
    p.settings(json!({}));
    renders(&p, "complete_milestone", &["v1.0"]);
    // ship: the twin with its phase 01 verified, its phase 02 plan still a draft
    let t = twin();
    assert_eq!(t.verify_phase("v1.0/01"), 0);
    stale_plan_check(&t, "v1.0/02/02-01");
    assert_eq!(
        plan_check_state(&t, "v1.0/02/02-01").as_deref(),
        Some("stale")
    );
    renders(&t, "ship", &["v1.0/01"]);
    strict(&t);
    refused_by(&t, "ship", &["v1.0/01"], "E516");
    t.settings(json!({}));
    renders(&t, "ship", &["v1.0/01"]);
}

/// A plan check is owed while its plan is open: on the twin's executed 01-02 a stale one is owed nothing and is no
/// E516 under gsd-strict and gsd-plus, where the draft 02-01's is (the case above). The twin's ship is refused under
/// gsd-strict and gsd-plus by its untraced requirements (E515) either way.
#[test]
fn e516_counts_no_stale_plan_check_of_an_executed_plan() {
    let t = twin();
    stale_plan_check(&t, "v1.0/01/01-02");
    assert_eq!(plan_check_state(&t, "v1.0/01/01-02"), None);
    plus(&t);
    let codes = failing(&t, "ship", &["v1.0/01"]);
    assert!(!codes.contains(&"gsd/E516".to_string()), "{codes:?}");
}

/// A must-have is owed its verdict once its plan is complete: a stale verifier verdict on the executed 01-02's truth
/// (its cited file edited) counts under gsd-plus, beside the unverified phase it makes (E511).
#[test]
fn e516_counts_a_stale_verdict_on_an_executed_plans_truth() {
    let t = twin();
    let truth = "v1.0/01/01-02/TR1";
    let run = t.record(truth, "E501", "verified", &["--cited", &t.cites(truth)]);
    assert_eq!(run.code, 0, "{}", run.all());
    t.write(
        "src/tasks.js",
        &format!("{}\n// edited\n", t.read("src/tasks.js")),
    );
    assert_eq!(t.inputs(truth, "E501")["verdict"]["state"], "stale");
    let codes = failing(&t, "ship", &["v1.0/01"]);
    assert!(codes.contains(&"gsd/E511".to_string()), "{codes:?}");
    assert!(!codes.contains(&"gsd/E516".to_string()), "{codes:?}");
    plus(&t);
    let codes = failing(&t, "ship", &["v1.0/01"]);
    assert!(codes.contains(&"gsd/E516".to_string()), "{codes:?}");
}

/// A verdict owed and never given blocks every step but the one that judges it: with phase 01's verifier verdicts
/// stripped, gsd-plus refuses `ship` for each, and `execute_phase`, which runs the verifier, renders.
#[test]
fn e516_a_missing_verdict_refuses_ship_but_not_the_execution_that_judges_it() {
    let t = twin();
    t.strip_verdicts();
    plus(&t);
    let found = gate_failed(&prompt(&t, "ship", &["v1.0/01"], "claude-code"));
    let missing: Vec<&str> = found["data"]["failing"]
        .as_array()
        .expect("failing")
        .iter()
        .filter(|d| d["code"] == "gsd/E516")
        .filter_map(|d| d["message"].as_str())
        .collect();
    assert_eq!(
        missing.len(),
        7,
        "one per must-have of phase 01: {missing:?}"
    );
    assert!(
        missing.iter().all(
            |m| m.starts_with("the gsd/E501 verdict on 'v1.0/01/") && m.ends_with("is missing")
        ),
        "{missing:?}"
    );
    renders(&t, "execute_phase", &["v1.0/01"]);
}

#[test]
fn the_convergence_gates_are_the_planning_gates() {
    let declaration = gsd_core::build().declaration();
    let gates = |id: &str| -> Vec<String> {
        declaration
            .surfaces
            .prompts
            .iter()
            .find(|p| p.id == id)
            .expect("the prompt")
            .gates
            .clone()
    };
    let expected = ["E201", "W204", "E206", "W406", "E516"];
    for id in PLANNING {
        assert_eq!(gates(id), expected, "{id}");
    }
}

/// A refusal leaves a project exactly as it was, whatever it asked for (`gate_failed` is not a partial write).
#[test]
fn a_refusal_in_the_twin_writes_nothing() {
    let p = twin();
    let before = bytes(&p);
    let run: Run = prompt(&p, "ship", &["v1.0/02"], "claude-code");
    gate_failed(&run);
    unchanged(&p, &before);
}
