use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue};
use super::{
    bulk_status, feature_dependents, feature_impact, journey_coverage, list_entities,
    milestone_completion, persona_features, channel_features, project_health,
    ListFilter,
};

fn make_node(id: &str, kind: &str) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind { raw: Sym::new(kind) },
        title: Some(id.to_string()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: Sym::new("test.spec"),
            start_line: 1,
            start_col: 1,
            end_line: 1,
            end_col: 1,
        },
    }
}

fn make_node_with_fields(id: &str, kind: &str, fields: &[(&str, &str)]) -> Node {
    let mut fm = FieldMap::new();
    for (k, v) in fields {
        fm.push(Sym::new(k), FieldValue::String(v.to_string()));
    }
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind { raw: Sym::new(kind) },
        title: Some(id.to_string()),
        fields: fm,
        source_span: SourceSpan {
            file: Sym::new("test.spec"),
            start_line: 1,
            start_col: 1,
            end_line: 1,
            end_col: 1,
        },
    }
}

fn make_edge(source: &str, target: &str, label: &str) -> Edge {
    Edge {
        source: Sym::new(source),
        target: Sym::new(target),
        label: Sym::new(label),
    }
}

// ---------------------------------------------------------------------------
// list_entities
// ---------------------------------------------------------------------------

#[test]
fn list_entities_filters_by_kind() {
    let mut g = Graph::new();
    g.add_node(make_node("f1", "feature"));
    g.add_node(make_node("b1", "behavior"));

    let result = list_entities(&g, &ListFilter { kind: "feature".into(), ..Default::default() });
    assert_eq!(result.total, 1);
    assert_eq!(result.entities[0].id, "f1");
}

#[test]
fn list_entities_filters_by_status() {
    let mut g = Graph::new();
    g.add_node(make_node_with_fields("f1", "feature", &[("status", "done")]));
    g.add_node(make_node_with_fields("f2", "feature", &[("status", "draft")]));

    let result = list_entities(&g, &ListFilter {
        kind: "feature".into(),
        status: Some("done".into()),
        ..Default::default()
    });
    assert_eq!(result.total, 1);
    assert_eq!(result.entities[0].id, "f1");
}

#[test]
fn list_entities_filters_by_priority() {
    let mut g = Graph::new();
    g.add_node(make_node_with_fields("f1", "feature", &[("priority", "high")]));
    g.add_node(make_node_with_fields("f2", "feature", &[("priority", "low")]));

    let result = list_entities(&g, &ListFilter {
        kind: "feature".into(),
        priority: Some("high".into()),
        ..Default::default()
    });
    assert_eq!(result.total, 1);
    assert_eq!(result.entities[0].id, "f1");
}

#[test]
fn list_entities_pagination() {
    let mut g = Graph::new();
    for i in 0..5 {
        g.add_node(make_node(&format!("f{i}"), "feature"));
    }

    let result = list_entities(&g, &ListFilter {
        kind: "feature".into(),
        offset: Some(1),
        limit: Some(2),
        ..Default::default()
    });
    assert_eq!(result.total, 5);
    assert_eq!(result.entities.len(), 2);
    assert_eq!(result.entities[0].id, "f1");
    assert_eq!(result.entities[1].id, "f2");
}

#[test]
fn list_entities_sorted_by_id() {
    let mut g = Graph::new();
    g.add_node(make_node("z_feature", "feature"));
    g.add_node(make_node("a_feature", "feature"));

    let result = list_entities(&g, &ListFilter { kind: "feature".into(), ..Default::default() });
    assert_eq!(result.entities[0].id, "a_feature");
    assert_eq!(result.entities[1].id, "z_feature");
}

#[test]
fn list_entities_empty_graph() {
    let g = Graph::new();
    let result = list_entities(&g, &ListFilter { kind: "feature".into(), ..Default::default() });
    assert_eq!(result.total, 0);
    assert!(result.entities.is_empty());
}

#[test]
fn list_entities_includes_edge_counts() {
    let mut g = Graph::new();
    g.add_node(make_node("f1", "feature"));
    g.add_node(make_node("m1", "milestone"));
    g.add_edge(make_edge("m1", "f1", "features"));

    let result = list_entities(&g, &ListFilter { kind: "feature".into(), ..Default::default() });
    assert_eq!(result.entities[0].incoming_edges, 1);
    assert_eq!(result.entities[0].outgoing_edges, 0);
}

// ---------------------------------------------------------------------------
// milestone_completion
// ---------------------------------------------------------------------------

#[test]
fn milestone_completion_basic() {
    let mut g = Graph::new();
    g.add_node(make_node_with_fields("ms1", "milestone", &[("status", "active")]));
    g.add_node(make_node_with_fields("f1", "feature", &[("status", "done")]));
    g.add_node(make_node_with_fields("f2", "feature", &[("status", "draft")]));
    g.add_edge(make_edge("ms1", "f1", "features"));
    g.add_edge(make_edge("ms1", "f2", "features"));

    let mc = milestone_completion(&g, "ms1").unwrap();
    assert_eq!(mc.total_features, 2);
    assert_eq!(mc.done_features, 1);
    assert!((mc.completion_pct - 50.0).abs() < 0.01);
    assert_eq!(mc.status.as_deref(), Some("active"));
}

#[test]
fn milestone_completion_returns_none_for_missing() {
    let g = Graph::new();
    assert!(milestone_completion(&g, "nonexistent").is_none());
}

#[test]
fn milestone_completion_returns_none_for_wrong_kind() {
    let mut g = Graph::new();
    g.add_node(make_node("f1", "feature"));
    assert!(milestone_completion(&g, "f1").is_none());
}

#[test]
fn milestone_completion_no_features() {
    let mut g = Graph::new();
    g.add_node(make_node("ms1", "milestone"));

    let mc = milestone_completion(&g, "ms1").unwrap();
    assert_eq!(mc.total_features, 0);
    assert!((mc.completion_pct - 0.0).abs() < 0.01);
}

// ---------------------------------------------------------------------------
// journey_coverage
// ---------------------------------------------------------------------------

#[test]
fn journey_coverage_basic() {
    let mut g = Graph::new();
    g.add_node(make_node_with_fields("j1", "journey", &[("persona", "dev")]));
    g.add_node(make_node("f1", "feature"));
    g.add_node(make_node("f2", "feature"));
    g.add_node(make_node("mod1", "module"));
    g.add_edge(make_edge("j1", "f1", "features"));
    g.add_edge(make_edge("j1", "f2", "features"));
    g.add_edge(make_edge("mod1", "f1", "features"));

    let jc = journey_coverage(&g, "j1").unwrap();
    assert_eq!(jc.total_features, 2);
    assert_eq!(jc.covered_by_modules, 1);
    assert!((jc.coverage_pct - 50.0).abs() < 0.01);
    assert_eq!(jc.persona.as_deref(), Some("dev"));
}

#[test]
fn journey_coverage_returns_none_for_missing() {
    let g = Graph::new();
    assert!(journey_coverage(&g, "nonexistent").is_none());
}

#[test]
fn journey_coverage_returns_none_for_wrong_kind() {
    let mut g = Graph::new();
    g.add_node(make_node("f1", "feature"));
    assert!(journey_coverage(&g, "f1").is_none());
}

// ---------------------------------------------------------------------------
// feature_impact
// ---------------------------------------------------------------------------

#[test]
fn feature_impact_basic() {
    let mut g = Graph::new();
    g.add_node(make_node("f1", "feature"));
    g.add_node(make_node("f2", "feature"));
    g.add_node(make_node("j1", "journey"));
    g.add_node(make_node("ms1", "milestone"));
    g.add_node(make_node("mod1", "module"));
    g.add_edge(make_edge("j1", "f1", "features"));
    g.add_edge(make_edge("ms1", "f1", "features"));
    g.add_edge(make_edge("mod1", "f1", "features"));
    g.add_edge(make_edge("f1", "f2", "depends_on"));

    let fi = feature_impact(&g, "f1").unwrap();
    assert_eq!(fi.referenced_by_journeys, vec!["j1"]);
    assert_eq!(fi.referenced_by_milestones, vec!["ms1"]);
    assert_eq!(fi.referenced_by_modules, vec!["mod1"]);
    assert_eq!(fi.depends_on, vec!["f2"]);
}

#[test]
fn feature_impact_returns_none_for_missing() {
    let g = Graph::new();
    assert!(feature_impact(&g, "nonexistent").is_none());
}

#[test]
fn feature_impact_returns_none_for_wrong_kind() {
    let mut g = Graph::new();
    g.add_node(make_node("j1", "journey"));
    assert!(feature_impact(&g, "j1").is_none());
}

#[test]
fn feature_impact_tracks_depended_on_by() {
    let mut g = Graph::new();
    g.add_node(make_node("f1", "feature"));
    g.add_node(make_node("f2", "feature"));
    g.add_edge(make_edge("f2", "f1", "depends_on"));

    let fi = feature_impact(&g, "f1").unwrap();
    assert_eq!(fi.depended_on_by, vec!["f2"]);
}

// ---------------------------------------------------------------------------
// feature_dependents
// ---------------------------------------------------------------------------

#[test]
fn feature_dependents_basic() {
    let mut g = Graph::new();
    g.add_node(make_node("f1", "feature"));
    g.add_node(make_node("f2", "feature"));
    g.add_node(make_node("f3", "feature"));
    g.add_edge(make_edge("f2", "f1", "depends_on"));
    g.add_edge(make_edge("f3", "f1", "depends_on"));

    let deps = feature_dependents(&g, "f1").unwrap();
    assert_eq!(deps.len(), 2);
    assert!(deps.contains(&"f2".to_string()));
    assert!(deps.contains(&"f3".to_string()));
}

#[test]
fn feature_dependents_returns_none_for_missing() {
    let g = Graph::new();
    assert!(feature_dependents(&g, "nonexistent").is_none());
}

// ---------------------------------------------------------------------------
// persona_features
// ---------------------------------------------------------------------------

#[test]
fn persona_features_via_field() {
    let mut g = Graph::new();
    g.add_node(make_node("p1", "persona"));
    g.add_node(make_node_with_fields("j1", "journey", &[("persona", "p1")]));
    g.add_node(make_node("f1", "feature"));
    g.add_edge(make_edge("j1", "f1", "features"));

    let feats = persona_features(&g, "p1").unwrap();
    assert_eq!(feats, vec!["f1"]);
}

#[test]
fn persona_features_via_edge() {
    let mut g = Graph::new();
    g.add_node(make_node("p1", "persona"));
    g.add_node(make_node("j1", "journey"));
    g.add_node(make_node("f1", "feature"));
    g.add_edge(make_edge("j1", "p1", "persona"));
    g.add_edge(make_edge("j1", "f1", "features"));

    let feats = persona_features(&g, "p1").unwrap();
    assert_eq!(feats, vec!["f1"]);
}

#[test]
fn persona_features_returns_none_for_missing() {
    let g = Graph::new();
    assert!(persona_features(&g, "nonexistent").is_none());
}

#[test]
fn persona_features_returns_none_for_wrong_kind() {
    let mut g = Graph::new();
    g.add_node(make_node("f1", "feature"));
    assert!(persona_features(&g, "f1").is_none());
}

// ---------------------------------------------------------------------------
// channel_features
// ---------------------------------------------------------------------------

#[test]
fn channel_features_basic() {
    let mut g = Graph::new();
    g.add_node(make_node("ch1", "channel"));
    g.add_node(make_node("j1", "journey"));
    g.add_node(make_node("f1", "feature"));
    g.add_edge(make_edge("j1", "ch1", "channels"));
    g.add_edge(make_edge("j1", "f1", "features"));

    let feats = channel_features(&g, "ch1").unwrap();
    assert_eq!(feats, vec!["f1"]);
}

#[test]
fn channel_features_returns_none_for_missing() {
    let g = Graph::new();
    assert!(channel_features(&g, "nonexistent").is_none());
}

#[test]
fn channel_features_returns_none_for_wrong_kind() {
    let mut g = Graph::new();
    g.add_node(make_node("f1", "feature"));
    assert!(channel_features(&g, "f1").is_none());
}

#[test]
fn channel_features_deduplicates() {
    let mut g = Graph::new();
    g.add_node(make_node("ch1", "channel"));
    g.add_node(make_node("j1", "journey"));
    g.add_node(make_node("j2", "journey"));
    g.add_node(make_node("f1", "feature"));
    g.add_edge(make_edge("j1", "ch1", "channels"));
    g.add_edge(make_edge("j2", "ch1", "channels"));
    g.add_edge(make_edge("j1", "f1", "features"));
    g.add_edge(make_edge("j2", "f1", "features"));

    let feats = channel_features(&g, "ch1").unwrap();
    assert_eq!(feats, vec!["f1"]);
}

// ---------------------------------------------------------------------------
// bulk_status
// ---------------------------------------------------------------------------

#[test]
fn bulk_status_aggregates_by_kind() {
    let mut g = Graph::new();
    g.add_node(make_node_with_fields("f1", "feature", &[("status", "done")]));
    g.add_node(make_node_with_fields("f2", "feature", &[("status", "done")]));
    g.add_node(make_node_with_fields("f3", "feature", &[("status", "draft")]));
    g.add_node(make_node_with_fields("ms1", "milestone", &[("status", "active")]));

    let results = bulk_status(&g);
    let feat = results.iter().find(|r| r.kind == "feature").unwrap();
    assert_eq!(feat.total, 3);
    let done = feat.by_status.iter().find(|s| s.status == "done").unwrap();
    assert_eq!(done.count, 2);

    let ms = results.iter().find(|r| r.kind == "milestone").unwrap();
    assert_eq!(ms.total, 1);
}

#[test]
fn bulk_status_empty_graph() {
    let g = Graph::new();
    let results = bulk_status(&g);
    assert!(results.is_empty());
}

#[test]
fn bulk_status_missing_status_field() {
    let mut g = Graph::new();
    g.add_node(make_node("f1", "feature"));

    let results = bulk_status(&g);
    let feat = results.iter().find(|r| r.kind == "feature").unwrap();
    assert_eq!(feat.by_status[0].status, "(none)");
}

// ---------------------------------------------------------------------------
// project_health
// ---------------------------------------------------------------------------

#[test]
fn project_health_empty_graph() {
    let g = Graph::new();
    let report = project_health(&g);
    assert!((report.score.overall - 100.0).abs() < 0.01);
}

#[test]
fn project_health_counts_entities() {
    let mut g = Graph::new();
    g.add_node(make_node("f1", "feature"));
    g.add_node(make_node("f2", "feature"));
    g.add_node(make_node("j1", "journey"));

    let report = project_health(&g);
    let feat_count = report.entity_counts.iter().find(|c| c.kind == "feature").unwrap();
    assert_eq!(feat_count.count, 2);
    let journey_count = report.entity_counts.iter().find(|c| c.kind == "journey").unwrap();
    assert_eq!(journey_count.count, 1);
}

#[test]
fn project_health_detects_orphans() {
    let mut g = Graph::new();
    g.add_node(make_node("f1", "feature"));
    g.add_node(make_node("f2", "feature"));
    g.add_node(make_node("j1", "journey"));
    g.add_edge(make_edge("j1", "f1", "features"));

    let report = project_health(&g);
    let feat_orphans = report.orphan_counts.iter().find(|c| c.kind == "feature").unwrap();
    assert_eq!(feat_orphans.orphans, 1);
    assert_eq!(feat_orphans.total, 2);
}

#[test]
fn project_health_completeness_tracks_status() {
    let mut g = Graph::new();
    g.add_node(make_node_with_fields("f1", "feature", &[("status", "done")]));
    g.add_node(make_node("f2", "feature"));

    let report = project_health(&g);
    assert_eq!(report.completeness.features_with_status, 1);
    assert_eq!(report.completeness.features_total, 2);
}

#[test]
fn project_health_completeness_tracks_milestones_with_features() {
    let mut g = Graph::new();
    g.add_node(make_node("ms1", "milestone"));
    g.add_node(make_node("ms2", "milestone"));
    g.add_node(make_node("f1", "feature"));
    g.add_edge(make_edge("ms1", "f1", "features"));

    let report = project_health(&g);
    assert_eq!(report.completeness.milestones_with_features, 1);
    assert_eq!(report.completeness.milestones_total, 2);
}
