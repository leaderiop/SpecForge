//! `specforge analyze` — static analysis passes over the compiled graph.
//!
//! The analysis itself is [`specforge_ops::analyze`], shared with the MCP
//! tool. This module is the CLI half: arguments in, rendering and exit
//! codes out.

use std::path::Path;

use specforge_emitter::truncate_diagnostics;
use specforge_ops::analyze::{
    AnalyzeOptions, Gate, ProjectView, ProveOptions, ReportSource, analyze,
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
        report,
        min,
        prove: prove.then(ProveOptions::default),
    };
    let outcome = match analyze(&ProjectView::of(&ctx, path), &runtime, &options) {
        Ok(outcome) => outcome,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    // D3: orphaned test records, on stderr before the reports. Exact
    // matching is preserved; the warning only surfaces what was silently
    // dropped before.
    for orphan in &outcome.orphans {
        match &orphan.near {
            Some(near) => eprintln!(
                "W097: test record references unknown entity '{}' (did you mean '{near}'?)",
                orphan.entity_id
            ),
            None => eprintln!(
                "W097: test record references unknown entity '{}'",
                orphan.entity_id
            ),
        }
    }
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
