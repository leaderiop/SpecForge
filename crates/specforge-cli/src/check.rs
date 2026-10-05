use specforge_common::{Diagnostic, Severity};
use specforge_ops::check::{CacheRecord, CheckOptions, check};
use specforge_ops::view::ProjectView;
use specforge_project::{CompiledProject, LintProfile};
use specforge_validator::{diagnostic_summary_detailed, render_diagnostics_colored};
use specforge_wasm::WasmRuntime;
use std::path::Path;

use crate::OutputFormat;

pub fn run(
    path: &Path,
    strict: bool,
    format: OutputFormat,
    lint_profiles: &[LintProfile],
    severity: Option<Severity>,
    cache: bool,
) -> i32 {
    let runtime = specforge_component::project_runtime(path);
    let options = CheckOptions {
        strict,
        lint_profiles: lint_profiles.to_vec(),
        severity,
        record_cache: cache,
    };
    run_in(path, &runtime, format, &options)
}

/// `specforge check` with the project's extensions running in `runtime`:
/// the check operation over what the compile reported, the diagnostics the
/// severity filter shows rendered, and the verdict over everything as the
/// exit code. With `record_cache`, a check that passes records the build's
/// lifecycle states (the fields kinds declare as `lifecycle_field`) in
/// `specforge-cache.json`.
fn run_in(
    path: &Path,
    runtime: &dyn WasmRuntime,
    format: OutputFormat,
    options: &CheckOptions,
) -> i32 {
    if options.lint_profiles.contains(&LintProfile::Pedantic) {
        eprintln!("note: --lint pedantic is the default: info diagnostics are always reported");
    }
    let compiled = CompiledProject::compile(path, Some(runtime));
    let outcome = match check(&ProjectView::of(&compiled), compiled.diagnostics(), options) {
        Ok(outcome) => outcome,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    let shown: Vec<Diagnostic> = outcome.shown().into_iter().cloned().collect();

    match format {
        OutputFormat::Json => {
            let entries = specforge_common::diagnostics_json(&shown);
            let json = serde_json::to_string_pretty(&entries).unwrap_or_default();
            println!("{}", json);
        }
        OutputFormat::Human => {
            let color = crate::color::stderr();
            if !shown.is_empty() {
                let sources = compiled.resolved.source_texts();
                let rendered = render_diagnostics_colored(&shown, &sources, color);
                eprint!("{}", rendered);
            }
            // The summary counts everything reported, as the verdict does.
            let summary = diagnostic_summary_detailed(&outcome.reported, color);
            eprintln!("{}", with_filter_note(&summary, outcome.severity));
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

/// `summary` with `(showing <severity> only)` after its first line when a
/// severity filter hides the rest.
fn with_filter_note(summary: &str, severity: Option<Severity>) -> String {
    let Some(severity) = severity else {
        return summary.to_string();
    };
    let (first, rest) = match summary.split_once('\n') {
        Some((first, rest)) => (first, Some(rest)),
        None => (summary, None),
    };
    let mut out = format!("{first} (showing {severity} only)");
    if let Some(rest) = rest {
        out.push('\n');
        out.push_str(rest);
    }
    out
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
                OutputFormat::Human,
                &CheckOptions::default(),
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
