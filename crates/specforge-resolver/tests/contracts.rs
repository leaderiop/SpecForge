use specforge_common::{DiagnosticData, Severity};
use specforge_resolver::resolve_project;
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

// B:resolve_use_imports — verify contract "requires/ensures consistency for use import resolution"
#[specforge_test(
    behavior = "resolve_use_imports",
    verify = "Resolve Use Imports: use import resolution holds — registries_populated_fired, filesystem_available, imports_resolved, missing_files_diagnosed, dependency_graph_built"
)]
fn resolve_use_imports_contract() {
    // Requires filesystem_available: the project lives on disk. main.spec
    // imports two files, one nested, and one that does not exist.
    let dir = setup_project(&[
        ("types.spec", r#"behavior alpha "A" { contract "first" }"#),
        ("models/user.spec", r#"behavior gamma "G" { contract "g" }"#),
        (
            "main.spec",
            "use \"types\"\nuse \"models/user\"\nuse \"missing\"\nbehavior beta \"B\" { invariants [alpha] }",
        ),
    ]);

    let result = resolve_project(dir.path());
    let file = |path: &str| {
        result
            .files
            .iter()
            .find(|f| f.path == path)
            .unwrap_or_else(|| panic!("{path} not resolved"))
    };

    // imports_resolved + dependency_graph_built: main.spec's edges are the
    // two files it imports, and the leaves import nothing.
    let mut targets = file("main.spec").import_targets.clone();
    targets.sort();
    assert_eq!(targets, ["models/user.spec", "types.spec"]);
    assert!(file("types.spec").import_targets.is_empty());
    assert!(file("models/user.spec").import_targets.is_empty());
    // Imports come before the file that uses them.
    let position = |path: &str| result.files.iter().position(|f| f.path == path).unwrap();
    assert!(position("types.spec") < position("main.spec"));
    assert!(position("models/user.spec") < position("main.spec"));

    // missing_files_diagnosed: the one missing import, and nothing else.
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].code, "E025");
    assert!(
        errors[0].message.contains("missing"),
        "{}",
        errors[0].message
    );
    let span = errors[0].span.as_ref().expect("E025 points at the use");
    assert_eq!(span.file.as_str(), "main.spec");
    assert_eq!(span.start_line, 3);
    // The path it names is data too, so no consumer parses the message.
    assert!(
        matches!(
            errors[0].data.as_deref(),
            Some(DiagnosticData::UnresolvedImport { path, .. }) if path == "missing"
        ),
        "{:?}",
        errors[0].data
    );
}

// B:detect_import_cycles — verify contract "requires/ensures consistency for import cycle detection"
#[specforge_test(
    behavior = "detect_import_cycles",
    verify = "Detect Import Cycles: import cycle detection holds — import_graph_available, cycles_detected, cycle_diagnostic_emitted, non_cyclic_unaffected"
)]
fn detect_import_cycles_contract() {
    // Requires: project with circular imports
    // Ensures: W113 cycle diagnostic produced as warning (not error)
    let dir = setup_project(&[
        ("a.spec", "use \"b\"\nbehavior alpha \"A\" { }"),
        ("b.spec", "use \"a\"\nbehavior beta \"B\" { }"),
    ]);

    let result = resolve_project(dir.path());

    let cycle_warnings: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "W113")
        .collect();
    assert!(
        !cycle_warnings.is_empty(),
        "circular imports must produce W113"
    );
    assert!(
        cycle_warnings
            .iter()
            .all(|d| d.severity == Severity::Warning),
        "import cycles must be warnings, not errors"
    );

    // Files should still be resolved despite the cycle
    assert!(
        !result.files.is_empty(),
        "files must still be resolved despite cycle"
    );
}
