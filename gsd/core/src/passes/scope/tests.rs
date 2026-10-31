use super::*;

fn entity(id: &str, kind: &str, fields: &[(&str, &str)]) -> PassEntity {
    PassEntity {
        id: id.into(),
        kind: kind.into(),
        fields: fields
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        ..Default::default()
    }
}

fn action(text: &str) -> PassInput {
    PassInput {
        entities: vec![entity("v/1/1-01/T1", "task", &[("action", text)])],
        ..Default::default()
    }
}

#[test]
fn scope_reduction_phrase_is_e312() {
    assert_eq!(SCOPE_REDUCTION.len(), 12);
    for phrase in SCOPE_REDUCTION {
        let found = scope_reduction(&action(&format!(
            "Write the module as a {phrase}, then stop."
        )));
        assert_eq!(found.len(), 1, "{phrase}: {found:?}");
        assert_eq!(found[0].code, "E312");
        assert_eq!(found[0].entity.as_deref(), Some("v/1/1-01/T1"));
        assert!(
            found[0].message.contains(&format!("\"{phrase}\"")),
            "{found:?}"
        );
    }
}

#[test]
fn a_phrase_is_matched_case_insensitively() {
    assert_eq!(scope_reduction(&action("Static For Now: a label")).len(), 1);
    assert_eq!(scope_reduction(&action("the API v1 route")).len(), 1);
}

#[test]
fn a_phrase_inside_a_word_is_not_e312() {
    for clean in [
        "Write v10 and rev1x, then v1_x",
        "Fill the placeholders table",
        "Deliver a nonplaceholder",
        "The skip for nowhere case",
        "Ship version 1 of the module",
    ] {
        assert!(scope_reduction(&action(clean)).is_empty(), "{clean}");
    }
}

#[test]
fn a_dotted_version_is_not_a_scope_cut() {
    for clean in [
        "Tag the v1.0 release in package.json",
        "Bump to v2.3.1",
        "Rename 3.v1 to 3.x",
    ] {
        assert!(scope_reduction(&action(clean)).is_empty(), "{clean}");
    }
    for hit in [
        "Ship v1 only.",
        "Ship v1. Then v1.0 follows",
        "the API v2, v2.1 later",
    ] {
        assert_eq!(scope_reduction(&action(hit)).len(), 1, "{hit}");
    }
}

#[test]
fn a_hit_that_cites_a_decision_of_the_phase_is_clean() {
    let mut input = action("Build the v1 only, as D-02 settles");
    input.entities.push(entity("v/1", "phase", &[]));
    input.entities.push(entity("v/1/D-02", "decision", &[]));
    input.entities.push(entity("v/1/1-01", "plan", &[]));
    // The plan is not yet in the phase: the citation names no decision of it.
    assert_eq!(scope_reduction(&input).len(), 1);
    input.edges.push(PassEdge {
        source: "v/1/1-01".into(),
        target: "v/1".into(),
        label: "phase".into(),
    });
    assert!(scope_reduction(&input).is_empty());
    input.entities[0]
        .fields
        .insert("action".into(), "v1 only, per D-20".into());
    assert_eq!(
        scope_reduction(&input).len(),
        1,
        "D-20 is no decision of the phase"
    );
}

fn row(source: &'static str, item: &str, covered: bool) -> Row {
    Row {
        source,
        item: item.into(),
        covered,
    }
}

/// The rows of the `gap` fixture of `arms.cjs` (REQ-01 and REQ-02 asked for, plan citing REQ-01 and D-01): the table
/// and the summary equal what `gsd-tools query check.gap-analysis-plan-post` printed, byte for byte.
#[test]
fn the_gap_table_is_gsds() {
    let golden: serde_json::Value = serde_json::from_str(include_str!(gsd_kit::snapshot!(
        "gate-cutover/gap-analysis-plan-post-enabled.json"
    )))
    .expect("the golden is JSON");
    let stdout: serde_json::Value =
        serde_json::from_str(golden["stdout"].as_str().expect("stdout")).expect("stdout is JSON");
    let mut rows = vec![
        row("CONTEXT.md", "D-01", true),
        row("REQUIREMENTS.md", "REQ-02", false),
        row("REQUIREMENTS.md", "REQ-01", true),
    ];
    sort_rows(&mut rows);
    assert_eq!(table(&rows), stdout["table"].as_str().expect("table"));
    assert_eq!(summary(&rows), stdout["summary"].as_str().expect("summary"));
}

#[test]
fn the_summary_and_an_empty_table_are_gsds() {
    let all = [
        row("CONTEXT.md", "D-01", true),
        row("CONTEXT.md", "D-02", true),
    ];
    assert_eq!(summary(&all), "✓ All 2 items covered by plans");
    assert_eq!(
        table(&[]),
        "## Post-Planning Gap Analysis\n\nNo requirements or decisions to check.\n"
    );
}

#[test]
fn rows_sort_by_source_then_natural_key() {
    let mut rows = vec![
        row("CONTEXT.md", "D-02", true),
        row("REQUIREMENTS.md", "REQ-10", true),
        row("REQUIREMENTS.md", "REQ-9", true),
        row("CONTEXT.md", "D-01", true),
    ];
    sort_rows(&mut rows);
    let order: Vec<&str> = rows.iter().map(|r| r.item.as_str()).collect();
    assert_eq!(order, ["REQ-9", "REQ-10", "D-01", "D-02"]);
}

fn planned_phase(text: &str) -> PassInput {
    let mut plan = entity("v/1/1-01", "plan", &[("objective", text)]);
    plan.values
        .insert("requirements".into(), serde_json::json!(["REQ-01"]));
    plan.fields.insert("requirements".into(), "REQ-01".into());
    let mut phase = entity("v/1", "phase", &[]);
    phase.values.insert(
        "requirements".into(),
        serde_json::json!(["REQ-01", "REQ-02"]),
    );
    PassInput {
        entities: vec![
            phase,
            plan,
            entity("v/1/D-01", "decision", &[("origin", "context")]),
            entity("v/1/D-02", "decision", &[("origin", "summary")]),
            entity("D-01", "decision", &[("origin", "context")]),
        ],
        ..Default::default()
    }
}

#[test]
fn an_uncovered_item_is_one_w317_on_the_phase() {
    let found = post_planning_gaps(&planned_phase("Implement D-01"));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, "W317");
    assert_eq!(found[0].entity.as_deref(), Some("v/1"));
    assert_eq!(
        found[0].message,
        "⚠ 1 of 3 items not covered by any plan: REQ-02"
    );
    assert!(
        found[0]
            .suggestion
            .as_deref()
            .is_some_and(|t| t.contains("| REQUIREMENTS.md | REQ-02 | ✗ Not covered |")),
        "{found:?}"
    );
}

#[test]
fn a_covered_phase_and_an_unplanned_phase_are_silent() {
    let mut input = planned_phase("Implement D-01 and REQ-02");
    assert!(post_planning_gaps(&input).is_empty());
    input.entities.retain(|e| e.kind != "plan");
    assert!(post_planning_gaps(&input).is_empty());
}
