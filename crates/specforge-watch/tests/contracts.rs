use specforge_graph::{build_graph, Edge, Graph, Node};
use specforge_common::{SourceSpan, Sym};
use specforge_parser::{parse, EntityId, EntityKind, FieldMap, FieldValue};
use specforge_watch::{
    compute_graph_delta, notify_delta_subscribers, plan_incremental_dispatch,
    validate_delta_correctness, DeltaSubscriber, DiagnosticsDelta, GraphDelta,
    ImportDag, IncrementalPipeline, KindDescriptor, NodeChange, ValidatorDescriptor,
    ValidatorInput,
};
use specforge_test_macros::test as specforge_test;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn make_node(id: &str, kind: &str, file: &str, line: usize) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind { raw: Sym::new(kind) },
        title: Some(id.to_string()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: Sym::new(file),
            start_line: line,
            start_col: 0,
            end_line: line,
            end_col: 0,
        },
    }
}

fn cold_build(files: &[(&str, &str)]) -> (IncrementalPipeline, HashMap<String, String>) {
    let mut sources: HashMap<String, String> = HashMap::new();
    let mut spec_files = Vec::new();

    for (path, content) in files {
        let spec = parse(content, path);
        spec_files.push((path.to_string(), spec));
        sources.insert(path.to_string(), content.to_string());
    }

    let all_specs: Vec<_> = spec_files.iter().map(|(_, s)| s.clone()).collect();
    let (graph, diagnostics) = build_graph(&all_specs);

    let mut dag = ImportDag::new();
    for (path, _) in &spec_files {
        dag.set_imports(path, vec![]);
    }
    for (path, spec) in &spec_files {
        let imports: Vec<String> = spec.imports.iter().map(|i| i.path.to_string()).collect();
        dag.set_imports_resolved(path, imports);
    }

    let pipeline = IncrementalPipeline::from_cold_build(spec_files, graph, dag, diagnostics);
    (pipeline, sources)
}

// B:compute_graph_delta — verify contract "requires/ensures consistency for graph delta computation"
#[test]
#[specforge_test(behavior = "compute_graph_delta", verify = "requires/ensures consistency for graph delta computation")]
fn compute_graph_delta_contract() {
    // Requires: old graph and new graph
    // Ensures: correct added/removed/modified nodes and edges
    let mut old = Graph::new();
    old.add_node(make_node("a", "behavior", "a.spec", 1));
    old.add_node(make_node("b", "feature", "a.spec", 5));
    old.add_edge(Edge {
        source: "b".into(), target: "a".into(), label: "behaviors".into(),
    });

    let mut new = Graph::new();
    // "a" modified (different field)
    let mut new_a = make_node("a", "behavior", "a.spec", 1);
    new_a.fields.push(Sym::new("status"), FieldValue::String("done".to_string()));
    new.add_node(new_a);
    // "b" removed, "c" added
    new.add_node(make_node("c", "type", "a.spec", 10));

    let delta = compute_graph_delta(&old, &new);

    assert_eq!(delta.added_nodes.len(), 1, "c must be added");
    assert_eq!(delta.added_nodes[0].id, "c");
    assert_eq!(delta.removed_nodes.len(), 1, "b must be removed");
    assert_eq!(delta.removed_nodes[0].id, "b");
    assert_eq!(delta.modified_nodes.len(), 1, "a must be modified");
    assert_eq!(delta.modified_nodes[0].id, "a");
    assert!(!delta.removed_edges.is_empty(), "b->a edge must be removed");
}

// B:debounce_file_changes — verify contract "requires/ensures consistency for file change debouncing"
#[test]
#[specforge_test(behavior = "debounce_file_changes", verify = "requires/ensures consistency for file change debouncing")]
fn debounce_file_changes_contract() {
    use specforge_watch::Debouncer;
    use std::sync::mpsc;
    use std::time::Duration;

    // Requires: rapid successive file change events (including duplicates)
    // Ensures: single coalesced batch with deduplicated and sorted files
    let (tx, rx) = mpsc::channel();
    let debouncer = Debouncer::new(Duration::from_millis(50));

    tx.send("b.spec".to_string()).unwrap();
    tx.send("a.spec".to_string()).unwrap();
    tx.send("b.spec".to_string()).unwrap(); // duplicate
    tx.send("a.spec".to_string()).unwrap(); // duplicate

    let batch = debouncer.coalesce(&rx).unwrap();

    assert_eq!(batch.len(), 2, "duplicates must be coalesced");
    assert_eq!(batch, vec!["a.spec", "b.spec"], "batch must be sorted and deduplicated");
}

// B:track_import_dag_incrementally — verify contract "requires/ensures consistency for incremental import DAG tracking"
#[test]
#[specforge_test(behavior = "track_import_dag_incrementally", verify = "requires/ensures consistency for incremental import DAG tracking")]
fn track_import_dag_incrementally_contract() {
    // Requires: file change that adds/removes imports
    // Ensures: DAG updated, dependents invalidated
    let (mut pipeline, mut sources) = cold_build(&[
        ("a.spec", r#"behavior foo "Foo" { contract "x" }"#),
        ("b.spec", r#"behavior bar "Bar" { contract "y" }"#),
    ]);

    // Initially no imports
    assert!(pipeline.import_dag().imports_of("b.spec").is_empty());

    // Add import to b.spec
    sources.insert(
        "b.spec".to_string(),
        "use \"a\"\nbehavior bar \"Bar\" { contract \"y\" }".to_string(),
    );
    pipeline.rebuild(&["b.spec".to_string()], |f| sources.get(f).cloned());

    // DAG must be updated
    let imports = pipeline.import_dag().imports_of("b.spec");
    assert!(imports.contains(&"a.spec"), "b.spec must now import a.spec");

    // Remove the import
    sources.insert(
        "b.spec".to_string(),
        r#"behavior bar "Bar" { contract "y" }"#.to_string(),
    );
    pipeline.rebuild(&["b.spec".to_string()], |f| sources.get(f).cloned());

    let imports_after = pipeline.import_dag().imports_of("b.spec");
    assert!(!imports_after.contains(&"a.spec"), "import must be removed from DAG");
}

// B:dispatch_incremental_validators — verify contract "requires/ensures consistency for incremental dispatch"
#[test]
#[specforge_test(behavior = "dispatch_incremental_validators", verify = "requires/ensures consistency for incremental dispatch")]
fn dispatch_incremental_validators_contract() {
    // Requires: delta with affected entity kinds + validator descriptors
    // Ensures: incremental validators get Delta input, non-incremental get FullGraph
    let validators = vec![
        ValidatorDescriptor {
            extension_name: "inc_ext".to_string(),
            kinds: vec![KindDescriptor {
                kind_name: "behavior".to_string(),
                incremental: true,
            }],
        },
        ValidatorDescriptor {
            extension_name: "full_ext".to_string(),
            kinds: vec![KindDescriptor {
                kind_name: "decision".to_string(),
                incremental: false,
            }],
        },
    ];

    let delta = GraphDelta {
        added_nodes: vec![
            NodeChange { id: "n0".into(), kind: "behavior".into(), file: Some("a.spec".into()), line: Some(1) },
            NodeChange { id: "n1".into(), kind: "decision".into(), file: Some("a.spec".into()), line: Some(5) },
        ],
        removed_nodes: vec![],
        modified_nodes: vec![],
        added_edges: vec![],
        removed_edges: vec![],
        affected_files: vec!["a.spec".to_string()],
    };

    let graph = Graph::new();
    let plan = plan_incremental_dispatch(&validators, &delta, &graph);

    assert_eq!(plan.entries.len(), 2, "both validators must be dispatched");
    assert_eq!(plan.entries[0].input, ValidatorInput::Delta, "incremental validator gets delta");
    assert_eq!(plan.entries[1].input, ValidatorInput::FullGraph, "non-incremental gets full graph");
}

// B:notify_delta_subscribers — verify contract "requires/ensures consistency for delta subscriber notification"
#[test]
#[specforge_test(behavior = "notify_delta_subscribers", verify = "requires/ensures consistency for delta subscriber notification")]
fn notify_delta_subscribers_contract() {
    // Requires: delta produced + registered subscribers
    // Ensures: all subscribers notified exactly once
    struct Counter { count: Arc<AtomicUsize> }
    impl DeltaSubscriber for Counter {
        fn on_delta(&self, _: &GraphDelta, _: &DiagnosticsDelta, _: &[String]) {
            self.count.fetch_add(1, Ordering::SeqCst);
        }
    }

    let count = Arc::new(AtomicUsize::new(0));
    let subscribers: Vec<Box<dyn DeltaSubscriber>> = (0..3)
        .map(|_| Box::new(Counter { count: count.clone() }) as Box<dyn DeltaSubscriber>)
        .collect();

    let delta = GraphDelta {
        added_nodes: vec![],
        removed_nodes: vec![],
        modified_nodes: vec![],
        added_edges: vec![],
        removed_edges: vec![],
        affected_files: vec!["a.spec".to_string()],
    };
    let diag_delta = DiagnosticsDelta { added: vec![], removed: vec![] };

    notify_delta_subscribers(&subscribers, &delta, &diag_delta);

    assert_eq!(count.load(Ordering::SeqCst), 3, "all 3 subscribers must be notified exactly once");
}

// B:rebuild_affected_subgraph — verify contract "requires/ensures consistency for affected subgraph rebuild"
#[test]
#[specforge_test(behavior = "rebuild_affected_subgraph", verify = "requires/ensures consistency for affected subgraph rebuild")]
fn rebuild_affected_subgraph_contract() {
    // Requires: file change in a multi-file project
    // Ensures: only affected subgraph rebuilt, unrelated files untouched
    let (mut pipeline, mut sources) = cold_build(&[
        ("a.spec", r#"behavior foo "Foo" { contract "x" }"#),
        ("b.spec", r#"behavior bar "Bar" { contract "y" }"#),
    ]);

    assert_eq!(pipeline.graph().node_count(), 2);

    // Edit a.spec: change entity
    sources.insert(
        "a.spec".to_string(),
        r#"behavior baz "Baz" { contract "new" }"#.to_string(),
    );

    let result = pipeline.rebuild(&["a.spec".to_string()], |f| sources.get(f).cloned());

    // Only a.spec rebuilt
    assert!(result.rebuilt_files.contains(&"a.spec".to_string()), "changed file must be rebuilt");
    assert!(!result.rebuilt_files.contains(&"b.spec".to_string()), "unrelated file must NOT be rebuilt");

    // Graph reflects the change
    assert!(pipeline.graph().node("foo").is_none(), "old node must be removed");
    assert!(pipeline.graph().node("baz").is_some(), "new node must be present");
    assert!(pipeline.graph().node("bar").is_some(), "unrelated node must be preserved");
}

// B:validate_delta_correctness — verify contract "requires/ensures consistency for delta correctness validation"
#[test]
#[specforge_test(behavior = "validate_delta_correctness", verify = "requires/ensures consistency for delta correctness validation")]
fn validate_delta_correctness_contract() {
    // Requires: computed delta from old→new graph
    // Ensures: delta is correct (no dangling refs), validation passes
    let old = Graph::new();
    let mut new = Graph::new();
    new.add_node(make_node("a", "behavior", "a.spec", 1));
    new.add_node(make_node("b", "feature", "a.spec", 5));
    new.add_edge(Edge {
        source: "b".into(), target: "a".into(), label: "behaviors".into(),
    });

    let delta = compute_graph_delta(&old, &new);
    let result = validate_delta_correctness(&old, &new, &delta);

    assert!(result.is_ok(), "correct delta must pass validation: {:?}", result);
    let counts = result.unwrap();
    assert_eq!(counts.node_count, 2, "validation must report correct node count");
    assert_eq!(counts.edge_count, 1, "validation must report correct edge count");

    // Fabricate bad delta to verify it catches errors
    let bad_delta = GraphDelta {
        added_nodes: vec![],
        removed_nodes: vec![],
        modified_nodes: vec![],
        added_edges: vec![],
        removed_edges: vec![],
        affected_files: vec![],
    };
    let bad_result = validate_delta_correctness(&old, &new, &bad_delta);
    assert!(bad_result.is_err(), "incorrect delta must fail validation");
}
