use specforge_common::Severity;
use specforge_resolver::{
    PathAlias, ResolveConfig, resolve_import, resolve_project, resolve_project_with_config,
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

#[specforge_test(
    behavior = "resolve_use_imports",
    verify = "resolve use path to file on disk"
)]
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
        result
            .diagnostics
            .iter()
            .all(|d| d.severity != Severity::Error),
        "unexpected errors: {:?}",
        result.diagnostics
    );
    assert_eq!(result.files.len(), 2);
}

#[specforge_test(
    behavior = "resolve_use_imports",
    verify = "missing import file produces E025"
)]
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

#[specforge_test(
    behavior = "detect_import_cycles",
    verify = "detect direct cycle between two files"
)]
fn detect_direct_import_cycle() {
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
        "should detect import cycle with W113"
    );
    assert!(
        cycle_warnings
            .iter()
            .all(|d| d.severity == Severity::Warning),
        "import cycles should be warnings, not errors"
    );
}

#[specforge_test(
    behavior = "detect_import_cycles",
    verify = "detect transitive cycle across three files"
)]
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
        .filter(|d| d.code == "W113")
        .collect();
    assert!(
        !cycle_warnings.is_empty(),
        "should detect transitive cycle with W113"
    );
}

#[specforge_test(
    behavior = "detect_import_cycles",
    verify = "non-cyclic files still process when a cycle exists"
)]
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

#[specforge_test(
    behavior = "resolve_use_imports",
    verify = "imports across nested directories resolve correctly"
)]
fn imports_across_nested_directories_resolve_correctly() {
    let dir = setup_project(&[
        (
            "sub/types.spec",
            r#"behavior alpha "A" { contract "first" }"#,
        ),
        (
            "main.spec",
            "use \"sub/types\"\nbehavior beta \"B\" {\n  invariants [alpha]\n}",
        ),
    ]);

    let result = resolve_project(dir.path());

    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| d.severity != Severity::Error),
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

// --- 5-step path resolution cascade ---

#[specforge_test(
    behavior = "resolve_use_imports",
    verify = "resolve use path to file on disk"
)]
fn resolve_relative_dot_slash() {
    let dir = setup_project(&[
        (
            "sub/helper.spec",
            r#"behavior helper "H" { contract "help" }"#,
        ),
        (
            "sub/main.spec",
            "use \"./helper\"\nbehavior user \"U\" { invariants [helper] }",
        ),
    ]);

    let result = resolve_project(dir.path());

    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| d.severity != Severity::Error),
        "relative ./helper should resolve without errors: {:?}",
        result.diagnostics
    );
    assert_eq!(result.files.len(), 2);
}

#[specforge_test(
    behavior = "resolve_use_imports",
    verify = "resolve use path to file on disk"
)]
fn resolve_relative_dot_dot_slash() {
    let dir = setup_project(&[
        (
            "shared.spec",
            r#"behavior shared "S" { contract "shared" }"#,
        ),
        (
            "sub/main.spec",
            "use \"../shared\"\nbehavior user \"U\" { invariants [shared] }",
        ),
    ]);

    let result = resolve_project(dir.path());

    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| d.severity != Severity::Error),
        "relative ../shared should resolve without errors: {:?}",
        result.diagnostics
    );
    assert_eq!(result.files.len(), 2);
}

#[specforge_test(
    behavior = "resolve_use_imports",
    verify = "relative import traversing above spec_root is rejected"
)]
fn resolve_relative_escaping_spec_root() {
    let dir = setup_project(&[(
        "sub/main.spec",
        "use \"../../escape\"\nbehavior user \"U\" { }",
    )]);

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

#[specforge_test(
    behavior = "resolve_use_imports",
    verify = "resolve use path to file on disk"
)]
fn resolve_path_alias() {
    let dir = setup_project(&[
        (
            "lib/shared/utils.spec",
            r#"behavior utils "U" { contract "util" }"#,
        ),
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
        ..ResolveConfig::default()
    };
    let result = resolve_project_with_config(dir.path(), &config);

    // The alias maps the import to the file under lib/shared, not to an
    // extension stub (I004) or a missing file (E025).
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let main = result.files.iter().find(|f| f.path == "main.spec").unwrap();
    assert_eq!(main.import_targets, ["lib/shared/utils.spec"]);

    // Without the alias the same import is taken for an extension.
    let plain = resolve_project(dir.path());
    let main = plain.files.iter().find(|f| f.path == "main.spec").unwrap();
    assert!(main.import_targets.is_empty());
    assert!(plain.diagnostics.iter().any(|d| d.code == "I004"));
}

#[specforge_test(
    behavior = "resolve_use_imports",
    verify = "resolve use path to file on disk"
)]
fn resolve_directory_to_index_spec() {
    let dir = setup_project(&[
        (
            "models/index.spec",
            r#"behavior model "M" { contract "model" }"#,
        ),
        (
            "main.spec",
            "use \"models\"\nbehavior caller \"C\" { invariants [model] }",
        ),
    ]);

    let result = resolve_project(dir.path());

    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| d.severity != Severity::Error),
        "directory import should resolve to index.spec: {:?}",
        result.diagnostics
    );
    // main.spec + models/index.spec = 2 files
    assert_eq!(result.files.len(), 2);
}

#[specforge_test(
    behavior = "resolve_use_imports",
    verify = "resolve use path to file on disk"
)]
fn bare_path_precedence_over_index() {
    let dir = setup_project(&[
        (
            "models.spec",
            r#"behavior direct_model "DM" { contract "direct" }"#,
        ),
        (
            "models/index.spec",
            r#"behavior index_model "IM" { contract "index" }"#,
        ),
        (
            "main.spec",
            "use \"models\"\nbehavior caller \"C\" { invariants [direct_model] }",
        ),
    ]);

    let result = resolve_project(dir.path());

    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| d.severity != Severity::Error),
        "models.spec should take precedence over models/index.spec: {:?}",
        result.diagnostics
    );
    // main.spec imports models.spec (the file), models/index.spec is also discovered
    // The import from main.spec should resolve to models.spec, not models/index.spec
    let main_file = result
        .files
        .iter()
        .find(|f| f.path.ends_with("main.spec"))
        .unwrap();
    assert!(
        main_file.import_targets.iter().any(|t| t == "models.spec"),
        "import should resolve to models.spec, not models/index.spec; targets: {:?}",
        main_file.import_targets
    );
}

#[specforge_test(
    behavior = "resolve_use_imports",
    verify = "resolve extension import path"
)]
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

#[specforge_test(
    behavior = "resolve_use_imports",
    verify = "missing import file produces E025"
)]
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
        errors[0]
            .suggestion
            .as_ref()
            .is_some_and(|s| s.contains("helpers")),
        "E025 should suggest close match 'helpers': {:?}",
        errors[0].suggestion
    );
}

#[specforge_test(
    behavior = "resolve_use_imports",
    verify = "missing import file produces E025"
)]
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

#[specforge_test(
    behavior = "resolve_reexports",
    verify = "pub use re-exports all entities from target"
)]
fn pub_use_reexports_all_entities() {
    let dir = setup_project(&[
        (
            "user.spec",
            r#"behavior User "U" { contract "user" }
behavior UserProfile "UP" { contract "profile" }"#,
        ),
        ("barrel.spec", "pub use \"./user\"\n"),
    ]);

    let result = resolve_project(dir.path());

    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| d.severity != Severity::Error),
        "pub use should resolve without errors: {:?}",
        result.diagnostics
    );
    let scope = result
        .file_scopes
        .get("barrel.spec")
        .expect("missing barrel.spec scope");
    assert!(
        scope.exported.contains("User"),
        "exported should contain User"
    );
    assert!(
        scope.exported.contains("UserProfile"),
        "exported should contain UserProfile"
    );
}

#[specforge_test(
    behavior = "resolve_reexports",
    verify = "pub use selective re-exports only named entities"
)]
fn pub_use_selective_reexport() {
    let dir = setup_project(&[
        (
            "user.spec",
            r#"behavior User "U" { contract "user" }
behavior UserProfile "UP" { contract "profile" }"#,
        ),
        ("barrel.spec", "pub use { User } from \"./user\"\n"),
    ]);

    let result = resolve_project(dir.path());

    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| d.severity != Severity::Error),
        "selective pub use should resolve without errors: {:?}",
        result.diagnostics
    );
    let scope = result
        .file_scopes
        .get("barrel.spec")
        .expect("missing barrel.spec scope");
    assert!(
        scope.exported.contains("User"),
        "exported should contain User"
    );
    assert!(
        !scope.exported.contains("UserProfile"),
        "exported should NOT contain UserProfile"
    );
}

#[specforge_test(
    behavior = "resolve_reexports",
    verify = "pub use chains resolve transitively"
)]
fn pub_use_transitive_chain() {
    let dir = setup_project(&[
        (
            "deep.spec",
            r#"behavior DeepEntity "D" { contract "deep" }"#,
        ),
        ("mid.spec", "pub use \"./deep\"\n"),
        ("top.spec", "pub use \"./mid\"\n"),
    ]);

    let result = resolve_project(dir.path());

    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| d.severity != Severity::Error),
        "transitive pub use should resolve: {:?}",
        result.diagnostics
    );
    let top_scope = result
        .file_scopes
        .get("top.spec")
        .expect("missing top.spec scope");
    assert!(
        top_scope.exported.contains("DeepEntity"),
        "transitive pub use chain should export DeepEntity; exported: {:?}",
        top_scope.exported
    );
}

#[specforge_test(
    behavior = "resolve_reexports",
    verify = "regular use does not re-export"
)]
fn regular_use_does_not_reexport() {
    let dir = setup_project(&[
        ("user.spec", r#"behavior User "U" { contract "user" }"#),
        (
            "consumer.spec",
            "use \"./user\"\nbehavior Consumer \"C\" { invariants [User] }",
        ),
    ]);

    let result = resolve_project(dir.path());

    let scope = result
        .file_scopes
        .get("consumer.spec")
        .expect("missing consumer.spec scope");
    assert!(
        scope.declared.contains("Consumer"),
        "declared should contain Consumer"
    );
    assert!(
        !scope.exported.contains("User"),
        "regular use should NOT re-export User"
    );
}

#[specforge_test(
    behavior = "resolve_reexports",
    verify = "barrel index with pub use re-exports from sub-files"
)]
fn barrel_index_with_pub_use() {
    let dir = setup_project(&[
        (
            "models/user.spec",
            r#"behavior User "U" { contract "user" }"#,
        ),
        (
            "models/order.spec",
            r#"behavior Order "O" { contract "order" }"#,
        ),
        (
            "models/index.spec",
            "pub use \"./user\"\npub use \"./order\"\n",
        ),
    ]);

    let result = resolve_project(dir.path());

    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| d.severity != Severity::Error),
        "barrel index with pub use should resolve: {:?}",
        result.diagnostics
    );
    let scope = result
        .file_scopes
        .get("models/index.spec")
        .expect("missing models/index.spec scope");
    assert!(scope.exported.contains("User"), "barrel should export User");
    assert!(
        scope.exported.contains("Order"),
        "barrel should export Order"
    );
}

#[specforge_test(
    behavior = "resolve_reexports",
    verify = "selective re-export of unknown entity produces W027"
)]
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
    assert_eq!(
        warnings.len(),
        1,
        "should produce W027 for unknown selective re-export"
    );
    assert!(warnings[0].message.contains("NonExistent"));
}

// === symlink safety ===

#[specforge_test(
    behavior = "resolve_use_imports",
    verify = "symlink pointing outside spec_root is rejected"
)]
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

#[specforge_test(
    behavior = "resolve_use_imports",
    verify = "relative import traversing above spec_root is rejected"
)]
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

// === M3: W113 import cycle diagnostic carries suggestion ===

#[specforge_test(
    behavior = "detect_import_cycles",
    verify = "W113 carries actionable suggestion"
)]
fn w113_import_cycle_has_suggestion() {
    let dir = setup_project(&[
        ("a.spec", "use \"b\"\nbehavior alpha \"A\" { }"),
        ("b.spec", "use \"a\"\nbehavior beta \"B\" { }"),
    ]);

    let result = resolve_project(dir.path());

    let w113s: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "W113")
        .collect();
    assert!(!w113s.is_empty(), "import cycle should produce W113");
    assert!(
        w113s[0].suggestion.is_some(),
        "W113 should carry an actionable suggestion, got None"
    );
    assert!(
        w113s[0].suggestion.as_ref().unwrap().contains("break"),
        "W113 suggestion should advise breaking the cycle, got: {:?}",
        w113s[0].suggestion
    );
}

#[specforge_test(
    behavior = "resolve_reexports",
    verify = "pub use through cycle participant uses only declared set"
)]
fn pub_use_through_cycle_no_transitive() {
    // a.spec and b.spec form a cycle. c.spec pub-uses a.spec.
    // c should get a.spec's declared entities, but not anything
    // that a.spec might transitively re-export from b.spec.
    let dir = setup_project(&[
        (
            "a.spec",
            "use \"./b\"\npub use \"./b\"\nbehavior Alpha \"A\" { }",
        ),
        ("b.spec", "use \"./a\"\nbehavior Beta \"B\" { }"),
        ("c.spec", "pub use \"./a\"\nbehavior Gamma \"G\" { }"),
    ]);

    let result = resolve_project(dir.path());

    let c_scope = result
        .file_scopes
        .get("c.spec")
        .expect("missing c.spec scope");
    // c should have Alpha (declared by a.spec) in its exported set
    assert!(
        c_scope.exported.contains("Alpha"),
        "c should export Alpha from a.spec"
    );
    // c should also have Gamma (its own declaration)
    assert!(
        c_scope.exported.contains("Gamma"),
        "c should export its own Gamma"
    );
    // ...and nothing a.spec re-exports from b.spec through the cycle.
    assert!(
        !c_scope.exported.contains("Beta"),
        "Beta reaches c only through the a<->b cycle: {:?}",
        c_scope.exported
    );
    let mut exported: Vec<&String> = c_scope.exported.iter().collect();
    exported.sort();
    assert_eq!(exported, ["Alpha", "Gamma"]);
}

#[specforge_test(
    behavior = "resolve_use_imports",
    verify = "an import path may spell out the .spec extension"
)]
fn use_import_may_spell_out_the_spec_extension() {
    let dir = setup_project(&[
        ("tokens.spec", r#"behavior token "T" { contract "t" }"#),
        ("main.spec", "use \"tokens.spec\"\nbehavior foo \"F\" { }"),
        ("other.spec", "use \"tokens\"\nbehavior bar \"B\" { }"),
    ]);

    let result = resolve_project(dir.path());

    assert!(
        !result.diagnostics.iter().any(|d| d.code == "E025"),
        "{:?}",
        result.diagnostics
    );
}

/// W113 names each cycle from its smallest path, and the cycles come in
/// sorted order, whatever order the files were walked in (ADR 0004 D1-a).
#[specforge_test(
    invariant = "import_dag",
    verify = "a circular import produces W113 naming the cycle participants in a deterministic order"
)]
fn w113_names_each_cycle_the_same_way_on_every_run() {
    let dir = setup_project(&[
        ("c.spec", "use \"a\"\nbehavior gamma \"G\" { }"),
        ("a.spec", "use \"b\"\nbehavior alpha \"A\" { }"),
        ("b.spec", "use \"c\"\nbehavior beta \"B\" { }"),
        ("y.spec", "use \"x\"\nbehavior yankee \"Y\" { }"),
        ("x.spec", "use \"y\"\nbehavior xray \"X\" { }"),
    ]);

    for _ in 0..20 {
        let result = resolve_project(dir.path());
        let messages: Vec<&str> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W113")
            .map(|d| d.message.as_str())
            .collect();
        assert_eq!(
            messages,
            [
                "circular import detected: a.spec -> b.spec -> c.spec",
                "circular import detected: x.spec -> y.spec",
            ]
        );
    }
}

/// `exclude` entries are path substrings relative to the spec root, not
/// globs (ADR 0004 D1-b).
#[specforge_test(
    behavior = "resolve_use_imports",
    verify = "files matching an exclude entry are not compiled"
)]
fn excluded_files_are_not_compiled() {
    let dir = setup_project(&[
        ("main.spec", "behavior alpha \"A\" { }"),
        ("drafts/draft.spec", "behavior alpha \"A again\" { }"),
    ]);
    let paths = |exclude: &[&str]| -> Vec<String> {
        let config = ResolveConfig {
            exclude: exclude.iter().map(|s| s.to_string()).collect(),
            ..ResolveConfig::default()
        };
        let mut paths: Vec<String> = resolve_project_with_config(dir.path(), &config)
            .files
            .into_iter()
            .map(|f| f.path)
            .collect();
        paths.sort();
        paths
    };

    assert_eq!(paths(&[]), ["drafts/draft.spec", "main.spec"]);
    assert_eq!(paths(&["drafts/"]), ["main.spec"]);
    assert_eq!(paths(&["drafts/**"]), ["drafts/draft.spec", "main.spec"]);
}

/// One import, resolved on its own (the LSP's go-to-definition on a `use`
/// path), the way the compile resolves it: bare, relative, alias and
/// directory-index targets, relative to the spec root.
#[specforge_test(
    behavior = "resolve_use_imports",
    verify = "resolve use path to file on disk"
)]
fn resolve_one_import_by_the_compile_cascade() {
    let dir = setup_project(&[
        ("types.spec", "term t \"T\" {\n}\n"),
        ("models/index.spec", "term m \"M\" {\n}\n"),
        ("lib/shared/utils.spec", "term u \"U\" {\n}\n"),
        ("sub/main.spec", "term main \"Main\" {\n}\n"),
    ]);
    let root = dir.path();
    let config = ResolveConfig {
        path_aliases: vec![PathAlias {
            alias: "shared".to_string(),
            target: "lib/shared".to_string(),
        }],
        ..ResolveConfig::default()
    };
    let resolve = |import: &str| resolve_import(root, "sub/main.spec", import, &config);

    assert_eq!(resolve("types").as_deref(), Some("types.spec"));
    assert_eq!(resolve("types.spec").as_deref(), Some("types.spec"));
    assert_eq!(resolve("../types").as_deref(), Some("types.spec"));
    assert_eq!(resolve("models").as_deref(), Some("models/index.spec"));
    assert_eq!(
        resolve("@shared/utils").as_deref(),
        Some("lib/shared/utils.spec")
    );
    assert_eq!(resolve("@specforge/software"), None, "an extension");
    assert_eq!(resolve("missing"), None);
}

/// Whichever cascade step names it, a target outside the spec root does
/// not resolve: E025, as for a relative import.
#[specforge_test(
    behavior = "resolve_use_imports",
    verify = "an import reaching above spec_root by any cascade step produces E025"
)]
fn no_import_reaches_above_the_spec_root() {
    let outer = TempDir::new().unwrap();
    fs::write(outer.path().join("outside.spec"), "term o \"O\" {\n}\n").unwrap();
    let root = outer.path().join("spec");
    fs::create_dir_all(&root).unwrap();
    for import in ["../outside", "sub/../../outside", "@up/../outside"] {
        fs::write(
            root.join("main.spec"),
            format!("use \"{import}\"\nterm main \"Main\" {{\n}}\n"),
        )
        .unwrap();
        let config = ResolveConfig {
            path_aliases: vec![PathAlias {
                alias: "up".to_string(),
                target: "..".to_string(),
            }],
            ..ResolveConfig::default()
        };
        assert_eq!(resolve_import(&root, "main.spec", import, &config), None);
        let result = resolve_project_with_config(&root, &config);
        assert!(
            result.diagnostics.iter().any(|d| d.code == "E025"),
            "{import}: {:?}",
            result.diagnostics
        );
    }
}
