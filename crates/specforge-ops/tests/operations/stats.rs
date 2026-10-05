use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue, VerifyStatement};
use specforge_test::prelude::*;

fn span() -> SourceSpan {
    SourceSpan {
        file: Sym::new("test.spec"),
        start_line: 1,
        start_col: 0,
        end_line: 1,
        end_col: 0,
    }
}

fn node(id: &str, kind: &str) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind {
            raw: Sym::new(kind),
        },
        title: None,
        fields: FieldMap::new(),
        source_span: span(),
        methods: Vec::new(),
    }
}

fn node_with_verify(id: &str, kind: &str) -> Node {
    let mut fields = FieldMap::new();
    fields.push(
        Sym::new("verify"),
        FieldValue::VerifyList(vec![VerifyStatement {
            kind: "unit".to_string(),
            description: "it works".to_string(),
        }]),
    );
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind {
            raw: Sym::new(kind),
        },
        title: None,
        fields,
        source_span: span(),
        methods: Vec::new(),
    }
}

// B:compute_project_statistics — verify unit "stats reports correct entity counts"
#[specforge_test(
    behavior = "compute_project_statistics",
    verify = "stats reports correct entity counts"
)]
fn stats_reports_correct_entity_counts() {
    let mut graph = Graph::new();
    graph.add_node(node("a", "behavior"));
    graph.add_node(node("b", "behavior"));
    graph.add_node(node("c", "feature"));

    let stats = specforge_ops::stats::compute_stats(&graph);
    assert_eq!(stats.total_entities, 3);
    assert_eq!(stats.entities_by_kind["behavior"], 2);
    assert_eq!(stats.entities_by_kind["feature"], 1);
}

// B:compute_project_statistics — verify unit "stats reports correct entity counts"
// (covers edge count reporting)
#[specforge_test(behavior = "compute_project_statistics")]
fn stats_reports_edge_count() {
    let mut graph = Graph::new();
    graph.add_node(node("a", "feature"));
    graph.add_node(node("b", "behavior"));
    graph.add_edge(Edge {
        source: Sym::new("a"),
        target: Sym::new("b"),
        label: Sym::new("behaviors"),
    });

    let stats = specforge_ops::stats::compute_stats(&graph);
    assert_eq!(stats.total_edges, 1);
}

// B:compute_project_statistics — verify unit "stats reports orphan count"
#[specforge_test(
    behavior = "compute_project_statistics",
    verify = "stats reports orphan count"
)]
fn stats_reports_orphan_count() {
    let mut graph = Graph::new();
    graph.add_node(node("a", "behavior")); // orphan — no edges
    graph.add_node(node("b", "feature"));
    graph.add_node(node("c", "behavior"));
    graph.add_edge(Edge {
        source: Sym::new("b"),
        target: Sym::new("c"),
        label: Sym::new("behaviors"),
    });

    let stats = specforge_ops::stats::compute_stats(&graph);
    assert_eq!(stats.orphan_count, 1); // only "a" is orphan
}

// B:compute_project_statistics — verify unit "stats reports coverage percentage"
// (covers verified entity counting for coverage computation)
#[specforge_test(behavior = "compute_project_statistics")]
fn stats_reports_verified_count() {
    let mut graph = Graph::new();
    graph.add_node(node_with_verify("a", "behavior"));
    graph.add_node(node("b", "behavior")); // no verify

    let stats = specforge_ops::stats::compute_stats(&graph);
    assert_eq!(stats.verified_count, 1);
}

// B:compute_project_statistics — verify unit "stats reports correct entity counts"
// (edge case: empty graph)
#[specforge_test(behavior = "compute_project_statistics")]
fn stats_on_empty_graph() {
    let graph = Graph::new();
    let stats = specforge_ops::stats::compute_stats(&graph);
    assert_eq!(stats.total_entities, 0);
    assert_eq!(stats.total_edges, 0);
    assert_eq!(stats.orphan_count, 0);
    assert_eq!(stats.verified_count, 0);
}

// B:compute_project_statistics — verify unit "stats reports coverage percentage"
#[specforge_test(
    behavior = "compute_project_statistics",
    verify = "stats reports coverage percentage"
)]
fn stats_coverage_with_testable_kinds() {
    let mut graph = Graph::new();
    graph.add_node(node_with_verify("a", "behavior")); // testable + verified
    graph.add_node(node("b", "behavior")); // testable, not verified
    graph.add_node(node("c", "feature")); // not testable

    let testable = &["behavior"];
    let stats = specforge_ops::stats::compute_stats_with_testable(&graph, testable);
    // 1 verified out of 2 testable = 50%
    assert_eq!(stats.testable_count, 2);
    assert_eq!(stats.verified_count, 1);
    assert!((stats.coverage_pct - 50.0).abs() < 0.01);
}

// B:compute_project_statistics — verify unit "coverage is 0% when testable_entity_count is zero"
#[specforge_test(
    behavior = "compute_project_statistics",
    verify = "coverage is 0% when testable_entity_count is zero"
)]
fn stats_coverage_zero_when_no_testable_entities() {
    let mut graph = Graph::new();
    graph.add_node(node("a", "feature")); // not testable

    let testable: &[&str] = &["behavior"]; // no behaviors in graph
    let stats = specforge_ops::stats::compute_stats_with_testable(&graph, testable);
    assert_eq!(stats.testable_count, 0);
    assert_eq!(stats.coverage_pct, 0.0);
}

// B:compute_project_statistics — verify unit "stats reports diagnostic summary"
#[specforge_test(
    behavior = "compute_project_statistics",
    verify = "stats reports diagnostic summary"
)]
fn stats_includes_diagnostic_summary() {
    let graph = Graph::new();
    let diagnostics = vec![
        specforge_common::Diagnostic {
            code: "E001".to_string(),
            severity: specforge_common::Severity::Error,
            message: "bad ref".to_string(),
            span: None,
            suggestion: None,
            data: None,
        },
        specforge_common::Diagnostic {
            code: "W012".to_string(),
            severity: specforge_common::Severity::Warning,
            message: "orphan".to_string(),
            span: None,
            suggestion: None,
            data: None,
        },
    ];
    let stats = specforge_ops::stats::compute_stats_with_diagnostics(&graph, &[], &diagnostics);
    assert_eq!(stats.error_count, 1);
    assert_eq!(stats.warning_count, 1);
    assert_eq!(stats.info_count, 0);
}

// A type may declare a struct member named `verify`; the entity's
// obligations are its verify statements wherever they sit among its fields,
// so the member must not hide them (plan 02, S2).
#[specforge_test(
    behavior = "compute_project_statistics",
    verify = "stats reports coverage percentage"
)]
fn stats_counts_obligations_behind_a_verify_member() {
    let mut payload = node_with_verify("Payload", "type");
    let mut fields = FieldMap::new();
    fields.push(Sym::new("verify"), FieldValue::Identifier("string".into()));
    for entry in payload.fields.entries() {
        fields.push(entry.key, entry.value.clone());
    }
    payload.fields = fields;
    let mut graph = Graph::new();
    graph.add_node(payload);
    graph.add_node(node("Status", "type"));

    let stats = specforge_ops::stats::compute_stats_with_testable(&graph, &["type"]);
    assert_eq!(stats.testable_count, 2);
    assert_eq!(stats.verified_count, 1, "Payload declares an obligation");
    assert!((stats.coverage_pct - 50.0).abs() < 0.01);
}

#[specforge_test(
    behavior = "compute_project_statistics",
    verify = "stats leaves the entities W004 exempts out of the testable count"
)]
fn stats_leaves_union_types_out_of_the_testable_count() {
    // `type Status = active | inactive`: no body to hold obligations.
    let mut status = node("Status", "type");
    status.fields.push(
        Sym::new("variants"),
        FieldValue::VariantList(vec!["active".into(), "inactive".into()]),
    );
    // A struct member that is only named `variants` exempts nothing.
    let mut named = node("Named", "type");
    named.fields.push(
        Sym::new("variants"),
        FieldValue::Identifier("string".into()),
    );
    let mut graph = Graph::new();
    graph.add_node(status);
    graph.add_node(named);
    graph.add_node(node_with_verify("Payload", "type"));

    let stats = specforge_ops::stats::compute_stats_with_testable(&graph, &["type"]);
    assert_eq!(stats.testable_count, 2, "Named and Payload");
    assert!((stats.coverage_pct - 50.0).abs() < 0.01);
}

/// The registries of a project whose `testable` kinds are testable and
/// must declare obligations (a W004 `no_verify_statements` rule targets
/// each), as `@specforge/software` declares `behavior`.
fn registries(testable: &[&str]) -> specforge_registry::RegistryBuild {
    use specforge_registry::KindRegistryEntry;
    use specforge_registry::validation_engine::{ValidationPatternKind, ValidationRulePattern};
    let mut build = specforge_registry::RegistryBuild::default();
    for kind in testable {
        build.kinds.register(KindRegistryEntry {
            kind_name: kind.to_string(),
            description: None,
            source_extension: "@t/soft".into(),
            testable: true,
            singleton: false,
            supports_verify: true,
            allowed_verify_kinds: Vec::new(),
            has_body_parser: false,
            semantic_token: None,
            lsp_icon: None,
            dot_shape: None,
            dot_color: None,
            dot_fillcolor: None,
            open_fields: false,
            contract_target: false,
            declares_types: false,
            lifecycle_field: None,
        });
        build.rules.push((
            ValidationRulePattern {
                code: "W004".into(),
                severity: specforge_common::Severity::Warning,
                message_template: "{kind} '{id}' is testable but declares no verify obligations"
                    .into(),
                check: ValidationPatternKind::NoVerifyStatements,
                target_kind: Some(kind.to_string()),
                edge_type: None,
                edge_peer_kind: None,
                field: Some("verify".into()),
                constraint: None,
                wasm_function: None,
            },
            "@t/soft".into(),
        ));
    }
    build
}

/// The contracts graph (a feature listing b, b depending on c, both
/// verified), plus an unverified behavior and a verified feature.
fn registries_graph() -> Graph {
    let mut graph = Graph::new();
    graph.add_node(node("a", "feature"));
    graph.add_node(node_with_verify("b", "behavior"));
    graph.add_node(node_with_verify("c", "behavior"));
    graph.add_node(node("d", "behavior"));
    graph.add_node(node_with_verify("e", "feature"));
    graph.add_edge(Edge {
        source: Sym::new("a"),
        target: Sym::new("b"),
        label: Sym::new("behaviors"),
    });
    graph.add_edge(Edge {
        source: Sym::new("b"),
        target: Sym::new("c"),
        label: Sym::new("depends_on"),
    });
    graph
}

/// `{:#?}` of the stats, less the type's name.
const STATS_TODAY: &str = r#"{
    total_entities: 5,
    total_edges: 2,
    orphan_count: 2,
    verified_count: 3,
    testable_count: 3,
    declared_count: 2,
    declared_pct: 66.66666666666666,
    coverage_pct: 66.66666666666666,
    proof_pct: None,
    error_count: 1,
    warning_count: 1,
    info_count: 0,
    entities_by_kind: {
        "behavior": 3,
        "feature": 2,
    },
}"#;

/// `stats` as `{:#?}` prints it, from its opening brace.
fn debug_body(stats: &impl std::fmt::Debug) -> String {
    let text = format!("{stats:#?}");
    text[text.find('{').unwrap()..].to_string()
}

// Pins what the surfaces' stats path computes today (plan 02 T0): no
// spec obligation, it must stay byte-identical when the path moves.
#[test]
fn stats_today_through_the_registries() {
    let graph = registries_graph();
    let build = registries(&["behavior"]);
    let diagnostics = vec![
        specforge_common::Diagnostic::error("E001", "err"),
        specforge_common::Diagnostic::warning("W002", "warn"),
    ];
    let coverage = specforge_project::coverage::ProjectCoverage::compute(
        &graph,
        specforge_project::coverage::CoverageRegistries {
            kinds: &build.kinds,
            fields: &build.fields,
            rules: &build.rules,
        },
        None,
    );
    let stats =
        specforge_ops::stats::compute_project_stats(&graph, &coverage.summary, &diagnostics);
    assert_eq!(debug_body(&stats), STATS_TODAY);
}
