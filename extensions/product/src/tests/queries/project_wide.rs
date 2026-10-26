use crate::queries::*;
use crate::tests::G;
use specforge_extension_sdk::prelude::CommandGraph;
use specforge_test::prelude::*;

#[test]
fn bulk_status_aggregates_by_kind() {
    let g = G::default()
        .node("f1", "feature", &[("status", "done")])
        .node("f2", "feature", &[("status", "done")])
        .n("f3", "feature")
        .node("ms1", "milestone", &[("status", "active")])
        .build();
    let results = bulk_status(&g).kinds;
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
    assert!(bulk_status(&CommandGraph::default()).kinds.is_empty());
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
    let unreferenced = report
        .unreferenced_counts
        .iter()
        .find(|c| c.kind == "feature")
        .unwrap();
    assert_eq!((unreferenced.unreferenced, unreferenced.total), (1, 2));
    assert_eq!(report.completeness.features_with_status, 1);
    assert_eq!(report.completeness.features_total, 2);
    assert_eq!(report.completeness.milestones_with_features, 1);
    assert_eq!(report.completeness.milestones_total, 2);
}

#[specforge_test(
    behavior = "surface_health",
    verify = "health counts an entity only it references as unreferenced, and a milestone whose only reference is itself as one without references"
)]
#[ignore = "flipped by T4"]
fn health_counts_an_entity_only_it_references_as_unreferenced() {
    let g = G::default()
        .n("f_self", "feature")
        .n("f_used", "feature")
        .n("j1", "journey")
        .n("ms_self", "milestone")
        .n("ms1", "milestone")
        .edge("f_self", "f_self", "depends_on")
        .edge("j1", "f_used", "features")
        .edge("ms_self", "ms_self", "depends_on")
        .edge("ms1", "f_used", "features")
        .build();
    let report = project_health(&g);
    let feature = report
        .unreferenced_counts
        .iter()
        .find(|c| c.kind == "feature")
        .unwrap();
    assert_eq!((feature.unreferenced, feature.total), (1, 2));
    assert_eq!(report.completeness.milestones_with_features, 1);
}
