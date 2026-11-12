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

fn edge(source: &str, target: &str, label: &str) -> PassEdge {
    PassEdge {
        source: source.into(),
        target: target.into(),
        label: label.into(),
    }
}

#[test]
fn a_hit_its_listed_decision_says_is_clean() {
    let mut input = action("Build the v1 only, as D-02 settles");
    input.entities.push(entity("v/1", "phase", &[]));
    input.entities.push(entity("v/1/1-01", "plan", &[]));
    input.entities.push(entity(
        "v/1/D-02",
        "decision",
        &[("text", "Ship the v1 only")],
    ));
    input.entities.push(entity(
        "v/1/D-03",
        "decision",
        &[("text", "Ship every route at once")],
    ));
    input.edges.push(edge("v/1/1-01", "v/1", "phase"));
    // A decision the plan does not list never excuses, whatever it says.
    assert_eq!(scope_reduction(&input).len(), 1);
    input.edges.push(edge("v/1/1-01", "v/1/D-03", "decisions"));
    assert_eq!(scope_reduction(&input).len(), 1, "D-03 does not say v1");
    input.edges.push(edge("v/1/1-01", "v/1/D-02", "decisions"));
    assert!(scope_reduction(&input).is_empty());
    // Each hit is excused on its own.
    input.entities[0].fields.insert(
        "action".into(),
        "Build the v1 only; dynamic pricing is a future enhancement".into(),
    );
    let found = scope_reduction(&input);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].message.contains("\"future enhancement\""),
        "{found:?}"
    );
}

/// Upstream's own red flag (`agents/gsd-plan-checker.md:437-438`, "ALWAYS BLOCKER"): the action cites the decision it
/// cuts, and the decision never says "v1".
#[test]
fn a_decision_that_does_not_say_the_phrase_does_not_excuse_it() {
    let mut input = action(
        "D-26 cost references (v1, static labels). NOT wired to the price table: dynamic pricing is a future \
         enhancement",
    );
    input.entities.push(entity("v/1", "phase", &[]));
    input.entities.push(entity("v/1/1-01", "plan", &[]));
    input.entities.push(entity(
        "v/1/D-26",
        "decision",
        &[(
            "text",
            "Config shows costs calculated in impulses from the price table",
        )],
    ));
    input.edges.push(edge("v/1/1-01", "v/1", "phase"));
    input.edges.push(edge("v/1/1-01", "v/1/D-26", "decisions"));
    let found = scope_reduction(&input);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        found[0].message,
        "task 'v/1/1-01/T1' reduces scope: its action says \"v1\""
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

/// Phase `v/1` asking for REQ-01 and REQ-02, with one plan that lists `requirements` and `decisions` (ids) and
/// whose objective is `text`.
fn planned_phase(text: &str, requirements: &[&str], decisions: &[&str]) -> PassInput {
    let mut edges = vec![
        edge("v/1", "REQ-01", "requirements"),
        edge("v/1", "REQ-02", "requirements"),
    ];
    edges.extend(
        requirements
            .iter()
            .map(|r| edge("v/1/1-01", r, "requirements")),
    );
    edges.extend(decisions.iter().map(|d| edge("v/1/1-01", d, "decisions")));
    PassInput {
        entities: vec![
            entity("v/1", "phase", &[]),
            entity("v/1/1-01", "plan", &[("objective", text)]),
            entity("v/1/D-01", "decision", &[("origin", "context")]),
            entity("v/1/D-02", "decision", &[("origin", "summary")]),
            entity("D-01", "decision", &[("origin", "context")]),
        ],
        edges,
        ..Default::default()
    }
}

#[test]
fn an_uncovered_item_is_one_w317_on_the_phase() {
    let found = post_planning_gaps(&planned_phase(
        "Implement REQ-02",
        &["REQ-01"],
        &["v/1/D-01"],
    ));
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
    // A decision named only in prose is not covered.
    let found = post_planning_gaps(&planned_phase("Implement D-01", &["REQ-01", "REQ-02"], &[]));
    assert_eq!(
        found[0].message,
        "⚠ 1 of 3 items not covered by any plan: D-01"
    );
}

#[test]
fn a_covered_phase_and_an_unplanned_phase_are_silent() {
    let mut input = planned_phase("Build it", &["REQ-01", "REQ-02"], &["v/1/D-01"]);
    assert!(post_planning_gaps(&input).is_empty());
    input.entities.retain(|e| e.kind != "plan");
    assert!(post_planning_gaps(&input).is_empty());
}
