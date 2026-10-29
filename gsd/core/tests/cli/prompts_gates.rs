//! The gates of the lifecycle prompts (REVIEW-r42 M6). A refused prompt answers a `gate_failed` JSON that names the
//! failing code and lists the gates, and it writes nothing: the tree is byte-identical after the refusal. Each gate
//! has a pair: the project that trips it, and the same project once the cause is gone.

use serde_json::{Value, json};

use super::cmd_milestone::ready;
use super::commands_support::{bytes, plus, strict, twin, unchanged};
use super::harness::{Project, Run, project};
use super::prompts_support::{payload, prompt};

/// The `gate_failed` JSON a refusal printed.
fn gate_failed(run: &Run) -> Value {
    assert_eq!(run.code, 1, "{}", run.all());
    let all = run.all();
    let start = all
        .find("{\"code\"")
        .unwrap_or_else(|| panic!("no JSON in: {all}"));
    let found: Value = serde_json::from_str(&all[start..]).unwrap_or_else(|e| panic!("{e}: {all}"));
    assert_eq!(found["code"], "gate_failed", "{all}");
    found
}

/// The prompt is refused with `code` among its failing diagnostics and the gate marked as not passing, and nothing is
/// written.
fn refused_by(p: &Project, id: &str, args: &[&str], code: &str) {
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
fn renders(p: &Project, id: &str, args: &[&str]) {
    let rendered = payload(&prompt(p, id, args, "claude-code"));
    assert!(
        rendered["payload"]["prompt"]
            .as_str()
            .is_some_and(|s| s.ends_with(id)),
        "{id}"
    );
}

const PHASE: &str = "v1.0/02";
const PLANNING: [&str; 3] = ["plan_phase", "plan_review_convergence", "ultraplan_phase"];

fn discuss_file(file: &str) -> String {
    let path = format!(
        "{}/tests/projects/discuss/spec/{file}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
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

/// The digest `compute` gave the context of phase 02, recorded on its research, then a decision of the context edited.
fn research_against_a_stale_context(p: &Project) {
    let query = p.run(&[
        "query",
        "v1.0/02/02-CONTEXT",
        "--depth",
        "0",
        "--format",
        "json",
    ]);
    let graph: Value = serde_json::from_str(&query.stdout).expect("JSON");
    let digest = graph["nodes"][0]["fields"]["digest"]
        .as_str()
        .expect("a digest")
        .to_string();
    let anchor = "research 02-RESEARCH \"Persistence research\" {\n";
    let spec = discuss_file("research.spec");
    p.write(
        "spec/research.spec",
        &spec.replace(anchor, &format!("{anchor}  context_digest {digest:?}\n")),
    );
    p.record_harness(
        "v1.0/02/02-RESEARCH",
        "context_digest",
        &format!("{digest:?}"),
    );
    let decisions = p.read("spec/discuss.spec");
    p.write(
        "spec/discuss.spec",
        &decisions.replace(
            "Tasks are kept in a single JSON file",
            "Tasks are kept in SQLite",
        ),
    );
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

/// A stale plan-check verdict on `plan`: recorded, then its input (the instructions) edited.
fn stale_verdict(p: &Project, plan: &str) {
    p.write("CLAUDE.md", "# Rules\n\nNo task is longer than an hour.\n");
    for code in ["W311", "W313", "W315", "W316"] {
        let run = p.record(plan, code, "passed", &[]);
        assert_eq!(run.code, 0, "{code}: {}", run.all());
    }
    p.write(
        "CLAUDE.md",
        "# Rules\n\nNo task is longer than half an hour.\n",
    );
    assert_eq!(p.inputs(plan, "W311")["verdict"]["state"], "stale");
}

#[test]
fn e516_a_stale_verdict_refuses_complete_milestone_and_ship_under_plus() {
    // complete_milestone: the one-phase milestone whose audit passes
    let p = ready();
    stale_verdict(&p, "v1.0/01/01-01");
    renders(&p, "complete_milestone", &["v1.0"]);
    plus(&p);
    refused_by(&p, "complete_milestone", &["v1.0"], "E516");
    p.settings(json!({}));
    renders(&p, "complete_milestone", &["v1.0"]);
    // ship: the twin with its phase 01 verified
    let t = twin();
    assert!(t.verify_phase("v1.0/01") > 0);
    stale_verdict(&t, "v1.0/01/01-02");
    renders(&t, "ship", &["v1.0/01"]);
    plus(&t);
    refused_by(&t, "ship", &["v1.0/01"], "E516");
    t.settings(json!({}));
    renders(&t, "ship", &["v1.0/01"]);
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
