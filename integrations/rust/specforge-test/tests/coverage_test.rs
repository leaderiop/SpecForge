use specforge_test::coverage::*;
use specforge_test::registry::{TestOutcome, TestRecordEntry};
use std::fs;

fn make_entry(entity_id: &str, verify: Option<&str>) -> TestRecordEntry {
    TestRecordEntry {
        entity_kind: "behavior".to_string(),
        entity_id: entity_id.to_string(),
        test_name: format!("test_{}", verify.unwrap_or("unknown")),
        file: "test.rs".to_string(),
        verify: verify.map(|s| s.to_string()),
        outcome: TestOutcome::Pass,
    }
}

fn make_graph(entities: Vec<ExportedEntity>) -> GraphExport {
    GraphExport {
        entities,
        timestamp: "2026-03-08T00:00:00Z".to_string(),
    }
}

fn make_entity(id: &str, kind: &str, verify: Vec<(&str, &str)>, testable: bool) -> ExportedEntity {
    ExportedEntity {
        id: id.to_string(),
        kind: kind.to_string(),
        verify: verify
            .into_iter()
            .map(|(k, desc)| ExportedVerify {
                kind: k.to_string(),
                description: desc.to_string(),
                slug: desc.to_lowercase().replace(' ', "_"),
            })
            .collect(),
        testable,
    }
}

#[test]
fn fully_covered_entity() {
    let graph = make_graph(vec![make_entity(
        "build_graph",
        "behavior",
        vec![
            ("unit", "graph contains one node per entity"),
            ("unit", "graph contains one edge per resolved reference"),
            ("unit", "edge types match relationship semantics"),
        ],
        true,
    )]);

    let entries = vec![
        make_entry("build_graph", Some("graph contains one node per entity")),
        make_entry("build_graph", Some("graph contains one edge per resolved reference")),
        make_entry("build_graph", Some("edge types match relationship semantics")),
    ];

    let diffs = compute_coverage_diff(&graph, &entries);
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].entity_id, "build_graph");
    assert_eq!(diffs[0].expected, 3);
    assert_eq!(diffs[0].covered, 3);
    assert_eq!(diffs[0].status, CoverageDiffStatus::FullyCovered);
}

#[test]
fn partially_covered_entity() {
    let graph = make_graph(vec![make_entity(
        "parse_imports",
        "behavior",
        vec![
            ("unit", "parse full use import"),
            ("unit", "parse selective use import with braces"),
            ("unit", "reject use import with .spec extension"),
        ],
        true,
    )]);

    let entries = vec![
        make_entry("parse_imports", Some("parse full use import")),
    ];

    let diffs = compute_coverage_diff(&graph, &entries);
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].expected, 3);
    assert_eq!(diffs[0].covered, 1);
    assert_eq!(diffs[0].status, CoverageDiffStatus::PartiallyCovered);
}

#[test]
fn uncovered_entity() {
    let graph = make_graph(vec![make_entity(
        "resolve_refs",
        "behavior",
        vec![
            ("unit", "ref resolves to node"),
            ("unit", "missing ref emits error"),
        ],
        true,
    )]);

    // Entity is tested but no verify descriptions match
    let entries = vec![make_entry("resolve_refs", None)];
    let diffs = compute_coverage_diff(&graph, &entries);
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].expected, 2);
    assert_eq!(diffs[0].covered, 0);
    assert_eq!(diffs[0].status, CoverageDiffStatus::Uncovered);
}

#[test]
fn no_intent_entity() {
    let graph = make_graph(vec![make_entity(
        "some_behavior",
        "behavior",
        vec![], // no verify statements
        true,
    )]);

    // Entity is tested but has no verify statements in spec
    let entries = vec![make_entry("some_behavior", None)];
    let diffs = compute_coverage_diff(&graph, &entries);
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].expected, 0);
    assert_eq!(diffs[0].covered, 0);
    assert_eq!(diffs[0].status, CoverageDiffStatus::NoIntent);
}

#[test]
fn non_testable_entities_excluded() {
    let graph = make_graph(vec![
        make_entity("testable_one", "behavior", vec![("unit", "a test")], true),
        make_entity("feature_one", "feature", vec![], false),
    ]);

    let entries = vec![make_entry("testable_one", Some("a test"))];
    let diffs = compute_coverage_diff(&graph, &entries);

    assert_eq!(diffs.len(), 1, "non-testable entity should be excluded");
    assert_eq!(diffs[0].entity_id, "testable_one");
}

// --- load_graph_at_exit ---

#[test]
fn load_valid_graph_json() {
    let dir = tempfile::tempdir().unwrap();
    let graph = make_graph(vec![make_entity(
        "alpha",
        "behavior",
        vec![("unit", "test alpha")],
        true,
    )]);
    let json = serde_json::to_string_pretty(&graph).unwrap();
    let path = dir.path().join("graph.json");
    fs::write(&path, &json).unwrap();

    let loaded = load_graph(&path);
    assert!(loaded.is_some(), "valid graph.json should load");
    let loaded = loaded.unwrap();
    assert_eq!(loaded.entities.len(), 1);
    assert_eq!(loaded.entities[0].id, "alpha");
}

#[test]
fn missing_graph_json_returns_none() {
    let path = std::path::PathBuf::from("/nonexistent/graph.json");
    let loaded = load_graph(&path);
    assert!(loaded.is_none(), "missing file should return None");
}

#[test]
fn malformed_graph_json_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("graph.json");
    fs::write(&path, "{ not valid json !!!").unwrap();

    let loaded = load_graph(&path);
    assert!(loaded.is_none(), "malformed JSON should return None");
}

// --- print_coverage_summary ---

#[test]
fn summary_includes_all_testable_entities() {
    let diffs = vec![
        CoverageDiff {
            entity_id: "alpha".to_string(),
            entity_kind: "behavior".to_string(),
            expected: 3,
            covered: 3,
            status: CoverageDiffStatus::FullyCovered,
        },
        CoverageDiff {
            entity_id: "beta".to_string(),
            entity_kind: "behavior".to_string(),
            expected: 2,
            covered: 1,
            status: CoverageDiffStatus::PartiallyCovered,
        },
    ];

    let mut buf = Vec::new();
    format_coverage_summary(&mut buf, &diffs, "2026-03-08T00:00:00Z").unwrap();
    let output = String::from_utf8(buf).unwrap();

    assert!(output.contains("alpha"), "should list alpha: {output}");
    assert!(output.contains("beta"), "should list beta: {output}");
    assert!(output.contains("3/3"), "should show 3/3 for alpha: {output}");
    assert!(output.contains("1/2"), "should show 1/2 for beta: {output}");
}

#[test]
fn summary_includes_timestamp() {
    let diffs = vec![CoverageDiff {
        entity_id: "alpha".to_string(),
        entity_kind: "behavior".to_string(),
        expected: 1,
        covered: 1,
        status: CoverageDiffStatus::FullyCovered,
    }];

    let mut buf = Vec::new();
    format_coverage_summary(&mut buf, &diffs, "2026-03-08T12:34:56Z").unwrap();
    let output = String::from_utf8(buf).unwrap();

    assert!(output.contains("2026-03-08T12:34:56Z"), "should contain timestamp: {output}");
}

#[test]
fn summary_empty_diffs_produces_no_output() {
    let mut buf = Vec::new();
    format_coverage_summary(&mut buf, &[], "2026-03-08T00:00:00Z").unwrap();
    assert!(buf.is_empty(), "empty diffs should produce no output");
}
