use specforge_common::Severity;
use specforge_resolver::{linker::link_references, resolve_project};
use specforge_test_macros::test as specforge_test;
use tempfile::TempDir;
use std::fs;

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

// B:resolve_use_imports — verify contract "requires/ensures consistency for use import resolution"
#[test]
#[specforge_test(behavior = "resolve_use_imports", verify = "requires/ensures consistency for use import resolution")]
fn resolve_use_imports_contract() {
    // Requires: project with valid use imports across files
    // Ensures: all files resolved, no E025 errors
    let dir = setup_project(&[
        ("types.spec", r#"behavior alpha "A" { contract "first" }"#),
        ("main.spec", "use \"types\"\nbehavior beta \"B\" { invariants [alpha] }"),
    ]);

    let result = resolve_project(dir.path());

    let errors: Vec<_> = result.diagnostics.iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "valid imports must not produce errors: {:?}", errors);
    assert_eq!(result.files.len(), 2, "both files must be resolved");

    // Both file paths should be present
    let paths: Vec<&str> = result.files.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.iter().any(|p| p.ends_with("types.spec")), "types.spec must be resolved");
    assert!(paths.iter().any(|p| p.ends_with("main.spec")), "main.spec must be resolved");
}

// B:detect_import_cycles — verify contract "requires/ensures consistency for import cycle detection"
#[test]
#[specforge_test(behavior = "detect_import_cycles", verify = "requires/ensures consistency for import cycle detection")]
fn detect_import_cycles_contract() {
    // Requires: project with circular imports
    // Ensures: W003 cycle diagnostic produced as warning (not error)
    let dir = setup_project(&[
        ("a.spec", "use \"b\"\nbehavior alpha \"A\" { }"),
        ("b.spec", "use \"a\"\nbehavior beta \"B\" { }"),
    ]);

    let result = resolve_project(dir.path());

    let cycle_warnings: Vec<_> = result.diagnostics.iter()
        .filter(|d| d.code == "W003")
        .collect();
    assert!(!cycle_warnings.is_empty(), "circular imports must produce W003");
    assert!(
        cycle_warnings.iter().all(|d| d.severity == Severity::Warning),
        "import cycles must be warnings, not errors"
    );

    // Files should still be resolved despite the cycle
    assert!(!result.files.is_empty(), "files must still be resolved despite cycle");
}

// B:link_entity_references — verify contract "requires/ensures consistency for entity reference linking"
#[test]
#[specforge_test(behavior = "link_entity_references", verify = "requires/ensures consistency for entity reference linking")]
fn link_entity_references_contract() {
    // Requires: project with cross-file entity references
    // Ensures: valid references produce edges, invalid produce E001
    let dir = setup_project(&[
        ("main.spec", r#"
behavior alpha "A" { contract "first" }
behavior beta "B" { contract "second" }
feature gamma "G" {
  behaviors [alpha, beta, nonexistent]
}"#),
    ]);

    let resolved = resolve_project(dir.path());
    let (edges, diagnostics) = link_references(&resolved);

    // Valid references produce edges
    assert_eq!(edges.len(), 2, "two valid references must produce two edges");
    assert!(edges.iter().any(|e| e.target == "alpha"), "alpha reference must produce edge");
    assert!(edges.iter().any(|e| e.target == "beta"), "beta reference must produce edge");
    assert!(edges.iter().all(|e| e.source == "gamma"), "all edges must originate from gamma");

    // Invalid reference produces E001
    let e001s: Vec<_> = diagnostics.iter().filter(|d| d.code == "E001").collect();
    assert_eq!(e001s.len(), 1, "unresolvable reference must produce E001");
    assert!(e001s[0].message.contains("nonexistent"), "E001 must mention the unresolved ID");
}
