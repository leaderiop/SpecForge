use specforge_common::Severity;
use specforge_resolver::{
    linker::link_references, resolve_project, resolve_project_with_config, PathAlias, ResolveConfig,
};
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
            "use \"types\"\nbehavior beta \"B\" {\n  invariants [alpha]\n}",
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
    let dir = setup_project(&[("main.spec", "use \"nonexistent\"\nbehavior foo \"F\" { }")]);

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
        ("a.spec", "use \"b\"\nbehavior alpha \"A\" { }"),
        ("b.spec", "use \"a\"\nbehavior beta \"B\" { }"),
    ]);

    let result = resolve_project(dir.path());

    let cycle_warnings: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "W003")
        .collect();
    assert!(
        !cycle_warnings.is_empty(),
        "should detect import cycle with W003"
    );
    assert!(
        cycle_warnings.iter().all(|d| d.severity == Severity::Warning),
        "import cycles should be warnings, not errors"
    );
}

#[specforge_test(behavior = "detect_import_cycles", verify = "detect transitive cycle across three files")]
#[test]
fn detect_transitive_import_cycle() {
    let dir = setup_project(&[
        ("a.spec", "use \"b\"\nbehavior alpha \"A\" { }"),
        ("b.spec", "use \"c\"\nbehavior beta \"B\" { }"),
        ("c.spec", "use \"a\"\nbehavior gamma \"G\" { }"),
    ]);

    let result = resolve_project(dir.path());

    let cycle_warnings: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "W003")
        .collect();
    assert!(
        !cycle_warnings.is_empty(),
        "should detect transitive cycle with W003"
    );
}

#[specforge_test(behavior = "detect_import_cycles", verify = "non-cyclic files still process when a cycle exists")]
#[test]
fn non_cyclic_files_still_resolve_when_cycle_exists() {
    let dir = setup_project(&[
        ("a.spec", "use \"b\"\nbehavior alpha \"A\" { }"),
        ("b.spec", "use \"a\"\nbehavior beta \"B\" { }"),
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
            "use \"sub/types\"\nbehavior beta \"B\" {\n  invariants [alpha]\n}",
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
fn link_reference_close_match_triggers_suggestion() {
    let dir = setup_project(&[
        ("main.spec", r#"
behavior alpha_handler "A" { contract "first" }
feature gamma "G" {
  behaviors [alpha_handlr]
}"#),
    ]);

    let resolved = resolve_project(dir.path());
    let (_, diagnostics) = link_references(&resolved);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E001").collect();
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].suggestion.as_ref().is_some_and(|s| s.contains("alpha_handler")),
        "close match in reference list should trigger did-you-mean suggestion"
    );
}

#[specforge_test(behavior = "provide_did_you_mean_suggestions", verify = "close match produces suggestion")]
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

#[specforge_test(behavior = "provide_did_you_mean_suggestions", verify = "distant match produces no suggestion")]
#[test]
fn distant_match_produces_no_suggestion() {
    let dir = setup_project(&[
        ("main.spec", r#"
behavior alpha "A" { contract "first" }
feature gamma "G" {
  behaviors [zzzzz_completely_different]
}"#),
    ]);

    let resolved = resolve_project(dir.path());
    let (_, diagnostics) = link_references(&resolved);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E001").collect();
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].suggestion.is_none(),
        "distant match must not produce suggestion"
    );
}

#[specforge_test(behavior = "provide_did_you_mean_suggestions", verify = "suggestion appears in help text")]
#[test]
fn suggestion_appears_in_help_text() {
    let dir = setup_project(&[
        ("main.spec", r#"
behavior alpha_handler "A" { contract "first" }
feature gamma "G" {
  behaviors [alpha_handlr]
}"#),
    ]);

    let resolved = resolve_project(dir.path());
    let (_, diagnostics) = link_references(&resolved);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E001").collect();
    assert_eq!(errors.len(), 1);
    let suggestion = errors[0].suggestion.as_ref().expect("should have suggestion");
    assert!(
        suggestion.contains("alpha_handler"),
        "suggestion must mention the closest match in help text"
    );
}

#[specforge_test(behavior = "provide_did_you_mean_suggestions", verify = "requires/ensures consistency for did-you-mean suggestions")]
#[test]
fn did_you_mean_contract_consistency() {
    // Close match → suggestion present
    let dir = setup_project(&[
        ("main.spec", r#"
behavior alpha_parser "A" { contract "first" }
feature gamma "G" { behaviors [alpha_parsr] }
"#),
    ]);
    let resolved = resolve_project(dir.path());
    let (_, diagnostics) = link_references(&resolved);
    let e001: Vec<_> = diagnostics.iter().filter(|d| d.code == "E001").collect();
    assert_eq!(e001.len(), 1);
    assert!(e001[0].suggestion.is_some(), "requires: close match → suggestion");

    // Distant match → no suggestion
    let dir2 = setup_project(&[
        ("main.spec", r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [xyz_totally_different] }
"#),
    ]);
    let resolved2 = resolve_project(dir2.path());
    let (_, diagnostics2) = link_references(&resolved2);
    let e001_2: Vec<_> = diagnostics2.iter().filter(|d| d.code == "E001").collect();
    assert_eq!(e001_2.len(), 1);
    assert!(e001_2[0].suggestion.is_none(), "ensures: distant match → no suggestion");
}

// --- 5-step path resolution cascade ---

#[specforge_test(behavior = "resolve_use_imports", verify = "resolve relative import from importing file directory")]
#[test]
fn resolve_relative_dot_slash() {
    let dir = setup_project(&[
        ("sub/helper.spec", r#"behavior helper "H" { contract "help" }"#),
        (
            "sub/main.spec",
            "use \"./helper\"\nbehavior user \"U\" { invariants [helper] }",
        ),
    ]);

    let result = resolve_project(dir.path());

    assert!(
        result.diagnostics.iter().all(|d| d.severity != Severity::Error),
        "relative ./helper should resolve without errors: {:?}",
        result.diagnostics
    );
    assert_eq!(result.files.len(), 2);
}

#[specforge_test(behavior = "resolve_use_imports", verify = "resolve relative import from importing file directory")]
#[test]
fn resolve_relative_dot_dot_slash() {
    let dir = setup_project(&[
        ("shared.spec", r#"behavior shared "S" { contract "shared" }"#),
        (
            "sub/main.spec",
            "use \"../shared\"\nbehavior user \"U\" { invariants [shared] }",
        ),
    ]);

    let result = resolve_project(dir.path());

    assert!(
        result.diagnostics.iter().all(|d| d.severity != Severity::Error),
        "relative ../shared should resolve without errors: {:?}",
        result.diagnostics
    );
    assert_eq!(result.files.len(), 2);
}

#[specforge_test(behavior = "resolve_use_imports", verify = "resolve relative import from importing file directory")]
#[test]
fn resolve_relative_escaping_spec_root() {
    let dir = setup_project(&[
        (
            "sub/main.spec",
            "use \"../../escape\"\nbehavior user \"U\" { }",
        ),
    ]);

    let result = resolve_project(dir.path());

    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "E025")
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "relative path escaping spec_root should produce E025"
    );
}

#[specforge_test(behavior = "resolve_use_imports", verify = "resolve path alias from specforge.json config")]
#[test]
fn resolve_path_alias() {
    let dir = setup_project(&[
        ("lib/shared/utils.spec", r#"behavior utils "U" { contract "util" }"#),
        (
            "main.spec",
            "use \"@shared/utils\"\nbehavior caller \"C\" { invariants [utils] }",
        ),
    ]);

    let config = ResolveConfig {
        path_aliases: vec![PathAlias {
            alias: "shared".to_string(),
            target: "lib/shared".to_string(),
        }],
    };
    let result = resolve_project_with_config(dir.path(), &config);

    assert!(
        result.diagnostics.iter().all(|d| d.severity != Severity::Error),
        "path alias @shared/utils should resolve without errors: {:?}",
        result.diagnostics
    );
    assert_eq!(result.files.len(), 2);
}

#[specforge_test(behavior = "resolve_use_imports", verify = "resolve directory to index.spec")]
#[test]
fn resolve_directory_to_index_spec() {
    let dir = setup_project(&[
        ("models/index.spec", r#"behavior model "M" { contract "model" }"#),
        (
            "main.spec",
            "use \"models\"\nbehavior caller \"C\" { invariants [model] }",
        ),
    ]);

    let result = resolve_project(dir.path());

    assert!(
        result.diagnostics.iter().all(|d| d.severity != Severity::Error),
        "directory import should resolve to index.spec: {:?}",
        result.diagnostics
    );
    // main.spec + models/index.spec = 2 files
    assert_eq!(result.files.len(), 2);
}

#[specforge_test(behavior = "resolve_use_imports", verify = "bare path takes precedence over directory index")]
#[test]
fn bare_path_precedence_over_index() {
    let dir = setup_project(&[
        ("models.spec", r#"behavior direct_model "DM" { contract "direct" }"#),
        ("models/index.spec", r#"behavior index_model "IM" { contract "index" }"#),
        (
            "main.spec",
            "use \"models\"\nbehavior caller \"C\" { invariants [direct_model] }",
        ),
    ]);

    let result = resolve_project(dir.path());

    assert!(
        result.diagnostics.iter().all(|d| d.severity != Severity::Error),
        "models.spec should take precedence over models/index.spec: {:?}",
        result.diagnostics
    );
    // main.spec imports models.spec (the file), models/index.spec is also discovered
    // The import from main.spec should resolve to models.spec, not models/index.spec
    let main_file = result.files.iter().find(|f| f.path.ends_with("main.spec")).unwrap();
    assert!(
        main_file.import_targets.iter().any(|t| t == "models.spec"),
        "import should resolve to models.spec, not models/index.spec; targets: {:?}",
        main_file.import_targets
    );
}

#[specforge_test(behavior = "resolve_use_imports", verify = "resolve extension import path")]
#[test]
fn extension_import_emits_i004() {
    let dir = setup_project(&[(
        "main.spec",
        "use \"@specforge/software\"\nbehavior foo \"F\" { }",
    )]);

    let result = resolve_project(dir.path());

    let infos: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "I004")
        .collect();
    assert_eq!(
        infos.len(),
        1,
        "uninstalled extension import should produce I004"
    );
    assert!(
        infos[0].message.contains("specforge") && infos[0].message.contains("software"),
        "I004 message should mention scope and name"
    );
    // No E025 should be emitted for extension imports
    assert!(
        result.diagnostics.iter().all(|d| d.code != "E025"),
        "extension import should not produce E025"
    );
}

#[specforge_test(behavior = "resolve_use_imports", verify = "missing import file produces E025 with suggestion")]
#[test]
fn missing_import_e025_with_suggestion() {
    let dir = setup_project(&[
        ("helpers.spec", r#"behavior helper "H" { contract "help" }"#),
        ("main.spec", "use \"helperz\"\nbehavior foo \"F\" { }"),
    ]);

    let result = resolve_project(dir.path());

    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "E025")
        .collect();
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].suggestion.as_ref().is_some_and(|s| s.contains("helpers")),
        "E025 should suggest close match 'helpers': {:?}",
        errors[0].suggestion
    );
}

#[specforge_test(behavior = "resolve_use_imports", verify = "missing import file with no close match produces E025 without suggestion")]
#[test]
fn missing_import_e025_no_suggestion() {
    let dir = setup_project(&[
        ("types.spec", r#"behavior t "T" { contract "t" }"#),
        (
            "main.spec",
            "use \"zzzzz_completely_unrelated\"\nbehavior foo \"F\" { }",
        ),
    ]);

    let result = resolve_project(dir.path());

    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "E025")
        .collect();
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].suggestion.is_none(),
        "distant path should not produce suggestion: {:?}",
        errors[0].suggestion
    );
}

// --- pub use re-export scope tests ---

#[specforge_test(behavior = "resolve_reexports", verify = "pub use re-exports all entities from target")]
#[test]
fn pub_use_reexports_all_entities() {
    let dir = setup_project(&[
        ("user.spec", r#"behavior User "U" { contract "user" }
behavior UserProfile "UP" { contract "profile" }"#),
        ("barrel.spec", "pub use \"./user\"\n"),
    ]);

    let result = resolve_project(dir.path());

    assert!(
        result.diagnostics.iter().all(|d| d.severity != Severity::Error),
        "pub use should resolve without errors: {:?}",
        result.diagnostics
    );
    let scope = result.file_scopes.get("barrel.spec").expect("missing barrel.spec scope");
    assert!(scope.exported.contains("User"), "exported should contain User");
    assert!(scope.exported.contains("UserProfile"), "exported should contain UserProfile");
}

#[specforge_test(behavior = "resolve_reexports", verify = "pub use selective re-exports only named entities")]
#[test]
fn pub_use_selective_reexport() {
    let dir = setup_project(&[
        ("user.spec", r#"behavior User "U" { contract "user" }
behavior UserProfile "UP" { contract "profile" }"#),
        ("barrel.spec", "pub use { User } from \"./user\"\n"),
    ]);

    let result = resolve_project(dir.path());

    assert!(
        result.diagnostics.iter().all(|d| d.severity != Severity::Error),
        "selective pub use should resolve without errors: {:?}",
        result.diagnostics
    );
    let scope = result.file_scopes.get("barrel.spec").expect("missing barrel.spec scope");
    assert!(scope.exported.contains("User"), "exported should contain User");
    assert!(!scope.exported.contains("UserProfile"), "exported should NOT contain UserProfile");
}

#[specforge_test(behavior = "resolve_reexports", verify = "pub use chains resolve transitively")]
#[test]
fn pub_use_transitive_chain() {
    let dir = setup_project(&[
        ("deep.spec", r#"behavior DeepEntity "D" { contract "deep" }"#),
        ("mid.spec", "pub use \"./deep\"\n"),
        ("top.spec", "pub use \"./mid\"\n"),
    ]);

    let result = resolve_project(dir.path());

    assert!(
        result.diagnostics.iter().all(|d| d.severity != Severity::Error),
        "transitive pub use should resolve: {:?}",
        result.diagnostics
    );
    let top_scope = result.file_scopes.get("top.spec").expect("missing top.spec scope");
    assert!(
        top_scope.exported.contains("DeepEntity"),
        "transitive pub use chain should export DeepEntity; exported: {:?}",
        top_scope.exported
    );
}

#[specforge_test(behavior = "resolve_reexports", verify = "regular use does not re-export")]
#[test]
fn regular_use_does_not_reexport() {
    let dir = setup_project(&[
        ("user.spec", r#"behavior User "U" { contract "user" }"#),
        ("consumer.spec", "use \"./user\"\nbehavior Consumer \"C\" { invariants [User] }"),
    ]);

    let result = resolve_project(dir.path());

    let scope = result.file_scopes.get("consumer.spec").expect("missing consumer.spec scope");
    assert!(scope.declared.contains("Consumer"), "declared should contain Consumer");
    assert!(!scope.exported.contains("User"), "regular use should NOT re-export User");
}

#[specforge_test(behavior = "resolve_reexports", verify = "barrel index with pub use re-exports from sub-files")]
#[test]
fn barrel_index_with_pub_use() {
    let dir = setup_project(&[
        ("models/user.spec", r#"behavior User "U" { contract "user" }"#),
        ("models/order.spec", r#"behavior Order "O" { contract "order" }"#),
        ("models/index.spec", "pub use \"./user\"\npub use \"./order\"\n"),
    ]);

    let result = resolve_project(dir.path());

    assert!(
        result.diagnostics.iter().all(|d| d.severity != Severity::Error),
        "barrel index with pub use should resolve: {:?}",
        result.diagnostics
    );
    let scope = result.file_scopes.get("models/index.spec").expect("missing models/index.spec scope");
    assert!(scope.exported.contains("User"), "barrel should export User");
    assert!(scope.exported.contains("Order"), "barrel should export Order");
}

#[specforge_test(behavior = "resolve_reexports", verify = "selective re-export of unknown entity produces W027")]
#[test]
fn pub_use_unknown_entity_w027() {
    let dir = setup_project(&[
        ("foo.spec", r#"behavior Foo "F" { contract "foo" }"#),
        ("barrel.spec", "pub use { NonExistent } from \"./foo\"\n"),
    ]);

    let result = resolve_project(dir.path());

    let warnings: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "W027")
        .collect();
    assert_eq!(warnings.len(), 1, "should produce W027 for unknown selective re-export");
    assert!(warnings[0].message.contains("NonExistent"));
}

#[specforge_test(behavior = "resolve_reexports", verify = "pub use through cycle participant uses only declared set")]
#[test]
// === symlink safety ===

#[specforge_test(behavior = "resolve_use_imports", verify = "symlink pointing outside spec_root is rejected")]
#[test]
fn symlink_outside_spec_root_rejected() {
    // Create a temp dir with a spec_root subdirectory and a secret file outside it
    let outer = TempDir::new().unwrap();
    let spec_root = outer.path().join("specs");
    let outside = outer.path().join("outside");
    fs::create_dir_all(&spec_root).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(
        outside.join("secret.spec"),
        r#"behavior secret "S" { contract "secret" }"#,
    )
    .unwrap();

    // Create a symlink inside spec_root pointing to the outside directory
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, spec_root.join("escape")).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&outside, spec_root.join("escape")).unwrap();

    // Discover should NOT follow the symlink
    let result = resolve_project(&spec_root);

    // The symlinked file should not be discovered
    let has_secret = result
        .files
        .iter()
        .any(|f| f.spec_file.entities.iter().any(|e| e.id.raw == "secret"));
    assert!(
        !has_secret,
        "symlinked files outside spec_root should not be discovered"
    );
}

#[specforge_test(behavior = "resolve_use_imports", verify = "relative import traversing above spec_root is rejected")]
#[test]
fn relative_import_path_traversal_rejected() {
    let outer = TempDir::new().unwrap();
    let spec_root = outer.path().join("specs");
    fs::create_dir_all(spec_root.join("sub")).unwrap();
    fs::write(
        outer.path().join("outside.spec"),
        r#"behavior outside "O" { contract "outside" }"#,
    )
    .unwrap();
    // A spec file that tries to import ../../outside (above spec_root)
    fs::write(
        spec_root.join("sub").join("main.spec"),
        "use \"../../outside\"\nbehavior inner \"I\" { }",
    )
    .unwrap();

    let result = resolve_project(&spec_root);

    let e025: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "E025")
        .collect();
    assert!(
        !e025.is_empty(),
        "import traversing above spec_root should produce E025"
    );
}

// === M3: W003 import cycle diagnostic carries suggestion ===

#[specforge_test(behavior = "detect_import_cycles", verify = "W003 carries actionable suggestion")]
#[test]
fn w003_import_cycle_has_suggestion() {
    let dir = setup_project(&[
        ("a.spec", "use \"b\"\nbehavior alpha \"A\" { }"),
        ("b.spec", "use \"a\"\nbehavior beta \"B\" { }"),
    ]);

    let result = resolve_project(dir.path());

    let w003s: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "W003")
        .collect();
    assert!(
        !w003s.is_empty(),
        "import cycle should produce W003"
    );
    assert!(
        w003s[0].suggestion.is_some(),
        "W003 should carry an actionable suggestion, got None"
    );
    assert!(
        w003s[0].suggestion.as_ref().unwrap().contains("break"),
        "W003 suggestion should advise breaking the cycle, got: {:?}",
        w003s[0].suggestion
    );
}

fn pub_use_through_cycle_no_transitive() {
    // a.spec and b.spec form a cycle. c.spec pub-uses a.spec.
    // c should get a.spec's declared entities, but not anything
    // that a.spec might transitively re-export from b.spec.
    let dir = setup_project(&[
        ("a.spec", "use \"./b\"\npub use \"./b\"\nbehavior Alpha \"A\" { }"),
        ("b.spec", "use \"./a\"\nbehavior Beta \"B\" { }"),
        ("c.spec", "pub use \"./a\"\nbehavior Gamma \"G\" { }"),
    ]);

    let result = resolve_project(dir.path());

    let c_scope = result.file_scopes.get("c.spec").expect("missing c.spec scope");
    // c should have Alpha (declared by a.spec) in its exported set
    assert!(c_scope.exported.contains("Alpha"), "c should export Alpha from a.spec");
    // c should also have Gamma (its own declaration)
    assert!(c_scope.exported.contains("Gamma"), "c should export its own Gamma");
}

// === H2: Cross-file duplicate entity ID detection (W063) ===

#[specforge_test(behavior = "link_entity_references", verify = "cross-file duplicate entity ID produces W063")]
#[test]
fn cross_file_duplicate_entity_id_produces_w063() {
    let dir = setup_project(&[
        ("a.spec", r#"behavior alpha "Alpha in file A" { contract "first" }"#),
        ("b.spec", r#"behavior alpha "Alpha in file B" { contract "second" }"#),
    ]);

    let resolved = resolve_project(dir.path());
    let (_, diagnostics) = link_references(&resolved);

    let w063s: Vec<_> = diagnostics.iter().filter(|d| d.code == "W063").collect();
    assert_eq!(
        w063s.len(), 1,
        "cross-file duplicate entity ID should produce exactly one W063, got: {:?}",
        w063s
    );
    assert!(
        w063s[0].message.contains("alpha"),
        "W063 message should mention the duplicate ID 'alpha'"
    );
}

#[specforge_test(behavior = "link_entity_references", verify = "same ID different kind across files does not produce W063")]
#[test]
fn same_id_different_kind_across_files_no_w063() {
    let dir = setup_project(&[
        ("a.spec", r#"behavior alpha "Alpha behavior" { contract "first" }"#),
        ("b.spec", r#"feature alpha "Alpha feature" { problem "different kind" }"#),
    ]);

    let resolved = resolve_project(dir.path());
    let (_, diagnostics) = link_references(&resolved);

    let w063s: Vec<_> = diagnostics.iter().filter(|d| d.code == "W063").collect();
    assert!(
        w063s.is_empty(),
        "same ID with different kind should NOT produce W063, got: {:?}",
        w063s
    );
}
