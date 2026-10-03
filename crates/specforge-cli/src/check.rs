use specforge_project::{CompiledProject, DiagnosticPolicy};
use specforge_validator::{diagnostic_summary_detailed, render_diagnostics_colored};
use specforge_wasm::WasmRuntime;
use std::collections::HashMap;
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

/// `specforge check` with the project's extensions running in `runtime`.
/// With `cache`, a check that passes records the build's lifecycle states
/// (the fields kinds declare as `lifecycle_field`) in `specforge-cache.json`.
fn run_in(
    path: &Path,
    runtime: &dyn WasmRuntime,
    strict: bool,
    format: OutputFormat,
    lint_profiles: &[String],
    cache: bool,
) -> i32 {
    let ctx = CompiledProject::compile(path, Some(runtime)).into_context();

    // --lint profiles add their diagnostics, --strict promotes warnings.
    let policy = DiagnosticPolicy {
        strict,
        lint_profiles: lint_profiles.to_vec(),
    };
    let all_diagnostics = policy.apply(path, ctx.diagnostics);

    // Output
    match format {
        OutputFormat::Json => {
            let entries = specforge_common::diagnostics_json(&all_diagnostics);
            let json = serde_json::to_string_pretty(&entries).unwrap_or_default();
            println!("{}", json);
        }
        OutputFormat::Human => {
            let color = crate::color::stderr();
            if !all_diagnostics.is_empty() {
                let sources = build_source_map(&ctx.spec_root, &ctx.resolved.files);
                let rendered = render_diagnostics_colored(&all_diagnostics, &sources, color);
                eprint!("{}", rendered);
            }
            eprintln!("{}", diagnostic_summary_detailed(&all_diagnostics, color));
        }
    }

    if cache {
        match specforge_project::record_build_cache(
            path,
            &ctx.graph,
            &ctx.kind_registry,
            &all_diagnostics,
        ) {
            Ok(true) => {}
            Ok(false) => eprintln!(
                "note: {} not written: the check failed",
                specforge_project::BUILD_CACHE_FILE
            ),
            Err(e) => {
                eprintln!(
                    "error: cannot write {}: {e}",
                    specforge_project::BUILD_CACHE_FILE
                );
                return 1;
            }
        }
    }

    // Strict already promoted warnings: errors alone decide.
    specforge_common::compute_exit_code(&all_diagnostics)
}

pub(crate) fn build_source_map(
    spec_root: &Path,
    files: &[specforge_resolver::ResolvedFile],
) -> HashMap<String, String> {
    let mut sources = HashMap::new();
    for file in files {
        let full_path = spec_root.join(&file.path);
        if let Ok(content) = std::fs::read_to_string(&full_path) {
            sources.insert(file.path.clone(), content);
        }
    }
    sources
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
