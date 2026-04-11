use specforge_watch::ImportDag;
use specforge_test_macros::test as spec;

// ── cycle detection ───────────────────────────────────────────

#[spec(behavior = "track_import_dag_incrementally")]
#[test]
fn detect_direct_cycle_in_import_dag() {
    let mut dag = ImportDag::new();
    dag.set_imports("a.spec", vec!["b.spec".to_string()]);
    dag.set_imports("b.spec", vec!["a.spec".to_string()]);

    let cycles = dag.detect_cycles();
    assert!(!cycles.is_empty(), "should detect cycle between a.spec and b.spec");
    // Both files should be in cycle participants
    let flat: Vec<&str> = cycles.iter().flat_map(|c| c.iter().map(|s| s.as_str())).collect();
    assert!(flat.contains(&"a.spec"));
    assert!(flat.contains(&"b.spec"));
}

#[spec(behavior = "track_import_dag_incrementally")]
#[test]
fn no_cycle_in_acyclic_import_dag() {
    let mut dag = ImportDag::new();
    dag.set_imports("a.spec", vec![]);
    dag.set_imports("b.spec", vec!["a.spec".to_string()]);
    dag.set_imports("c.spec", vec!["b.spec".to_string()]);

    let cycles = dag.detect_cycles();
    assert!(cycles.is_empty(), "acyclic DAG should have no cycles");
}

#[spec(behavior = "track_import_dag_incrementally")]
#[test]
fn detect_transitive_cycle_in_import_dag() {
    let mut dag = ImportDag::new();
    dag.set_imports("a.spec", vec!["b.spec".to_string()]);
    dag.set_imports("b.spec", vec!["c.spec".to_string()]);
    dag.set_imports("c.spec", vec!["a.spec".to_string()]);

    let cycles = dag.detect_cycles();
    assert!(!cycles.is_empty(), "should detect transitive cycle");
}

#[spec(behavior = "invalidate_changed_files", verify = "changed file is in invalidation set")]
#[test]
fn changed_file_is_in_invalidation_set() {
    let mut dag = ImportDag::new();
    dag.set_imports("a.spec", vec![]);

    let affected = dag.invalidation_set(&["a.spec".to_string()]);
    assert!(affected.contains("a.spec"));
}

#[spec(behavior = "invalidate_changed_files", verify = "direct importers are in invalidation set")]
#[test]
fn direct_importers_are_in_invalidation_set() {
    let mut dag = ImportDag::new();
    dag.set_imports("a.spec", vec![]);
    dag.set_imports("b.spec", vec!["a.spec".to_string()]);

    let affected = dag.invalidation_set(&["a.spec".to_string()]);
    assert!(affected.contains("a.spec"));
    assert!(affected.contains("b.spec"));
}

#[spec(behavior = "invalidate_changed_files", verify = "transitive importers are in invalidation set")]
#[test]
fn transitive_importers_are_in_invalidation_set() {
    let mut dag = ImportDag::new();
    dag.set_imports("a.spec", vec![]);
    dag.set_imports("b.spec", vec!["a.spec".to_string()]);
    dag.set_imports("c.spec", vec!["b.spec".to_string()]);

    let affected = dag.invalidation_set(&["a.spec".to_string()]);
    assert!(affected.contains("a.spec"));
    assert!(affected.contains("b.spec"));
    assert!(affected.contains("c.spec"));
}

#[spec(behavior = "invalidate_changed_files")]
#[test]
fn unrelated_files_are_not_in_invalidation_set() {
    let mut dag = ImportDag::new();
    dag.set_imports("a.spec", vec![]);
    dag.set_imports("b.spec", vec!["a.spec".to_string()]);
    dag.set_imports("unrelated.spec", vec![]);

    let affected = dag.invalidation_set(&["a.spec".to_string()]);
    assert!(!affected.contains("unrelated.spec"));
}

#[spec(behavior = "track_import_dag_incrementally")]
#[test]
fn removed_file_can_be_removed_from_dag() {
    let mut dag = ImportDag::new();
    dag.set_imports("a.spec", vec![]);
    dag.set_imports("b.spec", vec!["a.spec".to_string()]);

    dag.remove_file("b.spec");
    assert!(dag.imports_of("b.spec").is_empty());
}

#[spec(behavior = "track_import_dag_incrementally")]
#[test]
fn set_imports_replaces_previous_entry() {
    let mut dag = ImportDag::new();
    dag.set_imports("a.spec", vec!["old.spec".to_string()]);
    dag.set_imports("a.spec", vec!["new.spec".to_string()]);

    assert_eq!(dag.imports_of("a.spec"), vec!["new.spec"]);
}
