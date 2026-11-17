use serde_json::{Value, json};

use super::*;

const PLAN_01: &str = "v1.0/01/01-01";
const PLAN_02: &str = "v1.0/02/02-01";

fn entity(id: &str, kind: &str, fields: Value) -> Value {
    json!({ "id": id, "kind": kind, "fields": fields })
}

/// Phases 01 and 02 each with a context decision `D-01`, the project's own `D-01`, a plan in each phase, and `more`.
fn input(more: Vec<Value>) -> PassInput {
    let mut entities = vec![
        entity("v1.0/01", "phase", json!({})),
        entity("v1.0/02", "phase", json!({})),
        entity(
            "v1.0/01/D-01",
            "decision",
            json!({ "origin": "context", "text": "ES modules" }),
        ),
        entity(
            "v1.0/02/D-01",
            "decision",
            json!({ "origin": "context", "text": "One JSON file" }),
        ),
        entity("D-01", "decision", json!({ "text": "No build step" })),
        entity(PLAN_01, "plan", json!({ "phase": "v1.0/01" })),
        entity(PLAN_02, "plan", json!({ "phase": "v1.0/02" })),
    ];
    entities.extend(more);
    serde_json::from_value(crate::passes::plans::with_field_edges(
        json!({ "entities": entities, "edges": [] }),
    ))
    .expect("a pass input")
}

/// What `text`, written in plan 02-01, cites.
fn cited_in_02(text: &str) -> Vec<String> {
    let input = input(vec![]);
    cited(&decisions(&input), PLAN_02, text)
        .into_iter()
        .map(String::from)
        .collect()
}

/// What the plan `plan` cites, as `cited_by` reads it.
fn cited_by_plan(input: &PassInput, plan: &str) -> Vec<String> {
    let plan = input
        .entities
        .iter()
        .find(|e| e.id == plan)
        .expect("the plan");
    cited_by(input, [plan])
        .into_iter()
        .map(String::from)
        .collect()
}

/// An id meant for another phase is written with its path: upstream's reading, accepted.
#[test]
fn a_bare_id_is_the_plans_own_phases() {
    assert_eq!(cited_in_02("Implement D-01"), ["v1.0/02/D-01"]);
    let input = input(vec![]);
    assert_eq!(
        cited(&decisions(&input), PLAN_01, "Implement D-01"),
        BTreeSet::from(["v1.0/01/D-01"])
    );
    // A plan of a phase with no `D-01` reads the project's.
    assert_eq!(
        cited(&decisions(&input), "v1.0/03/03-01", "Implement D-01"),
        BTreeSet::from(["D-01"])
    );
}

#[test]
fn a_path_names_another_phases_decision() {
    for written in [
        "Keep 01/D-01",
        "Keep v1.0/01/D-01",
        "Keep /v1.0/01/D-01",
        "(01/D-01)",
    ] {
        assert_eq!(cited_in_02(written), ["v1.0/01/D-01"], "{written}");
    }
    assert_eq!(cited_in_02("Keep /D-01"), ["D-01"], "rooted: the project's");
    assert_eq!(
        cited_in_02("Keep 01/D-01 and D-01"),
        ["v1.0/01/D-01", "v1.0/02/D-01"]
    );
}

#[test]
fn a_path_that_names_no_decision_cites_nothing() {
    for written in ["src/D-01", "see src/D-01.js", "v1.0/03/D-01", "x/01/D-01"] {
        assert!(cited_in_02(written).is_empty(), "{written}");
    }
}

#[test]
fn the_id_is_a_whole_word() {
    for written in ["D-010", "XD-01", "_D-01"] {
        assert!(cited_in_02(written).is_empty(), "{written}");
    }
    for written in ["D-01-x", "(D-01)", "D-01.", "D-01,"] {
        assert_eq!(cited_in_02(written), ["v1.0/02/D-01"], "{written}");
    }
}

#[test]
fn case_is_as_written() {
    assert!(cited_in_02("d-01").is_empty());
}

/// gsd-core 1.19.0 also counts a decision cited when the first six normalised words of its text are in the plan's
/// normalised text; SpecForge counts the id alone.
#[test]
fn no_paraphrase_cites() {
    let input = input(vec![
        entity(
            "v1.0/02/D-02",
            "decision",
            json!({ "origin": "context", "text": "Completed tasks move to a done list at once" }),
        ),
        entity(
            "v1.0/02/02-01/T1",
            "task",
            json!({ "action": "Completed tasks: move to a done list, then write src/complete.js" }),
        ),
    ]);
    assert!(cited_by_plan(&input, PLAN_02).is_empty());
}

#[test]
fn the_plans_text_is_its_designated_sections() {
    let cites = |plan_fields: Value, children: Vec<Value>| {
        let mut more = vec![entity(PLAN_02, "plan", plan_fields)];
        more.extend(children);
        let mut input = input(more);
        // The later entity replaces the bare plan 02-01.
        let bare = input
            .entities
            .iter()
            .position(|e| e.id == PLAN_02)
            .expect("plan 02-01");
        input.entities.remove(bare);
        cited_by_plan(&input, PLAN_02)
    };
    let fields = json!({ "phase": "v1.0/02" });
    for (field, place) in [
        ("objective", "the objective"),
        ("purpose", "the purpose"),
        ("output", "the output"),
    ] {
        let mut plan = fields.clone();
        plan[field] = json!("Implement D-01");
        assert_eq!(cites(plan, vec![]), ["v1.0/02/D-01"], "{place}");
    }
    for (kind, field) in [
        ("task", "action"),
        ("truth", "text"),
        ("checkpoint", "what_built"),
        ("artifact", "provides"),
        ("key_link", "via"),
        ("prohibition", "text"),
    ] {
        let child = entity(
            &format!("{PLAN_02}/X1"),
            kind,
            json!({ field: "Implement D-01" }),
        );
        assert_eq!(
            cites(fields.clone(), vec![child]),
            ["v1.0/02/D-01"],
            "{kind}.{field}"
        );
    }
    for field in ["context_refs", "verification", "success_criteria"] {
        let mut plan = fields.clone();
        plan[field] = json!("Implement D-01");
        assert!(cites(plan, vec![]).is_empty(), "{field}");
    }
    // A child of another plan is not this plan's text.
    let other = entity(
        &format!("{PLAN_01}/T1"),
        "task",
        json!({ "action": "Implement 02/D-01" }),
    );
    assert!(cites(fields, vec![other]).is_empty());
}
