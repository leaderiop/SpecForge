//! `specforge analyze` — static analysis passes over the compiled graph.
//!
//! The analysis itself is [`specforge_ops::analyze`], shared with the MCP
//! tool. This module is the CLI half: arguments in, rendering and exit
//! codes out.

use std::path::Path;

use specforge_common::{codes, diagnostic_summary, render_diagnostics, truncate_diagnostics};
use specforge_ops::OpError;
use specforge_ops::analyze::{AnalyzeOptions, Gate, ProveOptions, ReportSource, analyze};
use specforge_ops::view::ProjectView;

use crate::OutputFormat;
use crate::outcome::{Exit, Refusal};
use crate::pipeline;

pub fn run(
    path: &Path,
    pass: Option<String>,
    json: bool,
    strict: bool,
    test_results: Option<&Path>,
    min: Option<f64>,
    prove: bool,
) -> Exit {
    let project = pipeline::compile_project(path);
    // Without --test-results, the operation uses what `specforge collect`
    // last recorded at the root it compiled.
    let report = match test_results {
        Some(report_path) => ReportSource::File(report_path.to_path_buf()),
        None => ReportSource::Recorded,
    };
    let options = AnalyzeOptions {
        // Any name: the passes are the project's, so the operation refuses
        // one it does not run (exit 2, as clap refuses a flag's value).
        pass: pass.unwrap_or_else(|| specforge_ops::analyze::EVERY_PASS.to_string()),
        strict,
        report,
        min,
        prove: prove.then(ProveOptions::default),
    };
    // A refusal is the error document under `--json`, the lines on stderr
    // otherwise; analyze measures, so it cannot judge the project either way.
    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Human
    };
    let view = ProjectView::of(&project);
    let outcome = match analyze(&view, view.runtime().map(|r| r.as_ref()), &options) {
        Ok(outcome) => outcome,
        Err(error) => return Refusal::measuring(format).report(&OpError::from(error)),
    };
    // D3: stray test records, on stderr before the reports. Exact
    // matching is preserved; the warning only surfaces what was silently
    // dropped before.
    for stray in &outcome.stray_records {
        match &stray.near {
            Some(near) => eprintln!(
                "{}: test record references unknown entity '{}' (did you mean '{near}'?)",
                codes::W097,
                stray.entity_id
            ),
            None => eprintln!(
                "{}: test record references unknown entity '{}'",
                codes::W097,
                stray.entity_id
            ),
        }
    }
    let reports = &outcome.passes;

    if json {
        let doc = outcome.to_json();
        println!("{}", serde_json::to_string_pretty(&doc).unwrap_or_default());
    } else {
        // Human output is capped at the codebase-wide diagnostic limit so a
        // noisy first run stays readable; JSON output is never truncated.
        let color = crate::color::stdout();
        // The text each file was compiled from, to quote in snippets.
        let sources = project.source_texts();
        for report in reports {
            println!("analyze/{} — {}", report.name, report.description);
            if report.findings.is_empty() {
                println!("  no findings");
            } else {
                let mut rendered_findings = report.findings.clone();
                truncate_diagnostics(&mut rendered_findings);
                let rendered = render_diagnostics(&rendered_findings, &sources, color);
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
            diagnostic_summary(
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
                "error[{}]: --min requires the coverage pass (pass=coverage or all) from @specforge/testing — enable it with `specforge add @specforge/testing`",
                codes::E068
            );
            return Exit::Unjudged;
        }
        Gate::UnreadableSummary(e) => {
            eprintln!(
                "error: the coverage pass summary is not the shape this specforge reads ({e}); update @specforge/testing"
            );
            return Exit::Unjudged;
        }
        Gate::Below {
            pct,
            min,
            proven,
            total,
        } => {
            eprintln!(
                "error[{}]: proof coverage {pct:.1}% is below the required minimum {min:.1}% ({proven}/{total} testable entities proven)",
                codes::E048
            );
            return Exit::Failed;
        }
    }

    Exit::of_verdict(outcome.ok)
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
