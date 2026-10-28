//! The taskboard's SpecForge twin of a discussed and researched phase against what gsd-tools 1.18.0 says about its
//! `.planning/` (parity suite, "scenario"): phase 2's CONTEXT, DISCUSSION-LOG and RESEARCH, and PROJECT.md's Key
//! Decisions. The goldens come from `record.sh`; the `.planning/` files are the fixture's.

use std::path::Path;

use serde_json::Value;

use crate::harness::{Project, taskboard};

fn parity(path: &str) -> String {
    let full = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../parity")
        .join(path);
    std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("read {}: {e}", full.display()))
}

fn golden(name: &str) -> Value {
    serde_json::from_str(&parity(&format!("goldens/taskboard/{name}")))
        .unwrap_or_else(|e| panic!("{name} is JSON: {e}"))
}

/// The nodes `specforge query <name> --depth 1` prints.
fn neighbours(p: &Project, name: &str) -> Vec<Value> {
    let run = p.run(&["query", name, "--depth", "1", "--format", "json"]);
    assert_eq!(run.code, 0, "query {name}: {}", run.all());
    let graph: Value = serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("query {name} is JSON ({e}): {}", run.all()));
    graph["nodes"].as_array().expect("nodes").clone()
}

fn of_kind<'a>(nodes: &'a [Value], kind: &str) -> Vec<&'a Value> {
    nodes.iter().filter(|n| n["kind"] == kind).collect()
}

fn field(node: &Value, name: &str) -> String {
    node["fields"][name]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

const PHASE_DIR: &str = "fixtures/taskboard/.planning/phases/02-persistence";

/// `(id, text)` of each `- **D-NN:** text` bullet of CONTEXT.md's `<decisions>` block.
fn context_decisions() -> Vec<(String, String)> {
    let context = parity(&format!("{PHASE_DIR}/02-CONTEXT.md"));
    let block = context
        .split("<decisions>")
        .nth(1)
        .and_then(|rest| rest.split("</decisions>").next())
        .expect("a <decisions> block");
    block
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("- **")?;
            let (id, text) = rest.split_once(":** ")?;
            Some((id.to_string(), text.to_string()))
        })
        .collect()
}

#[test]
fn taskboard_discuss_matches_planning() {
    let p = taskboard();
    let check = p.run(&["check"]);
    assert_eq!(check.code, 0, "check: {}", check.all());
    assert!(
        check.all().contains("0 errors, 0 warnings"),
        "the twin checks clean (no E201, W204, W406 or E083): {}",
        check.all()
    );

    // `roadmap analyze` says where a context and a research exist: phase 2 only.
    let analyzed = golden("roadmap-analyze.json");
    for phase in analyzed["phases"].as_array().expect("phases") {
        let number = phase["number"].as_str().expect("a number");
        let id = match number.split_once('.') {
            Some((whole, part)) => format!("v1.0/{whole:0>2}.{part}"),
            None => format!("v1.0/{number:0>2}"),
        };
        let nodes = neighbours(&p, &id);
        assert_eq!(
            !of_kind(&nodes, "context").is_empty(),
            phase["has_context"] == true,
            "phase {number}: has_context"
        );
        assert_eq!(
            !of_kind(&nodes, "research").is_empty(),
            phase["has_research"] == true,
            "phase {number}: has_research"
        );
    }

    // CONTEXT.md's D-NN bullets are the twin's `origin context` decisions, and gsd-tools counts the same.
    let phase = neighbours(&p, "v1.0/02");
    let mut twin: Vec<(String, String)> = of_kind(&phase, "decision")
        .iter()
        .filter(|d| field(d, "origin") == "context")
        .map(|d| {
            let id = d["id"].as_str().expect("an id");
            (
                id.rsplit('/').next().unwrap_or(id).to_string(),
                field(d, "text"),
            )
        })
        .collect();
    twin.sort();
    assert_eq!(twin, context_decisions(), "CONTEXT.md decisions");
    let coverage = golden("decision-coverage-plan-2.json");
    assert_eq!(
        coverage["total"],
        twin.len(),
        "gsd-tools counts the same decisions"
    );

    // PROJECT.md's Key Decisions rows are the twin's project decisions.
    let project = parity("fixtures/taskboard/.planning/PROJECT.md");
    let rows: Vec<String> = project
        .split("## Key Decisions")
        .nth(1)
        .and_then(|rest| rest.split("\n---").next())
        .expect("a Key Decisions section")
        .lines()
        .filter(|l| l.starts_with('|') && !l.contains("---") && !l.contains("| Decision |"))
        .filter_map(|l| l.trim_matches('|').split('|').next())
        .map(|decision| decision.trim().to_string())
        .collect();
    let mut kept: Vec<String> = neighbours(&p, "taskboard")
        .iter()
        .filter(|n| n["kind"] == "decision")
        .map(|d| field(d, "text"))
        .collect();
    kept.sort();
    let mut rows_sorted = rows.clone();
    rows_sorted.sort();
    assert_eq!(kept, rows_sorted, "PROJECT.md Key Decisions");
    assert_eq!(rows.len(), 2);

    // DISCUSSION-LOG's areas are the log's entries.
    let log = parity(&format!("{PHASE_DIR}/02-DISCUSSION-LOG.md"));
    let areas: Vec<&str> = log
        .lines()
        .filter_map(|l| l.strip_prefix("## "))
        .filter(|h| *h != "Claude's Discretion" && *h != "Deferred Ideas")
        .collect();
    let node = neighbours(&p, "v1.0/02");
    let discussion = of_kind(&node, "discussion");
    assert_eq!(discussion.len(), 1);
    let entries = discussion[0]["fields"]["entries"]
        .as_array()
        .expect("entries");
    let logged: Vec<&str> = entries
        .iter()
        .map(|e| e["area"].as_str().expect("an area"))
        .collect();
    assert_eq!(logged, areas, "DISCUSSION-LOG areas");

    // Every open question is resolved, as RESEARCH.md's heading says.
    let research = parity(&format!("{PHASE_DIR}/02-RESEARCH.md"));
    assert!(research.contains("## Open Questions (RESOLVED)"));
    let items = neighbours(&p, "v1.0/02/02-RESEARCH");
    let questions = of_kind(&items, "open_question");
    assert_eq!(questions.len(), 1);
    assert!(
        questions
            .iter()
            .all(|q| !field(q, "resolved_by").is_empty()),
        "{questions:?}"
    );

    // gsd-tools finds nothing stale, and the twin reports no W406.
    assert_eq!(
        golden("verify-context-drift-2.json")["stale_artifacts"],
        serde_json::json!([])
    );
    assert!(!check.all().contains("W406"), "{}", check.all());
}
