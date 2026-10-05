use specforge_ops::check::{CacheRecord, CheckOptions, check};
use specforge_ops::view::ProjectView;
use specforge_project::CompiledProject;
use specforge_validator::{diagnostic_summary_detailed, render_diagnostics_colored};
use specforge_wasm::WasmRuntime;
use std::path::Path;

use crate::OutputFormat;

pub fn run(
    path: &Path,
    strict: bool,
    format: OutputFormat,
    lint_profiles: &[String],
    cache: bool,
) -> i32 {
    let runtime = specforge_component::project_runtime(path);
    run_in(path, &runtime, strict, format, lint_profiles, cache)
}

/// `specforge check` with the project's extensions running in `runtime`:
/// the check operation over what the compile reported, rendered, and its
/// verdict as the exit code. With `cache`, a check that passes records the
/// build's lifecycle states (the fields kinds declare as `lifecycle_field`)
/// in `specforge-cache.json`.
fn run_in(
    path: &Path,
    runtime: &dyn WasmRuntime,
    strict: bool,
    format: OutputFormat,
    lint_profiles: &[String],
    cache: bool,
) -> i32 {
    let compiled = CompiledProject::compile(path, Some(runtime));
    let options = CheckOptions {
        strict,
        lint_profiles: lint_profiles
            .iter()
            .filter_map(|p| p.parse().ok())
            .collect(),
        severity: None,
        record_cache: cache,
    };
    let outcome = match check(
        &ProjectView::of(&compiled),
        compiled.diagnostics(),
        &options,
    ) {
        Ok(outcome) => outcome,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    match format {
        OutputFormat::Json => {
            let entries = specforge_common::diagnostics_json(&outcome.reported);
            let json = serde_json::to_string_pretty(&entries).unwrap_or_default();
            println!("{}", json);
        }
        OutputFormat::Human => {
            let color = crate::color::stderr();
            if !outcome.reported.is_empty() {
                let sources = compiled.resolved.source_texts();
                let rendered = render_diagnostics_colored(&outcome.reported, &sources, color);
                eprint!("{}", rendered);
            }
            eprintln!("{}", diagnostic_summary_detailed(&outcome.reported, color));
        }
    }

    match &outcome.cache {
        CacheRecord::NotRequested | CacheRecord::Written => {}
        CacheRecord::NotWritten => eprintln!(
            "note: {} not written: the check failed",
            specforge_project::BUILD_CACHE_FILE
        ),
        CacheRecord::WriteFailed(e) => {
            eprintln!(
                "error: cannot write {}: {e}",
                specforge_project::BUILD_CACHE_FILE
            );
            return 1;
        }
    }

    // Strict already promoted warnings: errors alone decide.
    if outcome.ok() { 0 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use specforge_test_macros::test as specforge_test;
    use specforge_wasm::{WasmCallResult, WasmTrapInfo};

    const EXT: &str = "@test/audit";

    /// An extension, in process, that declares the `gadget` kind and one
    /// check-phase pass, `audit`, which fails (E951) every gadget whose id
    /// starts with `bad`.
    struct AuditExtension;

    impl WasmRuntime for AuditExtension {
        fn load_module(&self, _: &Path) -> Result<(), String> {
            Ok(())
        }

        fn call_export(&self, extension: &str, export: &str, input: &[u8]) -> WasmCallResult {
            let ok = |value: Value| WasmCallResult::Ok(value.to_string().into_bytes());
            if extension != EXT {
                return WasmCallResult::Trap(WasmTrapInfo {
                    kind: "extension_not_found".into(),
                    message: extension.into(),
                    export_name: export.into(),
                });
            }
            let input: Value = serde_json::from_slice(input).unwrap_or(Value::Null);
            match export {
                "__handshake" => ok(json!({
                    "protocol_version": "1.0.0", "name": EXT, "version": "1.0.0",
                    "contribution_flags": { "entities": true },
                    "peer_dependencies": [], "sandbox_policy": null
                })),
                "__describe" => {
                    let category = input["category"].as_str().unwrap_or_default();
                    let items = match category {
                        "entities" => json!([{ "name": "gadget", "keyword": "gadget" }]),
                        "passes" => json!([{ "name": "audit", "phase": "check" }]),
                        _ => json!([]),
                    };
                    ok(json!({ "category": category, "items": items }))
                }
                "__pass_audit" => ok(Value::Array(
                    input["entities"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|e| e["id"].as_str())
                        .filter(|id| id.starts_with("bad"))
                        .map(|id| {
                            json!({
                                "code": "E951", "severity": "Error",
                                "message": format!("gadget '{id}' fails the audit"),
                                "entity": id
                            })
                        })
                        .collect(),
                )),
                other => WasmCallResult::Trap(WasmTrapInfo {
                    kind: "export_not_found".into(),
                    message: other.into(),
                    export_name: other.into(),
                }),
            }
        }
    }

    fn project(spec: &str) -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        let config = json!({ "name": "p", "version": "0.1.0", "extensions": [EXT] });
        std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
        std::fs::write(dir.path().join("a.spec"), spec).unwrap();
        dir
    }

    #[specforge_test(
        behavior = "run_check_phase_passes",
        verify = "a check pass's diagnostics are reported by specforge check"
    )]
    fn check_reports_a_check_pass_and_exits_on_its_errors() {
        let clean = project("gadget good \"Good\" {\n}\n");
        let failing = project("gadget good \"Good\" {\n}\n\ngadget bad_one \"Bad\" {\n}\n");

        let human = |dir: &tempfile::TempDir| {
            run_in(
                dir.path(),
                &AuditExtension,
                false,
                OutputFormat::Human,
                &[],
                false,
            )
        };
        assert_eq!(human(&clean), 0);
        assert_eq!(human(&failing), 1, "the pass's error fails the check");

        // What check reports is the compile's diagnostics, the pass's
        // among them with its code and severity.
        let reported =
            CompiledProject::compile(failing.path(), Some(&AuditExtension)).diagnostics();
        let audit: Vec<_> = reported.iter().filter(|d| d.code == "E951").collect();
        assert_eq!(audit.len(), 1, "{reported:?}");
        assert_eq!(audit[0].severity, specforge_common::Severity::Error);
    }
}
