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
use specforge_registry::ManifestV2;
use specforge_wasm::WasmRuntime;
use std::path::Path;

use crate::OpError;

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

/// Migrate the project, running its extensions (and their migration hooks)
/// in `runtime`.
pub fn run(request: &Request, runtime: Option<&dyn WasmRuntime>) -> Outcome {
    run_with_hooks(request, runtime, &mut |manifests| match runtime {
        Some(runtime) => invoke_hooks(manifests, runtime),
        None => (Vec::new(), Vec::new()),
    })
}

/// [`run`] with the hook step supplied: given the loaded manifests, it
/// returns the hooks it ran and why any failed.
pub fn run_with_hooks(
    request: &Request,
    runtime: Option<&dyn WasmRuntime>,
    hooks: &mut dyn FnMut(&[ManifestV2]) -> HookRun,
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
    };
    if !pending || request.dry_run {
        return outcome;
    }

    // The graph and schema before any file is touched.
    let pre = CompiledProject::compile(root, runtime);
    let pre_schema = schema_of(&pre);

    outcome.summary = migrate_project(root, target, false, request.no_backup);
    outcome.applied = true;
    if outcome.summary.failed_count > 0 {
        return outcome;
    }

    // Extension hooks run on the migrated files, before validation.
    let (invoked, failures) = hooks(&pre.env.registries.manifests);
    outcome.hooks_invoked = invoked;
    outcome.hook_failures = failures;
    if !outcome.hook_failures.is_empty() {
        outcome.rollback = Some(run_rollback(root));
        return outcome;
    }

    // The migrated project must have the graph it had before.
    let post = CompiledProject::compile(root, runtime);
    outcome.validated = true;
    outcome.schema_warnings = check_schema_compatibility(&pre_schema, &schema_of(&post));
    outcome.structural_differences = compare_graphs(&pre.graph, &post.graph);
    outcome.post_diagnostics = post.diagnostics();
    if !outcome.structural_differences.is_empty() {
        outcome.rollback = Some(run_rollback(root));
    }
    outcome
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
        &registries.extension_info,
    )
}

/// Run each extension's declared migration hook, in dependency order. A
/// hook that traps is recorded and the rest still run. Returns the hooks
/// run (`extension:hook`) and the failures.
pub fn invoke_hooks(manifests: &[ManifestV2], runtime: &dyn WasmRuntime) -> HookRun {
    use specforge_wasm::runtime::WasmCallResult;

    let order = match specforge_wasm::topological_sort_extensions(manifests) {
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
        let Some(manifest) = manifests.iter().find(|m| &m.name == name) else {
            continue;
        };
        let Some(hook) = manifest.migration_hook.as_deref().filter(|h| !h.is_empty()) else {
            continue;
        };
        match runtime.call_export(name, hook, b"{}") {
            WasmCallResult::Ok(_) => invoked.push(format!("{name}:{hook}")),
            WasmCallResult::Trap(trap) => failures.push(format!(
                "migration hook '{hook}' of {name} did not execute: {}: {}",
                trap.kind, trap.message
            )),
        }
    }
    (invoked, failures)
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_test_macros::test as specforge_test;
    use specforge_wasm::runtime::{WasmCallResult, WasmTrapInfo};
    use std::sync::Mutex;

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
        let outcome = run_with_hooks(&request(dir.path()), None, &mut |_| {
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

        let outcome = run_with_hooks(&request(dir.path()), None, &mut |_| {
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

        let outcome = run_with_hooks(&request(dir.path()), None, &mut |_| {
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

    /// An extension served over the protocol whose handshake names its
    /// migration hook; it records every export the host calls.
    struct HookedExtension {
        hook: Option<&'static str>,
        calls: Mutex<Vec<String>>,
    }

    impl WasmRuntime for HookedExtension {
        fn load_module(&self, _: &Path) -> Result<(), String> {
            Ok(())
        }

        fn call_export(&self, extension: &str, export: &str, input: &[u8]) -> WasmCallResult {
            self.calls.lock().unwrap().push(export.to_string());
            let reply = match export {
                "__handshake" => {
                    let mut handshake = serde_json::json!({
                        "protocol_version": "1.0.0",
                        "name": extension,
                        "version": "1.0.0",
                        "contribution_flags": {},
                        "peer_dependencies": [],
                        "sandbox_policy": null
                    });
                    if let Some(hook) = self.hook {
                        handshake["migration_hook"] = hook.into();
                    }
                    handshake
                }
                "__describe" => {
                    let request: serde_json::Value = serde_json::from_slice(input).unwrap();
                    serde_json::json!({ "category": request["category"], "items": [] })
                }
                _ => serde_json::json!({}),
            };
            WasmCallResult::Ok(reply.to_string().into_bytes())
        }
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
        let runtime = HookedExtension {
            hook: Some("migrate_acme"),
            calls: Mutex::new(Vec::new()),
        };

        let outcome = run(&request(dir.path()), Some(&runtime));

        assert!(outcome.migrated(), "{outcome:?}");
        assert_eq!(outcome.hooks_invoked, ["@acme/x:migrate_acme"]);
        assert!(outcome.hook_failures.is_empty(), "{outcome:?}");
        assert!(
            runtime
                .calls
                .lock()
                .unwrap()
                .contains(&"migrate_acme".to_string())
        );
    }

    #[specforge_test(
        behavior = "invoke_extension_migration_hooks",
        verify = "extension without migration_hook field is skipped silently"
    )]
    fn an_extension_whose_handshake_names_no_hook_is_skipped_silently() {
        let dir = project_with_extension();
        let runtime = HookedExtension {
            hook: None,
            calls: Mutex::new(Vec::new()),
        };

        let outcome = run(&request(dir.path()), Some(&runtime));

        assert!(outcome.migrated(), "{outcome:?}");
        assert!(outcome.hooks_invoked.is_empty() && outcome.hook_failures.is_empty());
        assert!(
            runtime
                .calls
                .lock()
                .unwrap()
                .iter()
                .all(|c| c.starts_with("__")),
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

    /// Two extensions with hooks; the first one's traps.
    struct Hooks {
        calls: Mutex<Vec<String>>,
    }

    impl WasmRuntime for Hooks {
        fn load_module(&self, _: &Path) -> Result<(), String> {
            Ok(())
        }

        fn call_export(&self, extension: &str, export: &str, _: &[u8]) -> WasmCallResult {
            self.calls
                .lock()
                .unwrap()
                .push(format!("{extension}:{export}"));
            if extension == "@acme/a" {
                return WasmCallResult::Trap(WasmTrapInfo {
                    kind: "unreachable".into(),
                    message: "hook panicked".into(),
                    export_name: export.into(),
                });
            }
            WasmCallResult::Ok(Vec::new())
        }
    }

    fn manifest(name: &str, hook: &str) -> ManifestV2 {
        serde_json::from_value(serde_json::json!({
            "name": name,
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "",
            "migrationHook": hook,
        }))
        .unwrap()
    }

    #[specforge_test(
        behavior = "invoke_extension_migration_hooks",
        verify = "hook that traps collects WasmTrapInfo and continues"
    )]
    fn a_trapping_hook_is_recorded_and_the_next_one_still_runs() {
        let runtime = Hooks {
            calls: Mutex::new(Vec::new()),
        };
        let manifests = [
            manifest("@acme/a", "migrate_a"),
            manifest("@acme/b", "migrate_b"),
        ];

        let (invoked, failures) = invoke_hooks(&manifests, &runtime);

        assert_eq!(invoked, ["@acme/b:migrate_b"]);
        assert_eq!(failures.len(), 1, "{failures:?}");
        assert!(
            failures[0].contains("unreachable") && failures[0].contains("hook panicked"),
            "{failures:?}"
        );
        assert_eq!(
            *runtime.calls.lock().unwrap(),
            ["@acme/a:migrate_a", "@acme/b:migrate_b"]
        );
    }
}
