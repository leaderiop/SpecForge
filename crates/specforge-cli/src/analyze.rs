//! `specforge analyze` — static analysis passes over the compiled graph.
//!
//! Pass bodies live in [`specforge_emitter::analyze`] so the CLI, the MCP
//! tool, and any future surface run identical code, behind
//! [`specforge_ops::analyze`]. This module is the CLI half: arguments in,
//! rendering and exit codes out.

use std::path::Path;

use specforge_emitter::truncate_diagnostics;
use specforge_ops::analyze::{
    AnalyzeError, AnalyzeOptions, Gate, ProjectView, ProveOptions, ReportSource, analyze,
};
use specforge_validator::{diagnostic_summary_detailed, render_diagnostics_colored};

use crate::AnalysisPass;
use crate::check::build_source_map;
use crate::pipeline;

pub fn run(
    path: &Path,
    pass: Option<AnalysisPass>,
    json: bool,
    strict: bool,
    test_results: Option<&Path>,
    min: Option<f64>,
    prove: bool,
) -> i32 {
    let (ctx, runtime) = pipeline::compile_with_runtime(path);
    // Without --test-results, the operation uses what `specforge collect`
    // last recorded.
    let report = match test_results {
        Some(report_path) => ReportSource::File(report_path.to_path_buf()),
        None => ReportSource::Recorded,
    };
    let options = AnalyzeOptions {
        pass: pass.unwrap_or(AnalysisPass::All).name().to_string(),
        strict,
        report: report.clone(),
        min,
        prove: prove.then(ProveOptions::default),
    };
    let outcome = match analyze(&ProjectView::of(&ctx, path), &runtime, &options) {
        Ok(outcome) => outcome,
        Err(AnalyzeError::MinNeedsTestResults) => {
            eprintln!(
                "error: --min needs test results: run `specforge collect` or pass --test-results"
            );
            return 2;
        }
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    warn_orphans(&ctx, path, &report);
    let reports = &outcome.passes;
    let sources = build_source_map(&ctx.spec_root, &ctx.resolved.files);

    if json {
        let doc = outcome.to_json();
        println!("{}", serde_json::to_string_pretty(&doc).unwrap_or_default());
    } else {
        // Human output is capped at the codebase-wide diagnostic limit so a
        // noisy first run stays readable; JSON output is never truncated.
        let color = crate::color::stdout();
        for report in reports {
            println!("analyze/{} — {}", report.name, report.description);
            if report.findings.is_empty() {
                println!("  no findings");
            } else {
                let mut rendered_findings = report.findings.clone();
                truncate_diagnostics(&mut rendered_findings);
                let rendered = render_diagnostics_colored(&rendered_findings, &sources, color);
                if !rendered.is_empty() {
                    println!("{rendered}");
                } else {
                    for d in &rendered_findings {
                        println!("  [{d}]");
                    }
                }
            }
            let mut lines = Vec::new();
            summary_lines(&report.summary, "", &mut lines);
            if !lines.is_empty() {
                println!("  summary:");
                for line in lines {
                    println!("    {line}");
                }
            }
            println!();
        }
        println!(
            "{}",
            diagnostic_summary_detailed(
                &reports
                    .iter()
                    .flat_map(|r| r.findings.iter().cloned())
                    .collect::<Vec<_>>(),
                color,
            )
        );
    }

    // The gate comes after the reports print, so the operator still sees the
    // full analysis; it only decides the exit code.
    match &outcome.gate {
        Gate::NotRequested | Gate::Met => {}
        Gate::NoCoveragePass => {
            eprintln!(
                "error[E068]: --min requires the coverage pass (pass=coverage or all) from @specforge/testing — enable it with `specforge add @specforge/testing`"
            );
            return 2;
        }
        Gate::UnreadableSummary(e) => {
            eprintln!(
                "error: the coverage pass summary is not the shape this specforge reads ({e}); update @specforge/testing"
            );
            return 2;
        }
        Gate::Below {
            pct,
            min,
            proven,
            total,
        } => {
            eprintln!(
                "error[E048]: proof coverage {pct:.1}% is below the required minimum {min:.1}% ({proven}/{total} testable entities proven)"
            );
            return 1;
        }
    }

    if outcome.ok { 0 } else { 1 }
}

/// D3: orphaned test records, report entries for entities the graph does
/// not know, on stderr. Exact matching is preserved; the warning only
/// surfaces what was silently dropped before.
// TODO(#39): the operation returns these as data; this goes away then.
fn warn_orphans(
    ctx: &specforge_emitter::compile::CompilationContext,
    path: &Path,
    source: &ReportSource,
) {
    let report = match source {
        ReportSource::File(p) => specforge_emitter::coverage::read_report_file(p).ok(),
        _ => specforge_common::find_project_root(path).and_then(|root| {
            specforge_emitter::coverage::read_report(&root)
                .ok()
                .flatten()
        }),
    };
    let Some(report) = report else { return };
    for entity_id in report.results.keys() {
        if ctx.graph.node(entity_id).is_none() {
            let suggestion = specforge_common::suggest::find_close_match(
                entity_id,
                ctx.graph.nodes().iter().map(|n| n.id.raw.as_str()),
            );
            match suggestion {
                Some(near) => eprintln!(
                    "W097: test record references unknown entity '{entity_id}' (did you mean '{near}'?)"
                ),
                None => eprintln!("W097: test record references unknown entity '{entity_id}'"),
            }
        }
    }
}

/// A pass summary as `key: value` lines, nested keys dotted. Its shape is
/// the extension's, so nothing here knows what the keys mean; nulls (a
/// section with nothing to report) are left out.
fn summary_lines(value: &serde_json::Value, prefix: &str, out: &mut Vec<String>) {
    match value {
        serde_json::Value::Null => {}
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                let key = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                summary_lines(value, &key, out);
            }
        }
        serde_json::Value::String(s) => out.push(format!("{prefix}: {s}")),
        other => out.push(format!("{prefix}: {other}")),
    }
}

#[cfg(test)]
mod summary_tests {
    use super::summary_lines;

    #[test]
    fn nested_keys_are_dotted_and_nulls_left_out() {
        let mut lines = Vec::new();
        let summary = serde_json::json!({
            "pass": "coverage",
            "funnel": {"proven": 2, "failures": 0},
            "results": null,
            "kinds": ["a", "b"]
        });
        summary_lines(&summary, "", &mut lines);
        assert_eq!(
            lines,
            vec![
                "funnel.failures: 0",
                "funnel.proven: 2",
                "kinds: [\"a\",\"b\"]",
                "pass: coverage"
            ]
        );
    }
}
