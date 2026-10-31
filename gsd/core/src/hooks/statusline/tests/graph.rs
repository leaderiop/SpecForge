//! Ported from gsd-core v1.19.0 hooks/gsd-statusline.js (the state segment, read from the graph here), MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! The graph that replaces STATE.md, and the line built from it.

use serde_json::{Value, json};

use super::*;

/// A graph of the taskboard's shape: one milestone with four phases, `done` of them complete.
fn graph(done: usize, plans: &[(&str, &str)]) -> CommandGraph {
    let mut nodes = vec![GraphNode {
        id: "v1.0".into(),
        kind: "milestone".into(),
        fields: [
            ("state".to_string(), json!("in_progress")),
            ("name".to_string(), json!("Taskboard")),
            ("progress".to_string(), json!({"phases": 4, "phases_complete": done, "plans": 3, "plans_complete": 2, "percent": done * 25})),
        ]
        .into(),
        ..GraphNode::default()
    }];
    let names = ["scaffold", "persistence", "views", "polish"];
    for (i, name) in names.iter().enumerate() {
        let (status, counts) = plans.get(i).copied().unwrap_or(("not_started", "0/0"));
        nodes.push(GraphNode {
            id: format!("v1.0/0{}", i + 1),
            kind: "phase".into(),
            fields: [
                ("slug".to_string(), json!(name)),
                (
                    "status".to_string(),
                    json!(if i < done { "complete" } else { status }),
                ),
                (
                    "plans_complete".to_string(),
                    json!(if i < done { "1/1" } else { counts }),
                ),
            ]
            .into(),
            ..GraphNode::default()
        });
    }
    CommandGraph::new(nodes, Vec::new())
}

#[test]
fn the_state_segment_reads_the_graph() {
    let g = graph(1, &[("complete", "1/1"), ("planned", "0/2")]);
    let s = state_of(&g);
    assert_eq!(
        (
            s.milestone.as_str(),
            s.phase_name.as_str(),
            s.status.as_str(),
            s.percent,
            s.phase_num.as_str(),
            s.phase_total.as_str()
        ),
        ("v1.0", "persistence", "executing", Some(25.0), "1", "4")
    );
    assert_eq!(
        format_full(&s),
        "v1.0 Taskboard [██░░░░░░░░] 25% · executing · persistence (1/4)"
    );
    assert_eq!(format_compact(&s), "v1.0 · P1/4 · executing");
    // every phase complete: the milestone is done and no phase is named
    let done = state_of(&graph(4, &[]));
    assert_eq!(
        format_full(&done),
        "v1.0 Taskboard [██████████] 100% · milestone complete"
    );
    // no milestone: no segment
    assert_eq!(format_full(&state_of(&CommandGraph::default())), "");
}

#[test]
fn the_line_names_the_workstream_and_the_last_command() {
    let g = graph(1, &[("complete", "1/1"), ("planned", "0/2")]);
    let mut f = Facts {
        host: "claude-code".into(),
        status: Some(status(json!("Claude"), None, None, Value::Null)),
        graph: Some(&g),
        ..Facts::default()
    };
    f.project_state
        .insert("active_workstream".into(), json!("billing"));
    f.session_state = json!({"workflow": {"command": "/gsd-plan-phase"}})
        .as_object()
        .cloned();
    let mut o = opts("end", false, false);
    let out = plain(&statusline(&f, &o).statusline.expect("a line"));
    assert!(
        out.contains("persistence (1/4) · ws billing") && !out.contains("last:"),
        "{out}"
    );
    o.last_command = true;
    let out = plain(&statusline(&f, &o).statusline.expect("a line"));
    assert!(out.ends_with("myproject │ last: /gsd-plan-phase"), "{out}");
}

#[test]
fn a_status_without_a_window_writes_no_metrics() {
    let f = Facts {
        host: "claude-code".into(),
        session: Some("s1".into()),
        status: Some(status(json!("Claude"), None, None, Value::Null)),
        ..Facts::default()
    };
    assert!(statusline(&f, &opts("end", false, false)).state.is_none());
    let mut with = f.clone();
    with.status = Some(status(
        json!("Claude"),
        Some(200_000),
        Some(70.0),
        Value::Null,
    ));
    with.session = None;
    assert!(
        statusline(&with, &opts("end", false, false))
            .state
            .is_none(),
        "no session, nowhere to write"
    );
}

/// A session state with no `workflow` key is the usual one: the line must not index into it.
#[test]
fn a_session_without_a_marker_has_no_last_command() {
    let mut f = Facts {
        host: "claude-code".into(),
        status: Some(status(json!("Claude"), None, None, Value::Null)),
        ..Facts::default()
    };
    f.session_state = json!({"context": {"remaining_percentage": 50}})
        .as_object()
        .cloned();
    let mut o = opts("end", false, false);
    o.last_command = true;
    let out = plain(&statusline(&f, &o).statusline.expect("a line"));
    assert!(!out.contains("last:"), "{out}");
}
