//! `specforge_ops::migrate::run` through the runtime the extensions are served
//! by: the hooks they declare run in it, in dependency order, and a failing
//! hook or a changed graph structure restores the migrated files.

use std::path::Path;

use specforge_extension_sdk::{ContributionsBuilder, ExtensionMeta};
use specforge_migrate::{CURRENT_FORMAT_VERSION, migrate_project};
use specforge_ops::OpErrorKind;
use specforge_ops::migrate::{MigrationInput, Request, invoke_hooks, parse_target, rollback, run};
use specforge_protocol_types::ExtensionDeclaration;
use specforge_test_macros::test as specforge_test;
use specforge_wasm::testing::InProcessRuntime;

const OLD: &str = "// specforge-format: 0.9\nbehavior alpha \"Alpha\" {\n}\n";

fn project() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name": "p", "version": "0.1.0", "extensions": []}"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("old.spec"), OLD).unwrap();
    dir
}

fn request(root: &Path) -> Request<'_> {
    Request {
        root,
        target: CURRENT_FORMAT_VERSION,
        dry_run: false,
        no_backup: false,
    }
}

/// A project enabling `@acme/x`, with one file at the old format version.
fn project_with_extension() -> tempfile::TempDir {
    let dir = project();
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name": "p", "version": "0.1.0", "extensions": ["@acme/x"]}"#,
    )
    .unwrap();
    dir
}

/// `@acme/x`, served in process, declaring its migration hook
/// `migrate_acme` with `handler`; the runtime records every export the host
/// calls.
fn hooked(
    handler: impl Fn(&MigrationInput) -> Result<(), String> + Clone + Send + Sync + 'static,
) -> InProcessRuntime {
    InProcessRuntime::new().with(move || {
        let handler = handler.clone();
        let mut c = ContributionsBuilder::new(ExtensionMeta::new("@acme/x", "1.0.0"));
        c.migration_hook_handler("migrate_acme", move |input| handler(input));
        c
    })
}

/// `@acme/x` declaring no hook at all.
fn unhooked() -> InProcessRuntime {
    InProcessRuntime::new()
        .with(|| ContributionsBuilder::new(ExtensionMeta::new("@acme/x", "1.0.0")))
}

fn exports_called(runtime: &InProcessRuntime) -> Vec<String> {
    runtime.calls().into_iter().map(|c| c.export).collect()
}

#[specforge_test(
    behavior = "rollback_failed_migration",
    verify = "a migration whose graph changes structure is rolled back automatically"
)]
fn a_migration_that_changes_the_graph_is_rolled_back() {
    let dir = project_with_extension();
    let file = dir.path().join("old.spec");

    // A hook that renames the entity: the graph loses `alpha`.
    let runtime = hooked(|input| {
        let file = &input.files[0];
        let migrated = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
        std::fs::write(file, migrated.replace("alpha", "beta")).map_err(|e| e.to_string())
    });
    let outcome = run(&request(dir.path()), Some(&runtime));

    assert_eq!(outcome.hooks_invoked, ["@acme/x:migrate_acme"]);
    assert!(outcome.validated);
    assert!(
        outcome
            .structural_differences
            .iter()
            .any(|d| d.code == "W054" && d.message.contains("alpha")),
        "{:?}",
        outcome.structural_differences
    );
    let rollback = outcome.rollback.as_ref().expect("rolled back");
    assert_eq!(rollback.restored_count, 1, "{rollback:?}");
    assert!(!outcome.ok() && !outcome.migrated());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), OLD);
}

#[specforge_test(
    behavior = "rollback_failed_migration",
    verify = "a migration whose extension hook fails is rolled back automatically"
)]
fn a_migration_whose_hook_fails_is_rolled_back() {
    let dir = project_with_extension();
    let runtime = hooked(|_| Err("the data cannot be migrated".into()));

    let outcome = run(&request(dir.path()), Some(&runtime));

    assert!(!outcome.validated, "no validation after a failed hook");
    assert_eq!(
        outcome.hook_failures.len(),
        1,
        "{:?}",
        outcome.hook_failures
    );
    assert!(
        outcome.hook_failures[0].contains("migrate_acme() of '@acme/x' trapped")
            && outcome.hook_failures[0].contains("the data cannot be migrated"),
        "{:?}",
        outcome.hook_failures
    );
    assert_eq!(outcome.rollback.as_ref().unwrap().restored_count, 1);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("old.spec")).unwrap(),
        OLD
    );
}

#[test]
fn a_failed_migration_names_its_kind() {
    // Applied: nothing failed, no failure.
    let dir = project();
    let outcome = run(&request(dir.path()), None);
    assert!(outcome.ok());
    assert_eq!(outcome.failure(), None);

    // A file that cannot migrate: the run failed on its own side.
    let dir = project();
    std::fs::write(
        dir.path().join("bad.spec"),
        "// specforge-format: 99.0\nbehavior bad \"Bad\" {\n}\n",
    )
    .unwrap();
    let outcome = run(&request(dir.path()), None);
    let failure = outcome.failure().expect("a file failed to migrate");
    assert!(!outcome.ok());
    assert_eq!(
        (
            failure.kind,
            failure.code.as_ref(),
            failure.message.as_str()
        ),
        (
            OpErrorKind::Internal,
            "migration_failed",
            "the migration failed"
        )
    );

    // A hook that fails: rolled back, the project never compiled again.
    let dir = project_with_extension();
    let runtime = hooked(|_| Err("trapped".into()));
    let outcome = run(&request(dir.path()), Some(&runtime));
    assert!(outcome.rollback.is_some() && !outcome.ok());
    assert_eq!(outcome.failure().unwrap().kind, OpErrorKind::Internal);

    // A hook that leaves the project not compiling: rolled back with the
    // errors the migrated project reported.
    let dir = project_with_extension();
    let runtime =
        hooked(|input| std::fs::write(&input.files[0], "behavior {\n").map_err(|e| e.to_string()));
    let outcome = run(&request(dir.path()), Some(&runtime));
    assert!(outcome.post_errors().next().is_some(), "{outcome:?}");
    assert!(outcome.rollback.is_some());
    let failure = outcome.failure().expect("rolled back");
    assert_eq!(
        (failure.kind, failure.message.as_str()),
        (
            OpErrorKind::CompilationFailed,
            "the migrated project does not compile"
        )
    );
}

#[test]
fn a_header_only_migration_keeps_the_graph_and_stays_applied() {
    let dir = project();

    let outcome = run(&request(dir.path()), None);

    assert!(outcome.migrated(), "{outcome:?}");
    assert!(outcome.validated);
    assert!(outcome.structural_differences.is_empty());
    assert!(outcome.rollback.is_none());
    assert_eq!(outcome.from.to_string(), "0.9");
}

#[test]
fn nothing_pending_runs_nothing() {
    let dir = project_with_extension();
    std::fs::write(dir.path().join("old.spec"), "behavior alpha \"A\" {\n}\n").unwrap();
    let runtime = hooked(|_| Ok(()));

    let outcome = run(&request(dir.path()), Some(&runtime));

    assert!(!outcome.pending && !outcome.applied && !outcome.validated);
    assert!(
        runtime.calls().is_empty(),
        "no compile and no hook with nothing to migrate: {:?}",
        exports_called(&runtime)
    );
}

#[test]
fn a_target_newer_than_supported_or_malformed_is_e019() {
    assert_eq!(parse_target(Some("99.0")).unwrap_err().code, "E019");
    assert_eq!(parse_target(Some("latest")).unwrap_err().code, "E019");
    assert_eq!(parse_target(None).unwrap(), CURRENT_FORMAT_VERSION);
}

#[specforge_test(
    behavior = "migrate_spec_files_in_place",
    verify = "only the project's sources are migrated: under spec_root, without excluded files"
)]
fn migrate_reads_the_project_sources() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(
        root.join("specforge.json"),
        r#"{"spec_root": "specs", "exclude": ["drafts"]}"#,
    )
    .unwrap();
    for file in [
        "specs/a.spec",
        "specs/drafts/d.spec",
        "spec/old.spec",
        "fixtures/fx.spec",
    ] {
        std::fs::create_dir_all(root.join(file).parent().unwrap()).unwrap();
        std::fs::write(root.join(file), OLD).unwrap();
    }

    let summary = migrate_project(root, &CURRENT_FORMAT_VERSION, true, true);

    let visited: Vec<&str> = summary
        .results
        .iter()
        .map(|r| r.file_path.as_str())
        .collect();
    assert_eq!(visited.len(), 1, "{visited:?}");
    assert!(visited[0].ends_with("specs/a.spec"), "{visited:?}");
    // Rollback looks at the same files (none has a backup: a dry run).
    let rollback = rollback(root);
    assert_eq!(rollback.skipped_count, 1, "{rollback:?}");
    assert!(
        rollback.results[0].file_path.ends_with("specs/a.spec"),
        "{rollback:?}"
    );
}

#[specforge_test(
    behavior = "invoke_extension_migration_hooks",
    verify = "extension with migration_hook field has it invoked during migrate"
)]
fn the_hook_an_extension_declares_in_its_handshake_runs_on_migrate() {
    let dir = project_with_extension();
    let runtime = hooked(|_| Ok(()));

    let outcome = run(&request(dir.path()), Some(&runtime));

    assert!(outcome.migrated(), "{outcome:?}");
    assert_eq!(outcome.hooks_invoked, ["@acme/x:migrate_acme"]);
    assert!(outcome.hook_failures.is_empty(), "{outcome:?}");
    assert!(exports_called(&runtime).contains(&"migrate_acme".to_string()));
}

#[specforge_test(
    behavior = "invoke_extension_migration_hooks",
    verify = "extension without migration_hook field is skipped silently"
)]
fn an_extension_whose_handshake_names_no_hook_is_skipped_silently() {
    let dir = project_with_extension();
    let runtime = unhooked();

    let outcome = run(&request(dir.path()), Some(&runtime));

    assert!(outcome.migrated(), "{outcome:?}");
    assert!(outcome.hooks_invoked.is_empty() && outcome.hook_failures.is_empty());
    assert!(
        exports_called(&runtime).iter().all(|c| c.starts_with("__")),
        "only the protocol exports are called"
    );
    assert!(
        !outcome
            .post_diagnostics
            .iter()
            .any(|d| d.message.contains("hook")),
        "{outcome:?}"
    );
}

/// Two extensions with hooks, `@acme/a`'s panicking.
fn hooks() -> InProcessRuntime {
    InProcessRuntime::new()
        .with(|| {
            let mut c = ContributionsBuilder::new(ExtensionMeta::new("@acme/a", "1.0.0"));
            c.migration_hook_handler("migrate_a", |_| panic!("hook panicked"));
            c
        })
        .with(|| {
            let mut c = ContributionsBuilder::new(ExtensionMeta::new("@acme/b", "1.0.0"));
            c.migration_hook_handler("migrate_b", |_| Ok(()));
            c
        })
}

fn manifest(name: &str, hook: &str) -> ExtensionDeclaration {
    ExtensionDeclaration {
        handshake: specforge_protocol_types::HandshakeResponse {
            name: name.into(),
            version: "1.0.0".into(),
            migration_hook: Some(hook.into()),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[specforge_test(
    behavior = "invoke_extension_migration_hooks",
    verify = "a migration hook receives the from and to format versions and the migrated files"
)]
fn a_hook_receives_the_versions_and_the_migrated_files() {
    use std::sync::{Arc, Mutex};
    // The hook's handler records what it decoded.
    let seen: Arc<Mutex<Vec<MigrationInput>>> = Arc::default();
    let recorder = Arc::clone(&seen);
    let runtime = InProcessRuntime::new().with(move || {
        let recorder = Arc::clone(&recorder);
        let mut c = ContributionsBuilder::new(ExtensionMeta::new("@acme/a", "1.0.0"));
        c.migration_hook_handler("migrate_a", move |input| {
            recorder.lock().unwrap().push(input.clone());
            Ok(())
        });
        c
    });
    let input = MigrationInput {
        from: "0.9".into(),
        to: "1.0".into(),
        files: vec!["old.spec".into()],
    };

    invoke_hooks(&[manifest("@acme/a", "migrate_a")], &runtime, &input);

    assert_eq!(*seen.lock().unwrap(), [input]);
    assert_eq!(
        runtime.calls()[0].input,
        serde_json::json!({"from": "0.9", "to": "1.0", "files": ["old.spec"]})
    );
}

#[specforge_test(
    behavior = "invoke_extension_migration_hooks",
    verify = "the nine builtin extensions' hooks run in dependency order with no failure"
)]
fn the_builtin_extensions_hooks_run_without_a_dependency_failure() {
    // Every builtin loaded together, as a project enabling all of them
    // does: the peer graph (optional peers included) must sort, or the
    // hooks never run and a migration rolls back.
    let names: Vec<&str> = specforge_component::builtins::BUILTIN_EXTENSIONS
        .iter()
        .map(|(name, _)| *name)
        .collect();
    assert_eq!(names.len(), 9);
    let dir = tempfile::TempDir::new().unwrap();
    let config = serde_json::json!({"name": "p", "version": "0.1.0", "extensions": names});
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    let runtime = specforge_component::project_runtime(dir.path());
    let declarations: Vec<ExtensionDeclaration> = names
        .iter()
        .map(|name| {
            specforge_wasm::protocol::load_declaration(&runtime, name)
                .unwrap()
                .declaration
        })
        .collect();
    let input = MigrationInput {
        from: "0.9".into(),
        to: "1.0".into(),
        files: Vec::new(),
    };

    let (_, failures) = invoke_hooks(&declarations, &runtime, &input);

    assert!(failures.is_empty(), "{failures:?}");
    assert!(specforge_wasm::topological_sort_extensions(&declarations).is_ok());
}

#[specforge_test(
    behavior = "invoke_extension_migration_hooks",
    verify = "hook that traps collects WasmTrapInfo and continues"
)]
fn a_trapping_hook_is_recorded_and_the_next_one_still_runs() {
    let runtime = hooks();
    let manifests = [
        manifest("@acme/a", "migrate_a"),
        manifest("@acme/b", "migrate_b"),
    ];

    let input = MigrationInput {
        from: "0.9".into(),
        to: "1.0".into(),
        files: Vec::new(),
    };
    let (invoked, failures) = invoke_hooks(&manifests, &runtime, &input);

    assert_eq!(invoked, ["@acme/b:migrate_b"]);
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(
        failures[0].contains("unreachable") && failures[0].contains("hook panicked"),
        "{failures:?}"
    );
    let called: Vec<String> = runtime
        .calls()
        .into_iter()
        .map(|c| format!("{}:{}", c.extension, c.export))
        .collect();
    assert_eq!(called, ["@acme/a:migrate_a", "@acme/b:migrate_b"]);
}
