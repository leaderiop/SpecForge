use specforge_common::Severity;
use specforge_resolver::{link_references, resolve_project};
use specforge_test_macros::test as specforge_test;
use std::fs;
use tempfile::TempDir;

fn setup_project(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (path, content) in files {
        let full = dir.path().join(path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&full, content).unwrap();
    }
    dir
}

#[specforge_test(behavior = "resolve_use_imports", verify = "resolve use path to file on disk")]
#[test]
fn resolve_use_import_to_file() {
    let dir = setup_project(&[
        ("types.spec", r#"behavior alpha "A" { contract "first" }"#),
        (
            "main.spec",
            "use types\nbehavior beta \"B\" {\n  invariants [alpha]\n}",
        ),
    ]);

    let result = resolve_project(dir.path());

    assert!(
        result.diagnostics.iter().all(|d| d.severity != Severity::Error),
        "unexpected errors: {:?}",
        result.diagnostics
    );
    assert_eq!(result.files.len(), 2);
}

#[specforge_test(behavior = "resolve_use_imports", verify = "missing import file produces E025")]
#[test]
fn missing_import_produces_e025() {
    let dir = setup_project(&[("main.spec", "use nonexistent\nbehavior foo \"F\" { }")]);

    let result = resolve_project(dir.path());

    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "E025")
        .collect();
    assert_eq!(errors.len(), 1, "should produce E025 for missing import");
}

#[specforge_test(behavior = "detect_import_cycles", verify = "detect direct cycle between two files")]
#[test]
fn detect_direct_import_cycle() {
    let dir = setup_project(&[
        ("a.spec", "use b\nbehavior alpha \"A\" { }"),
        ("b.spec", "use a\nbehavior beta \"B\" { }"),
    ]);

    let result = resolve_project(dir.path());

    let cycle_errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "E003")
        .collect();
    assert!(
        !cycle_errors.is_empty(),
        "should detect import cycle with E003"
    );
}

#[specforge_test(behavior = "detect_import_cycles", verify = "detect transitive cycle across three files")]
#[test]
fn detect_transitive_import_cycle() {
    let dir = setup_project(&[
        ("a.spec", "use b\nbehavior alpha \"A\" { }"),
        ("b.spec", "use c\nbehavior beta \"B\" { }"),
        ("c.spec", "use a\nbehavior gamma \"G\" { }"),
    ]);

    let result = resolve_project(dir.path());

    let cycle_errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "E003")
        .collect();
    assert!(
        !cycle_errors.is_empty(),
        "should detect transitive cycle with E003"
    );
}

#[specforge_test(behavior = "detect_import_cycles", verify = "non-cyclic files still process when a cycle exists")]
#[test]
fn non_cyclic_files_still_resolve_when_cycle_exists() {
    let dir = setup_project(&[
        ("a.spec", "use b\nbehavior alpha \"A\" { }"),
        ("b.spec", "use a\nbehavior beta \"B\" { }"),
        ("clean.spec", "behavior gamma \"G\" { status \"ok\" }"),
    ]);

    let result = resolve_project(dir.path());

    // clean.spec should still be in the resolved files
    let clean = result.files.iter().find(|f| f.path.ends_with("clean.spec"));
    assert!(clean.is_some(), "non-cyclic file should still be resolved");
}

// --- nested directory imports ---

#[specforge_test(behavior = "resolve_use_imports", verify = "imports across nested directories resolve correctly")]
#[test]
fn imports_across_nested_directories_resolve_correctly() {
    let dir = setup_project(&[
        ("sub/types.spec", r#"behavior alpha "A" { contract "first" }"#),
        (
            "main.spec",
            "use sub/types\nbehavior beta \"B\" {\n  invariants [alpha]\n}",
        ),
    ]);

    let result = resolve_project(dir.path());

    assert!(
        result.diagnostics.iter().all(|d| d.severity != Severity::Error),
        "nested import should resolve without errors: {:?}",
        result.diagnostics
    );
    assert_eq!(result.files.len(), 2);
    // The file from the subdirectory should be present
    assert!(
        result.files.iter().any(|f| f.path.contains("sub")),
        "subdirectory file should be in resolved files"
    );
}

// --- link_entity_references ---

#[specforge_test(behavior = "link_entity_references", verify = "reference list IDs create graph edges")]
#[test]
fn reference_list_creates_pending_edges() {
    let dir = setup_project(&[
        ("main.spec", r#"
behavior alpha "A" { contract "first" }
behavior beta "B" { contract "second" }
feature gamma "G" {
  behaviors [alpha, beta]
}"#),
    ]);

    let resolved = resolve_project(dir.path());
    let (edges, diagnostics) = link_references(&resolved);

    assert!(
        diagnostics.iter().all(|d| d.severity != Severity::Error),
        "unexpected errors: {:?}",
        diagnostics
    );
    assert_eq!(edges.len(), 2, "should produce 2 edges from behaviors list");
    assert_eq!(edges[0].source, "gamma");
    assert!(edges.iter().any(|e| e.target == "alpha"));
    assert!(edges.iter().any(|e| e.target == "beta"));
}

#[specforge_test(behavior = "link_entity_references", verify = "unresolvable reference produces E001")]
#[test]
fn unresolvable_reference_produces_e001() {
    let dir = setup_project(&[
        ("main.spec", r#"
behavior alpha "A" { contract "first" }
feature gamma "G" {
  behaviors [alpha, nonexistent]
}"#),
    ]);

    let resolved = resolve_project(dir.path());
    let (_, diagnostics) = link_references(&resolved);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E001").collect();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("nonexistent"));
}

#[specforge_test(behavior = "link_entity_references", verify = "close match triggers did-you-mean suggestion")]
#[test]
fn close_match_triggers_did_you_mean() {
    let dir = setup_project(&[
        ("main.spec", r#"
behavior alpha_parser "A" { contract "first" }
feature gamma "G" {
  behaviors [alpha_parsr]
}"#),
    ]);

    let resolved = resolve_project(dir.path());
    let (_, diagnostics) = link_references(&resolved);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E001").collect();
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].suggestion.is_some(),
        "should suggest did-you-mean for close match"
    );
    assert!(
        errors[0].suggestion.as_ref().unwrap().contains("alpha_parser"),
        "suggestion should contain the close match"
    );
}
