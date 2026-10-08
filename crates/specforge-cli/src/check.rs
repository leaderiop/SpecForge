use specforge_common::{Diagnostic, Severity, diagnostic_summary, render_diagnostics};
use specforge_ops::check::{CacheRecord, CheckOptions, check};
use specforge_ops::view::ProjectView;
use specforge_ops::{OpError, OpErrorKind};
use specforge_project::{CompiledProject, LintProfile};
use specforge_wasm::WasmRuntime;
use std::path::Path;

use crate::OutputFormat;
use crate::outcome::{Exit, Refusal};

pub fn run(
    path: &Path,
    strict: bool,
    format: OutputFormat,
    lint_profiles: &[LintProfile],
    severity: Option<Severity>,
    cache: bool,
) -> Exit {
    let runtime = specforge_component::ComponentRuntime::with_user_cache();
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
) -> Exit {
    if options.lint_profiles.contains(&LintProfile::Pedantic) {
        eprintln!("note: --lint pedantic is the default: info diagnostics are always reported");
    }
    let compiled = CompiledProject::compile(path, Some(runtime));
    let outcome = match check(&ProjectView::of(&compiled), compiled.diagnostics(), options) {
        Ok(outcome) => outcome,
        Err(error) => return Refusal::of(format).report(&error.into()),
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
                let sources = compiled.source_texts();
                let rendered = render_diagnostics(&shown, &sources, color);
                eprint!("{}", rendered);
            }
            // The summary counts everything reported, as the verdict does.
            let summary = diagnostic_summary(&outcome.reported, color);
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
            return Refusal::of(format).report(&OpError::new(
                OpErrorKind::Internal,
                "cache_write_failed",
                format!("cannot write {}: {e}", specforge_project::BUILD_CACHE_FILE),
            ));
        }
    }

    // Strict already promoted warnings: errors alone decide.
    Exit::of_verdict(outcome.ok())
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
    use serde_json::json;
    use specforge_extension_sdk::prelude::*;
    use specforge_test_macros::test as specforge_test;
    use specforge_wasm::testing::InProcessRuntime;

    const EXT: &str = "@test/audit";

    /// An extension, in process, that declares the `gadget` kind and one
    /// check-phase pass, `audit`, which fails (E951) every gadget whose id
    /// starts with `bad`.
    fn audit_extension() -> InProcessRuntime {
        InProcessRuntime::new().with(|| {
            let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
            c.kind("gadget", |k| {
                k.keyword("gadget");
            });
            c.pass("audit", |p| {
                p.phase("check").run(|input: &PassInput| {
                    input
                        .entities
                        .iter()
                        .filter(|e| e.id.starts_with("bad"))
                        .map(|e| {
                            PassDiagnostic::new(
                                "E951",
                                PassSeverity::Error,
                                format!("gadget '{}' fails the audit", e.id),
                            )
                            .with_entity(&e.id)
                        })
                        .collect::<Vec<_>>()
                });
            });
            c
        })
    }

    fn project(spec: &str) -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        let config = json!({ "name": "p", "version": "0.1.0", "extensions": [EXT] });
        std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
        specforge_installed::testing::install(dir.path(), &[EXT]);
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
                &audit_extension(),
                OutputFormat::Human,
                &CheckOptions::default(),
            )
        };
        assert_eq!(human(&clean), Exit::Passed);
        assert_eq!(
            human(&failing),
            Exit::Failed,
            "the pass's error fails the check"
        );

        // What check reports is the compile's diagnostics, the pass's
        // among them with its code and severity.
        let reported =
            CompiledProject::compile(failing.path(), Some(&audit_extension())).diagnostics();
        let audit: Vec<_> = reported.iter().filter(|d| d.code == "E951").collect();
        assert_eq!(audit.len(), 1, "{reported:?}");
        assert_eq!(audit[0].severity, specforge_common::Severity::Error);
    }
}
