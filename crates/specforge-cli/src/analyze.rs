//! `specforge analyze` — static analysis passes over the compiled graph.
//!
//! Pass bodies live in [`specforge_emitter::analyze`] so the CLI, the MCP
//! tool, and any future surface run identical code. This module is the CLI
//! orchestration: pass selection, extension-owned pass dispatch (constraint
//! ordering + wasm calls), strictness, and output.

use std::path::Path;

use specforge_common::{Diagnostic, Severity};
use specforge_emitter::truncate_diagnostics;
use specforge_validator::{diagnostic_summary_detailed, render_diagnostics_colored};

use crate::AnalysisPass;
use crate::check::build_source_map;
use crate::pipeline;

/// Result of one analysis pass (built-in or extension-owned).
struct Report {
    name: String,
    description: String,
    findings: Vec<Diagnostic>,
    summary: serde_json::Value,
}

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
    // Without --test-results, use what `specforge collect` last recorded.
    let read = match test_results {
        Some(report_path) => specforge_emitter::coverage::read_report_file(report_path).map(Some),
        None => specforge_common::find_project_root(path).map_or(Ok(None), |root| {
            specforge_emitter::coverage::read_report(&root)
        }),
    };
    let parsed_report = match read {
        Ok(report) => report,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    if min.is_some() && parsed_report.is_none() {
        eprintln!(
            "error: --min needs test results: run `specforge collect` or pass --test-results"
        );
        return 2;
    }

    // D3: orphaned test records — report entries for entities the graph
    // does not know. Exact matching is preserved; the warning only surfaces
    // what was silently dropped before.
    if let Some(report) = &parsed_report {
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

    let requested = match pass {
        // Coverage is an extension pass (ADR 0002).
        Some(AnalysisPass::Coverage) => specforge_emitter::analyze::COVERAGE_PASS,
        other => other.map_or("all", AnalysisPass::name),
    };
    let base_input = specforge_emitter::analyze::AnalysisContext {
        graph: &ctx.graph,
        kind_registry: &ctx.kind_registry,
        field_registry: &ctx.field_registry,
        project_root: Some(path),
        test_results: parsed_report.as_ref(),
        proved_claims: None,
    };

    // Run the SMT proof pass FIRST when requested: its entailment verdicts
    // feed the coverage discharge funnel (a proved formal claim discharges
    // `verify property` obligations without executable tests).
    let prove_report = if prove {
        Some(crate::prove::run_prove(&base_input))
    } else {
        None
    };
    let proved_claims: std::collections::HashSet<String> = prove_report
        .as_ref()
        .map(|r| r.proved_claim_ids.iter().cloned().collect())
        .unwrap_or_default();

    let input = specforge_emitter::analyze::AnalysisContext {
        proved_claims: if prove_report.is_some() {
            Some(&proved_claims)
        } else {
            None
        },
        ..base_input
    };
    let selected: &[&str] = match pass.unwrap_or(AnalysisPass::All) {
        AnalysisPass::All => specforge_emitter::analyze::PASS_NAMES,
        AnalysisPass::Coverage => &[],
        AnalysisPass::Contracts => &["contracts"],
    };

    let sources = build_source_map(&ctx.spec_root, &ctx.resolved.files);
    let mut reports: Vec<Report> = Vec::new();
    for name in selected {
        let Some(report) = specforge_emitter::analyze::run_pass(&input, name) else {
            continue;
        };
        reports.push(Report {
            name: report.name.to_string(),
            description: report.description.to_string(),
            findings: report.findings,
            summary: report.summary,
        });
    }

    // Extension-owned passes run after the built-ins (see RES-25 ordering:
    // declared `after` constraints are advisory in this first version).
    reports.extend(
        specforge_emitter::analyze::run_extension_passes(
            &ctx.manifests,
            &input,
            &runtime,
            requested,
        )
        .into_iter()
        .map(|r| Report {
            name: r.name,
            description: "extension compiler pass".to_string(),
            findings: r.findings,
            summary: r.summary,
        }),
    );
    // SMT proof pass report (computed before the built-ins so coverage can
    // thread formal discharge verdicts): verify numeric constraint bounds
    // with z3.
    if let Some(report) = prove_report {
        reports.push(Report {
            name: "prove".to_string(),
            description: "numeric constraint bounds verified with an SMT solver".to_string(),
            findings: report.findings,
            summary: report.summary,
        });
    }

    // Apply strictness and compute the error state uniformly across every
    // report source (built-in passes, extension passes, prove).
    let mut has_errors = false;
    for report in &mut reports {
        if strict {
            for d in &mut report.findings {
                if d.severity == Severity::Warning {
                    d.severity = Severity::Error;
                }
            }
        }
        if report
            .findings
            .iter()
            .any(|d| d.severity == Severity::Error)
        {
            has_errors = true;
        }
    }

    if json {
        let doc = serde_json::json!({
            "ok": !has_errors,
            "passes": reports
                .iter()
                .map(|r| serde_json::json!({
                    "pass": r.name,
                    "findings": r.findings,
                    "summary": r.summary,
                }))
                .collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&doc).unwrap_or_default());
    } else {
        // Human output is capped at the codebase-wide diagnostic limit so a
        // noisy first run stays readable; JSON output is never truncated.
        let color = crate::color::stdout();
        for report in &reports {
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

    // D2: coverage gate — after the reports print, so the operator still
    // sees the full analysis; the gate only decides the exit code.
    if let Some(min_pct) = min {
        let coverage_report = reports
            .iter()
            .find(|r| r.name == specforge_emitter::analyze::COVERAGE_PASS);
        let Some(coverage_report) = coverage_report else {
            eprintln!(
                "error[E068]: --min requires the coverage pass (pass=coverage or all) from @specforge/testing — enable it with `specforge add @specforge/testing`"
            );
            return 2;
        };
        let total = coverage_report.summary["testable_total"]
            .as_u64()
            .unwrap_or(0);
        let proven = coverage_report.summary["discharge_funnel"]["entities_proven"]
            .as_u64()
            .unwrap_or(0);
        let pct = if total == 0 {
            100.0 // nothing testable: the gate is vacuously satisfied
        } else {
            proven as f64 * 100.0 / total as f64
        };
        if pct + f64::EPSILON < min_pct {
            eprintln!(
                "error[E048]: proof coverage {pct:.1}% is below the required minimum {min_pct:.1}% ({proven}/{total} entities proven)"
            );
            return 1;
        }
    }

    if has_errors { 1 } else { 0 }
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

#[cfg(test)]
mod order_tests {
    use specforge_emitter::analyze::order_passes;
    use specforge_protocol_types::CompilerPassDescriptor;

    fn pass(name: &str, after: Option<&str>, before: Option<&str>) -> CompilerPassDescriptor {
        CompilerPassDescriptor {
            name: name.to_string(),
            after: after.map(str::to_string),
            before: before.map(str::to_string),
            phase: None,
        }
    }

    fn names(passes: &[CompilerPassDescriptor]) -> Vec<&str> {
        passes.iter().map(|p| p.name.as_str()).collect()
    }

    #[test]
    fn after_constraints_order_dependencies_first() {
        let passes = vec![
            pass("layering_verify", Some("condition_check"), None),
            pass("condition_check", Some("resolve"), None),
            pass("event_graph_analyze", Some("layering_verify"), None),
        ];
        assert_eq!(
            names(&order_passes(&passes)),
            vec!["condition_check", "layering_verify", "event_graph_analyze"]
        );
    }

    #[test]
    fn before_constraints_run_this_pass_first() {
        // `before: "first"` means this pass runs BEFORE "first".
        let passes = vec![
            pass("second", None, Some("first")),
            pass("first", None, None),
        ];
        assert_eq!(names(&order_passes(&passes)), vec!["second", "first"]);
    }

    #[test]
    fn ties_resolve_in_declaration_order() {
        let passes = vec![pass("b", None, None), pass("a", None, None)];
        assert_eq!(names(&order_passes(&passes)), vec!["b", "a"]);
    }

    #[test]
    fn unknown_constraint_names_are_ignored() {
        let passes = vec![pass("solo", Some("resolve"), None)];
        assert_eq!(names(&order_passes(&passes)), vec!["solo"]);
    }

    #[test]
    fn constraint_cycles_fall_back_to_declaration_order() {
        let passes = vec![pass("a", Some("b"), None), pass("b", Some("a"), None)];
        assert_eq!(names(&order_passes(&passes)), vec!["a", "b"]);
    }
}
