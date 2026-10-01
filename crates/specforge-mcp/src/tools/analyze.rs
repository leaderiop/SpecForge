use serde::Deserialize;
use std::path::PathBuf;

use crate::args::lenient;
use crate::state::McpState;
use crate::tool::ToolOutcome;
use specforge_emitter::analyze::{AnalysisContext, TestReport, run_pass};
use specforge_project::{CompiledProject, DiagnosticPolicy};

#[derive(Debug, Deserialize)]
pub struct Args {
    #[serde(default, deserialize_with = "lenient")]
    pass: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    strict: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    test_results: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    use_cached: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    path: Option<String>,
}

/// `specforge.analyze` — run the analysis passes (coverage, contracts) plus
/// extension-owned compiler passes over the project and return structured
/// findings. Extension passes execute through the same Wasm runtime the CLI
/// uses (WASM-only migration, Phase 4).
pub fn call(state: &mut McpState, args: Args) -> ToolOutcome {
    let path = args
        .path
        .map(PathBuf::from)
        .or_else(|| state.project_root.clone());

    let use_cached = args.use_cached.unwrap_or(false);

    // A path naming another project is analyzed for this call only: the
    // server keeps serving its own. Otherwise the served project is
    // recompiled unless the caller opted into the cached graph; a rootless
    // server analyzes its existing state.
    let other: Option<(PathBuf, CompiledProject)> = match &path {
        Some(root) if state.serves_other_than(root) => {
            Some((root.clone(), state.compile_project(root)))
        }
        Some(root) => {
            if !use_cached || state.graph.node_count() == 0 {
                state.recompile(root);
            }
            None
        }
        None => None,
    };
    let state: &McpState = state;
    let (graph, kind_registry, field_registry, rules, manifests, project_root) = match &other {
        Some((root, project)) => (
            &project.graph,
            &project.env.registries.kinds,
            &project.env.registries.fields,
            project.env.registries.rules.as_slice(),
            project.env.registries.manifests.as_slice(),
            Some(root.as_path()),
        ),
        None => (
            &state.graph,
            &state.kind_registry,
            &state.field_registry,
            state.rules.as_slice(),
            state.manifests.as_slice(),
            state.project_root.as_deref(),
        ),
    };

    // `strict` promotes warnings in every pass's findings, as the CLI does.
    let policy = DiagnosticPolicy::strict(args.strict.unwrap_or(false));

    // Without `test_results`, use what `specforge collect` last recorded, as
    // the CLI does; otherwise proof coverage would silently read nothing.
    // A report that can't be used is an error result, as the CLI exits 2.
    let read = match args.test_results.as_deref() {
        Some(named) => {
            specforge_emitter::coverage::read_report_file(&PathBuf::from(named)).map(Some)
        }
        None => match project_root {
            Some(root) => specforge_emitter::coverage::read_report(root),
            None => Ok(None),
        },
    };
    let parsed_report: Option<TestReport> = match read {
        Ok(report) => report,
        Err(e) => return super::coverage::report_error_result(&e, "specforge.analyze"),
    };

    // The tool doesn't run prove, so no claim is proved: `None`, as
    // `specforge analyze` without --prove passes (ADR 0004 D3-f). An empty
    // set would tell extension passes that prove ran and proved nothing.
    let context = AnalysisContext {
        proved_claims: None,
        graph,
        kind_registry,
        field_registry,
        rules,
        project_root,
        test_results: parsed_report.as_ref(),
    };

    let requested = args.pass.unwrap_or_else(|| "all".to_string());
    // Coverage is an extension pass owned by @specforge/testing (ADR 0002).
    let requested = if requested == "coverage" {
        specforge_emitter::analyze::COVERAGE_PASS.to_string()
    } else {
        requested
    };
    let pass_names: Vec<&str> = if requested == "all" {
        specforge_emitter::analyze::PASS_NAMES.to_vec()
    } else if specforge_emitter::analyze::PASS_NAMES.contains(&requested.as_str()) {
        vec![requested.as_str()]
    } else if requested.contains(':') {
        // `<extension>:<pass>` selects an extension pass only.
        Vec::new()
    } else {
        return ToolOutcome::invalid_params(format!(
            "Unknown analysis pass '{requested}' (available: all, coverage, {})",
            specforge_emitter::analyze::PASS_NAMES.join(", ")
        ));
    };

    let mut passes = Vec::new();
    let mut has_errors = false;
    for name in pass_names {
        let Some(mut report) = run_pass(&context, name) else {
            continue;
        };
        policy.promote(&mut report.findings);
        if report
            .findings
            .iter()
            .any(|d| d.severity == specforge_common::Severity::Error)
        {
            has_errors = true;
        }
        passes.push(serde_json::json!({
            "pass": report.name,
            "findings": report.findings,
            "summary": report.summary,
        }));
    }

    // Extension-owned passes run through the Wasm runtime, same as the CLI
    // (RES-25 ordering: declared `after` constraints are advisory here too).
    if !manifests.is_empty()
        && let Some(root) = project_root
    {
        let runtime = state.wasm_runtime(root);
        let extension_reports = specforge_emitter::analyze::run_extension_passes(
            manifests,
            &context,
            runtime.as_ref(),
            &requested,
        );
        for mut report in extension_reports {
            policy.promote(&mut report.findings);
            if report
                .findings
                .iter()
                .any(|d| d.severity == specforge_common::Severity::Error)
            {
                has_errors = true;
            }
            passes.push(serde_json::json!({
                "pass": report.name,
                "findings": report.findings,
                "summary": report.summary,
            }));
        }
    }

    let doc = serde_json::json!({ "ok": !has_errors, "passes": passes });
    ToolOutcome::ok(doc).flagged(has_errors)
}
