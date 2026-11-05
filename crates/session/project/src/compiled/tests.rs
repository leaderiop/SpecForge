use super::*;
use specforge_registry::{Asked, MemoryDisk, PlainDisk};
use specforge_test_macros::test as specforge_test;

const MISSING_IMPORT: &str = "use \"missing\"\nbehavior a \"A\" {\n}\n";

#[test]
fn a_detached_project_has_no_root_and_resolves_no_import() {
    let mut detached = CompiledProject::detached();
    assert_eq!(detached.root(), None);
    detached.apply(vec![(
        "a.spec".into(),
        Read::Text(MISSING_IMPORT.to_string()),
    )]);
    let imports = |p: &CompiledProject| -> Vec<Diagnostic> {
        p.diagnostics()
            .into_iter()
            .filter(|d| d.code == "W113" || d.code == "I004" || d.code == "E025")
            .collect()
    };
    assert!(
        imports(&detached).is_empty(),
        "{:?}",
        detached.diagnostics()
    );

    // The same text in a project on disk resolves its imports.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("specforge.json"), r#"{"extensions": []}"#).unwrap();
    std::fs::write(dir.path().join("a.spec"), MISSING_IMPORT).unwrap();
    let compiled = CompiledProject::compile(
        &specforge_common::ProjectRoot::of(dir.path()).unwrap(),
        None,
    );
    assert!(
        !imports(&compiled).is_empty(),
        "{:?}",
        compiled.diagnostics()
    );
}

#[test]
fn a_detached_project_asks_no_disk_for_its_report() {
    let disk = MemoryDisk::new().with_file("/specforge-report.json", b"{");
    assert!(
        CompiledProject::detached()
            .recorded(&disk)
            .unwrap()
            .report
            .is_none()
    );
    assert!(disk.asked().is_empty());
}

#[test]
fn a_rooted_project_reads_its_report_through_the_disk_it_is_given() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("specforge.json"), r#"{"extensions": []}"#).unwrap();
    let project = CompiledProject::compile(
        &specforge_common::ProjectRoot::of(dir.path()).unwrap(),
        None,
    );
    let file = project.root().unwrap().join("specforge-report.json");
    let disk = MemoryDisk::new().with_file(
        &file,
        br#"{"results":{"a":{"tests":[{"name":"t","status":"passed"}]}}}"#,
    );
    assert!(project.test_report(&disk).unwrap().is_some());
    assert_eq!(disk.asked(), [Asked::Read(file)]);
    // Nothing on the file system: the memo re-keys on content.
    assert!(project.test_report(&PlainDisk).unwrap().is_none());
}

#[specforge_test(
    behavior = "snapshot_entities_once",
    verify = "the checks, the check passes and the coverage of one compile read one snapshot"
)]
fn skipping_the_checks_reports_none_and_scores_the_graph_on_first_use() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"s","version":"0.1.0","extensions":["@specforge/software"]}"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("a.spec"), "behavior alpha \"A\" {\n}\n").unwrap();
    let runtime = specforge_component::ComponentRuntime::with_user_cache();
    let mut project = CompiledProject::compile(
        &specforge_common::ProjectRoot::of(dir.path()).unwrap(),
        Some(Arc::new(runtime)),
    );
    let checked = project.diagnostics();
    assert!(checked.iter().any(|d| d.code == "W006"), "{checked:?}");
    assert!(!project.checks_skipped());

    project.skip_checks();
    assert!(project.checks_skipped());
    assert!(
        project.diagnostics().iter().all(|d| d.code != "W006"),
        "{:?}",
        project.diagnostics()
    );
    assert_eq!(project.entities().kind_of("alpha"), Some("behavior"));
}
