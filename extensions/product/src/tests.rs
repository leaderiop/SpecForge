use crate::queries::*;
use specforge_extension_sdk::prelude::{CommandGraph, CommandInput, GraphEdge, GraphNode};

/// A graph built entity by entity.
#[derive(Default)]
struct G {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}

impl G {
    fn node(mut self, id: &str, kind: &str, fields: &[(&str, &str)]) -> Self {
        self.nodes.push(GraphNode {
            id: id.to_string(),
            kind: kind.to_string(),
            title: Some(id.to_string()),
            fields: fields
                .iter()
                .map(|(k, v)| (k.to_string(), serde_json::json!(v)))
                .collect(),
        });
        self
    }

    fn n(self, id: &str, kind: &str) -> Self {
        self.node(id, kind, &[])
    }

    fn edge(mut self, source: &str, target: &str, label: &str) -> Self {
        self.edges.push(GraphEdge {
            source: source.to_string(),
            target: target.to_string(),
            label: label.to_string(),
        });
        self
    }

    fn build(self) -> CommandGraph {
        CommandGraph::new(self.nodes, self.edges)
    }
}

fn of(kind: &str) -> ListFilter<'_> {
    ListFilter {
        kind,
        ..Default::default()
    }
}

// ── list_entities ──────────────────────────────────────────────────────────

#[test]
fn list_entities_filters_by_kind() {
    let g = G::default().n("f1", "feature").n("b1", "behavior").build();
    let result = list_entities(&g, &of("feature"));
    assert_eq!(result.total, 1);
    assert_eq!(result.entities[0].id, "f1");
}

#[test]
fn list_entities_filters_by_status_and_priority() {
    let g = G::default()
        .node("f1", "feature", &[("status", "done"), ("priority", "high")])
        .node(
            "f2",
            "feature",
            &[("status", "draft"), ("priority", "high")],
        )
        .node("f3", "feature", &[("status", "done"), ("priority", "low")])
        .build();
    let done = list_entities(
        &g,
        &ListFilter {
            status: Some("done"),
            ..of("feature")
        },
    );
    assert_eq!(done.total, 2);
    let done_high = list_entities(
        &g,
        &ListFilter {
            status: Some("done"),
            priority: Some("high"),
            ..of("feature")
        },
    );
    assert_eq!(done_high.total, 1);
    assert_eq!(done_high.entities[0].id, "f1");
}

#[test]
fn list_entities_pages_after_counting() {
    let mut g = G::default();
    for i in 0..5 {
        g = g.n(&format!("f{i}"), "feature");
    }
    let result = list_entities(
        &g.build(),
        &ListFilter {
            offset: Some(1),
            limit: Some(2),
            ..of("feature")
        },
    );
    assert_eq!(result.total, 5);
    let ids: Vec<&str> = result.entities.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["f1", "f2"]);
}

#[test]
fn list_entities_sorted_by_id_with_edge_counts() {
    let g = G::default()
        .n("z_feature", "feature")
        .n("a_feature", "feature")
        .n("m1", "milestone")
        .edge("m1", "z_feature", "features")
        .build();
    let result = list_entities(&g, &of("feature"));
    assert_eq!(result.entities[0].id, "a_feature");
    assert_eq!(result.entities[1].id, "z_feature");
    assert_eq!(result.entities[1].incoming_edges, 1);
    assert_eq!(result.entities[1].outgoing_edges, 0);
    assert_eq!(
        list_entities(&CommandGraph::default(), &of("feature")).total,
        0
    );
}

// ── milestones and journeys ────────────────────────────────────────────────

#[test]
fn milestone_completion_counts_done_features_in_declared_order() {
    let mut g = G::default()
        .node("f2", "feature", &[("status", "done")])
        .node("f1", "feature", &[("status", "draft")])
        .edge("ms1", "f1", "features")
        .edge("ms1", "f2", "features");
    g.nodes.push(GraphNode {
        id: "ms1".into(),
        kind: "milestone".into(),
        title: None,
        fields: [
            ("status".to_string(), serde_json::json!("active")),
            ("features".to_string(), serde_json::json!(["f2", "f1"])),
        ]
        .into_iter()
        .collect(),
    });
    let mc = milestone_completion(&g.build(), "ms1").unwrap();
    assert_eq!(mc.total_features, 2);
    assert_eq!(mc.done_features, 1);
    assert!((mc.completion_pct - 50.0).abs() < 0.01);
    assert_eq!(mc.status.as_deref(), Some("active"));
    let ids: Vec<&str> = mc.features.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["f2", "f1"]);
}

#[test]
fn milestone_completion_of_a_milestone_only() {
    let g = G::default()
        .n("f1", "feature")
        .n("ms1", "milestone")
        .build();
    assert!(milestone_completion(&g, "nonexistent").is_none());
    assert!(milestone_completion(&g, "f1").is_none());
    let empty = milestone_completion(&g, "ms1").unwrap();
    assert_eq!(empty.total_features, 0);
    assert!(empty.completion_pct.abs() < 0.01);
}

#[test]
fn journey_coverage_counts_features_some_module_contains() {
    let g = G::default()
        .node("j1", "journey", &[("persona", "dev")])
        .n("f1", "feature")
        .n("f2", "feature")
        .n("mod1", "module")
        .edge("j1", "f1", "features")
        .edge("j1", "f2", "features")
        .edge("mod1", "f1", "features")
        .build();
    let jc = journey_coverage(&g, "j1").unwrap();
    assert_eq!(jc.total_features, 2);
    assert_eq!(jc.covered_by_modules, 1);
    assert!((jc.coverage_pct - 50.0).abs() < 0.01);
    assert_eq!(jc.persona.as_deref(), Some("dev"));
    assert!(journey_coverage(&g, "f1").is_none());
    assert!(journey_coverage(&g, "nonexistent").is_none());
}

// ── features ───────────────────────────────────────────────────────────────

#[test]
fn feature_impact_groups_references_by_kind() {
    let g = G::default()
        .n("f1", "feature")
        .n("f2", "feature")
        .n("f3", "feature")
        .n("j1", "journey")
        .n("ms1", "milestone")
        .n("mod1", "module")
        .edge("j1", "f1", "features")
        .edge("ms1", "f1", "features")
        .edge("mod1", "f1", "features")
        .edge("f1", "f2", "depends_on")
        .edge("f3", "f1", "depends_on")
        .build();
    let fi = feature_impact(&g, "f1").unwrap();
    assert_eq!(fi.referenced_by_journeys, ["j1"]);
    assert_eq!(fi.referenced_by_milestones, ["ms1"]);
    assert_eq!(fi.referenced_by_modules, ["mod1"]);
    assert_eq!(fi.depends_on, ["f2"]);
    assert_eq!(fi.depended_on_by, ["f3"]);
    assert!(feature_impact(&g, "j1").is_none());
    assert!(feature_impact(&g, "nonexistent").is_none());
}

#[test]
fn feature_dependents_are_the_depends_on_sources() {
    let g = G::default()
        .n("f1", "feature")
        .n("f2", "feature")
        .n("f3", "feature")
        .edge("f2", "f1", "depends_on")
        .edge("f3", "f1", "depends_on")
        .build();
    assert_eq!(feature_dependents(&g, "f1").unwrap(), ["f2", "f3"]);
    assert!(feature_dependents(&g, "nonexistent").is_none());
}

#[test]
fn persona_features_follow_the_field_or_the_reference() {
    let by_field = G::default()
        .n("p1", "persona")
        .node("j1", "journey", &[("persona", "p1")])
        .n("f1", "feature")
        .edge("j1", "f1", "features")
        .build();
    assert_eq!(persona_features(&by_field, "p1").unwrap(), ["f1"]);
    let by_edge = G::default()
        .n("p1", "persona")
        .n("j1", "journey")
        .n("f1", "feature")
        .edge("j1", "p1", "persona")
        .edge("j1", "f1", "features")
        .build();
    assert_eq!(persona_features(&by_edge, "p1").unwrap(), ["f1"]);
    assert!(persona_features(&by_edge, "f1").is_none());
    assert!(persona_features(&by_edge, "nonexistent").is_none());
}

#[test]
fn channel_features_deduplicate_across_journeys() {
    let g = G::default()
        .n("ch1", "channel")
        .n("j1", "journey")
        .n("j2", "journey")
        .n("f1", "feature")
        .edge("j1", "ch1", "channels")
        .edge("j2", "ch1", "channels")
        .edge("j1", "f1", "features")
        .edge("j2", "f1", "features")
        .build();
    assert_eq!(channel_features(&g, "ch1").unwrap(), ["f1"]);
    assert!(channel_features(&g, "f1").is_none());
    assert!(channel_features(&g, "nonexistent").is_none());
}

// ── project-wide ───────────────────────────────────────────────────────────

#[test]
fn bulk_status_aggregates_by_kind() {
    let g = G::default()
        .node("f1", "feature", &[("status", "done")])
        .node("f2", "feature", &[("status", "done")])
        .n("f3", "feature")
        .node("ms1", "milestone", &[("status", "active")])
        .build();
    let results = bulk_status(&g);
    let feat = results.iter().find(|r| r.kind == "feature").unwrap();
    assert_eq!(feat.total, 3);
    let counts: Vec<(&str, usize)> = feat
        .by_status
        .iter()
        .map(|s| (s.status.as_str(), s.count))
        .collect();
    assert_eq!(counts, [("(none)", 1), ("done", 2)]);
    assert_eq!(
        results
            .iter()
            .find(|r| r.kind == "milestone")
            .unwrap()
            .total,
        1
    );
    assert!(bulk_status(&CommandGraph::default()).is_empty());
}

#[test]
fn project_health_scores_coverage_and_completeness() {
    assert!((project_health(&CommandGraph::default()).score.overall - 100.0).abs() < 0.01);
    let g = G::default()
        .node("f1", "feature", &[("status", "done")])
        .n("f2", "feature")
        .n("j1", "journey")
        .n("ms1", "milestone")
        .n("ms2", "milestone")
        .edge("j1", "f1", "features")
        .edge("ms1", "f1", "features")
        .build();
    let report = project_health(&g);
    let count = |kind: &str| {
        report
            .entity_counts
            .iter()
            .find(|c| c.kind == kind)
            .unwrap()
            .count
    };
    assert_eq!((count("feature"), count("journey")), (2, 1));
    let orphans = report
        .orphan_counts
        .iter()
        .find(|c| c.kind == "feature")
        .unwrap();
    assert_eq!((orphans.orphans, orphans.total), (1, 2));
    assert_eq!(report.completeness.features_with_status, 1);
    assert_eq!(report.completeness.features_total, 2);
    assert_eq!(report.completeness.milestones_with_features, 1);
    assert_eq!(report.completeness.milestones_total, 2);
}

// ── commands ───────────────────────────────────────────────────────────────

fn input(args: serde_json::Value, graph: CommandGraph) -> CommandInput {
    CommandInput {
        args: args.as_object().unwrap().clone(),
        cwd: "/p".into(),
        graph,
    }
}

fn sample() -> CommandGraph {
    G::default()
        .node(
            "f1",
            "feature",
            &[("status", "proposed"), ("priority", "high")],
        )
        .node("f2", "feature", &[("status", "done")])
        .build()
}

#[test]
fn a_list_command_renders_human_by_default_and_json_on_request() {
    let human = crate::commands::run(
        "cmd__product_features",
        &input(serde_json::json!({}), sample()),
    )
    .unwrap();
    assert_eq!(human.exit_code, 0);
    assert_eq!(
        human.stdout,
        "2 feature entities (showing 2):\n  f1 f1 [proposed] pri=high in=0 out=0\n  f2 f2 [done] pri=- in=0 out=0\n"
    );
    let json = crate::commands::run(
        "cmd__product_features",
        &input(
            serde_json::json!({"format": "json", "status": "done"}),
            sample(),
        ),
    )
    .unwrap();
    let value: serde_json::Value = serde_json::from_str(&json.stdout).unwrap();
    assert_eq!(value["total"], 1);
    assert_eq!(value["entities"][0]["id"], "f2");
    assert!(value["entities"][0].get("priority").is_none());
}

#[test]
fn a_query_about_a_missing_entity_fails_on_stderr() {
    let out = crate::commands::run(
        "cmd__product_milestone_completion",
        &input(serde_json::json!({"milestone": "nope"}), sample()),
    )
    .unwrap();
    assert_eq!(out.exit_code, 1);
    assert_eq!(out.stdout, "");
    assert_eq!(out.stderr, "milestone 'nope' not found\n");
}

#[test]
fn an_id_list_says_none_when_empty() {
    let out = crate::commands::run(
        "cmd__product_feature_dependents",
        &input(serde_json::json!({"feature": "f1"}), sample()),
    )
    .unwrap();
    assert_eq!(out.stdout, "Features depending on 'f1':\n  (none)\n");
}

#[test]
fn every_declared_command_has_its_export() {
    let surfaces: serde_json::Value =
        serde_json::from_slice(include_bytes!("describe_surfaces.json")).unwrap();
    let commands = surfaces["items"][0]["commands"].as_array().unwrap();
    assert_eq!(commands.len(), 17);
    for command in commands {
        let export = command["export"].as_str().unwrap();
        assert_eq!(
            export,
            format!("cmd__product_{}", command["id"].as_str().unwrap())
        );
        let args = serde_json::json!({"format": "json", "milestone": "x", "journey": "x",
            "feature": "x", "persona": "x", "channel": "x"});
        assert!(
            crate::commands::run(export, &input(args, sample())).is_some(),
            "{export} is declared but not exported"
        );
    }
    assert!(
        crate::commands::run("cmd__product_nope", &input(serde_json::json!({}), sample()))
            .is_none()
    );
}
