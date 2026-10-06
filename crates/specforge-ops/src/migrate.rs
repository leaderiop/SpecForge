//! `specforge migrate` and the MCP `specforge.migrate` tool: one migration.
//!
//! The run previews first: a project with nothing to migrate is left alone
//! (no compile, no hooks). Otherwise it compiles the project, migrates the
//! files, runs the extensions' migration hooks, and compiles again. When a
//! hook fails, or the graph's structure changed, the migrated files are
//! restored from their backups.

use specforge_common::{Diagnostic, Severity};
use specforge_migrate::{
    CURRENT_FORMAT_VERSION, FormatVersion, MAX_SUPPORTED_VERSION, MIN_SUPPORTED_VERSION,
    MigrationSummary, RollbackSummary, check_schema_compatibility, compare_graphs, migrate_project,
    run_rollback,
};
use specforge_project::CompiledProject;
use specforge_protocol_types::ExtensionDeclaration;
use specforge_wasm::WasmRuntime;
use std::path::Path;

use crate::{OpError, Writes};

/// The format version to migrate to: `raw`, checked, else the current one.
/// A version that doesn't parse, or one newer than this build supports, is
/// E019.
pub fn parse_target(raw: Option<&str>) -> Result<FormatVersion, OpError> {
    let Some(raw) = raw else {
        return Ok(CURRENT_FORMAT_VERSION);
    };
    let supported = format!(
        "Use a format version between {MIN_SUPPORTED_VERSION} and {MAX_SUPPORTED_VERSION}."
    );
    match raw.parse::<FormatVersion>() {
        Ok(version) if version > MAX_SUPPORTED_VERSION => Err(OpError::new(
            "E019",
            format!("unsupported target version {raw} (max supported: {MAX_SUPPORTED_VERSION})"),
        )
        .with_suggestion(supported)),
        Ok(version) => Ok(version),
        Err(e) => Err(
            OpError::new("E019", format!("invalid target version '{raw}': {e}"))
                .with_suggestion(supported),
        ),
    }
}

pub struct Request<'a> {
    pub root: &'a Path,
    pub target: FormatVersion,
    /// Report what would change; write nothing.
    pub dry_run: bool,
    /// Skip the `.bak` copy of each migrated file (nothing to roll back to).
    pub no_backup: bool,
}

#[derive(Debug, Clone)]
pub struct Outcome {
    /// The oldest format version among the project's files.
    pub from: FormatVersion,
    pub to: FormatVersion,
    /// Whether any file needed migrating. When not, nothing else ran.
    pub pending: bool,
    /// Whether files were rewritten (not a dry run, and not every file
    /// failed). A rolled-back migration was applied, then undone.
    pub applied: bool,
    /// The per-file migration (the preview, in a dry run or with nothing
    /// pending).
    pub summary: MigrationSummary,
    /// `extension:hook` for each migration hook that ran.
    pub hooks_invoked: Vec<String>,
    /// Why each failing hook failed.
    pub hook_failures: Vec<String>,
    /// Whether the migrated project was compiled and compared.
    pub validated: bool,
    /// Breaking Graph Protocol schema changes (W053).
    pub schema_warnings: Vec<Diagnostic>,
    /// How the migrated graph differs from the one before (W054).
    pub structural_differences: Vec<Diagnostic>,
    /// What compiling the migrated project reported.
    pub post_diagnostics: Vec<Diagnostic>,
    /// The restore, when the migration was rolled back.
    pub rollback: Option<RollbackSummary>,
    /// The files the run left changed: each migrated file and each backup;
    /// after a rollback, the backups (the migrated files hold their old
    /// text again). Nothing for a dry run or with nothing pending.
    pub writes: Writes,
}

impl Outcome {
    /// Whether the migrated files are on disk now.
    pub fn migrated(&self) -> bool {
        self.applied && self.summary.migrated_count > 0 && self.rollback.is_none()
    }

    /// Whether the run failed: a file failed to migrate, or it rolled back.
    pub fn failed(&self) -> bool {
        self.summary.failed_count > 0 || self.rollback.is_some()
    }

    /// The errors compiling the migrated project reported.
    pub fn post_errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.post_diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
    }
}

/// The migration hooks that ran (`extension:hook`), and why any failed.
pub type HookRun = (Vec<String>, Vec<String>);

/// What a migration hook is called with (the protocol's
/// `MigrationInput`): the format versions the project moves between and
/// the files the core migration rewrote.
pub use specforge_protocol_types::MigrationInput;

/// Migrate the project, running its extensions (and their migration hooks)
/// in `runtime`.
pub fn run(request: &Request, runtime: Option<&dyn WasmRuntime>) -> Outcome {
    run_with_hooks(request, runtime, &mut |declarations, input| match runtime {
        Some(runtime) => invoke_hooks(declarations, runtime, input),
        None => (Vec::new(), Vec::new()),
    })
}

/// [`run`] with the hook step supplied: given the loaded declarations, it
/// returns the hooks it ran and why any failed.
pub fn run_with_hooks(
    request: &Request,
    runtime: Option<&dyn WasmRuntime>,
    hooks: &mut dyn FnMut(&[ExtensionDeclaration], &MigrationInput) -> HookRun,
) -> Outcome {
    let root = request.root;
    let target = &request.target;
    let preview = migrate_project(root, target, true, true);
    let from = preview
        .results
        .iter()
        .filter_map(|r| r.from_version.clone())
        .min()
        .unwrap_or_else(|| target.clone());
    let pending = preview.migrated_count > 0 || preview.failed_count > 0;
    let mut outcome = Outcome {
        from,
        to: target.clone(),
        pending,
        applied: false,
        summary: preview,
        hooks_invoked: Vec::new(),
        hook_failures: Vec::new(),
        validated: false,
        schema_warnings: Vec::new(),
        structural_differences: Vec::new(),
        post_diagnostics: Vec::new(),
        rollback: None,
        writes: Writes::none(),
    };
    if !pending || request.dry_run {
        return outcome;
    }

    // The graph and schema before any file is touched.
    let pre = CompiledProject::compile(root, runtime);
    let pre_schema = schema_of(&pre);

    outcome.summary = migrate_project(root, target, false, request.no_backup);
    outcome.applied = true;
    outcome.writes = summary_writes(&outcome.summary);
    if outcome.summary.failed_count > 0 {
        return outcome;
    }

    // Extension hooks run on the migrated files, before validation.
    let input = MigrationInput {
        from: outcome.from.to_string(),
        to: outcome.to.to_string(),
        files: outcome
            .summary
            .results
            .iter()
            .filter(|r| r.status == specforge_migrate::MigrationStatus::Migrated)
            .map(|r| r.file_path.clone())
            .collect(),
    };
    let (invoked, failures) = hooks(pre.env.registries.declarations(), &input);
    outcome.hooks_invoked = invoked;
    outcome.hook_failures = failures;
    if !outcome.hook_failures.is_empty() {
        roll_back(root, &mut outcome);
        return outcome;
    }

    // The migrated project must have the graph it had before.
    let post = CompiledProject::compile(root, runtime);
    outcome.validated = true;
    outcome.schema_warnings = check_schema_compatibility(&pre_schema, &schema_of(&post));
    outcome.structural_differences = compare_graphs(&pre.graph, &post.graph);
    outcome.post_diagnostics = post.diagnostics();
    if !outcome.structural_differences.is_empty() {
        roll_back(root, &mut outcome);
    }
    outcome
}

/// What `migrate_project` wrote: each file it migrated and each backup
/// it made.
fn summary_writes(summary: &MigrationSummary) -> Writes {
    let migrated = summary
        .results
        .iter()
        .filter(|r| r.status == specforge_migrate::MigrationStatus::Migrated)
        .map(|r| r.file_path.as_str());
    let backups = summary.backups.iter().map(|b| b.backup_path.as_str());
    migrated.chain(backups).collect()
}

/// Restore the project's files from their backups after a failed check: a
/// file this run migrated holds its old text again and is forgotten; any
/// other file a backup restored was rewritten, and is recorded.
fn roll_back(root: &Path, outcome: &mut Outcome) {
    let summary = run_rollback(root);
    for restored in summary
        .results
        .iter()
        .filter(|r| r.status == specforge_migrate::MigrationStatus::Restored)
    {
        let path = Path::new(&restored.file_path);
        let migrated_here = outcome.summary.results.iter().any(|r| {
            r.status == specforge_migrate::MigrationStatus::Migrated
                && r.file_path == restored.file_path
        });
        if migrated_here {
            outcome.writes.forget(path);
        } else {
            outcome.writes.record(path);
        }
    }
    outcome.rollback = Some(summary);
}

/// What a rollback rewrote: each file it restored from its backup.
pub fn restored(summary: &RollbackSummary) -> Writes {
    summary
        .results
        .iter()
        .filter(|r| r.status == specforge_migrate::MigrationStatus::Restored)
        .map(|r| r.file_path.as_str())
        .collect()
}

/// Restore every migrated file from its `.bak` backup.
pub fn rollback(root: &Path) -> RollbackSummary {
    run_rollback(root)
}

fn schema_of(project: &CompiledProject) -> specforge_emitter::GraphProtocolSchema {
    let registries = &project.env.registries;
    specforge_emitter::generate_schema(
        &registries.kinds,
        &registries.edges,
        &registries.fields,
        &registries
            .extension_info()
            .map(|(name, version)| (name.to_string(), version.to_string()))
            .collect::<Vec<_>>(),
    )
}

/// Run each extension's declared migration hook, in dependency order. A
/// hook that fails (E028: it trapped, or the extension does not route it)
/// is recorded and the rest still run. Returns the hooks
/// run (`extension:hook`) and the failures.
pub fn invoke_hooks(
    declarations: &[ExtensionDeclaration],
    runtime: &dyn WasmRuntime,
    input: &MigrationInput,
) -> HookRun {
    let calls = specforge_wasm::ExtensionCalls::new(runtime);

    let order = match specforge_wasm::topological_sort_extensions(declarations) {
        Ok(order) => order,
        Err(diagnostics) => {
            let reason = diagnostics
                .first()
                .map(|d| d.message.clone())
                .unwrap_or_else(|| "dependency cycle".to_string());
            return (Vec::new(), vec![reason]);
        }
    };
    let mut invoked = Vec::new();
    let mut failures = Vec::new();
    for name in &order {
        let Some(declaration) = declarations.iter().find(|d| d.name() == name) else {
            continue;
        };
        let Some(hook) = declaration
            .handshake
            .migration_hook
            .as_deref()
            .filter(|h| !h.is_empty())
        else {
            continue;
        };
        // The hook's answer is not read: the protocol defines none.
        match calls.migrate(name, hook, input) {
            Ok(()) => invoked.push(format!("{name}:{hook}")),
            Err(error) => failures.push(error.to_string()),
        }
    }
    (invoked, failures)
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_extension_sdk::{ContributionsBuilder, ExtensionMeta};
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

    #[specforge_test(
        behavior = "rollback_failed_migration",
        verify = "a migration whose graph changes structure is rolled back automatically"
    )]
    fn a_migration_that_changes_the_graph_is_rolled_back() {
        let dir = project();
        let file = dir.path().join("old.spec");

        // A hook that renames the entity: the graph loses `alpha`.
        let outcome = run_with_hooks(&request(dir.path()), None, &mut |_, _| {
            let migrated = std::fs::read_to_string(&file).unwrap();
            std::fs::write(&file, migrated.replace("alpha", "beta")).unwrap();
            (vec!["@acme/x:migrate".into()], Vec::new())
        });

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
        assert!(outcome.failed() && !outcome.migrated());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), OLD);
    }

    #[specforge_test(
        behavior = "rollback_failed_migration",
        verify = "a migration whose extension hook fails is rolled back automatically"
    )]
    fn a_migration_whose_hook_fails_is_rolled_back() {
        let dir = project();

        let outcome = run_with_hooks(&request(dir.path()), None, &mut |_, _| {
            (
                Vec::new(),
                vec!["migration hook 'm' of @acme/x trapped".into()],
            )
        });

        assert!(!outcome.validated, "no validation after a failed hook");
        assert_eq!(outcome.hook_failures.len(), 1);
        assert_eq!(outcome.rollback.as_ref().unwrap().restored_count, 1);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("old.spec")).unwrap(),
            OLD
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
        let dir = project();
        std::fs::write(dir.path().join("old.spec"), "behavior alpha \"A\" {\n}\n").unwrap();
        let mut called = false;

        let outcome = run_with_hooks(&request(dir.path()), None, &mut |_, _| {
            called = true;
            (Vec::new(), Vec::new())
        });

        assert!(!outcome.pending && !outcome.applied && !outcome.validated);
        assert!(!called, "no hooks with nothing to migrate");
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

    /// `@acme/x`, served in process, declaring its migration hook
    /// `migrate_acme` with its handler, or no hook; the runtime records
    /// every export the host calls.
    fn hooked_extension(hook: bool) -> InProcessRuntime {
        let build = move || {
            let mut c = ContributionsBuilder::new(ExtensionMeta::new("@acme/x", "1.0.0"));
            if hook {
                c.migration_hook_handler("migrate_acme", |_| Ok(()));
            }
            c
        };
        InProcessRuntime::new().with(build)
    }

    fn exports_called(runtime: &InProcessRuntime) -> Vec<String> {
        runtime.calls().into_iter().map(|c| c.export).collect()
    }

    fn project_with_extension() -> tempfile::TempDir {
        let dir = project();
        std::fs::write(
            dir.path().join("specforge.json"),
            r#"{"name": "p", "version": "0.1.0", "extensions": ["@acme/x"]}"#,
        )
        .unwrap();
        dir
    }

    #[specforge_test(
        behavior = "invoke_extension_migration_hooks",
        verify = "extension with migration_hook field has it invoked during migrate"
    )]
    fn the_hook_an_extension_declares_in_its_handshake_runs_on_migrate() {
        let dir = project_with_extension();
        let runtime = hooked_extension(true);

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
        let runtime = hooked_extension(false);

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
}
