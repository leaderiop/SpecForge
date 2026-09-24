//! `specforge analyze` — static analysis passes over the compiled graph.
//!
//! Pass bodies live in [`specforge_emitter::analyze`] so the CLI, the MCP
//! tool, and any future surface run identical code. This module is the CLI
//! orchestration: pass selection, extension-owned pass dispatch (constraint
//! ordering + wasm calls), strictness, and output.

use std::path::Path;

use specforge_common::{Diagnostic, Severity};
use specforge_emitter::truncate_diagnostics;
use specforge_protocol_types::CompilerPassDescriptor;
use specforge_validator::{diagnostic_summary_detailed, render_diagnostics};
use specforge_wasm::WasmRuntime;
use specforge_wasm::protocol::ProtocolHost;

use crate::check::build_source_map;
use crate::pipeline;

const PASSES: &[&str] = &["all", "coverage", "contracts"];

/// Result of one analysis pass (built-in or extension-owned).
struct Report {
    name: String,
    description: String,
    findings: Vec<Diagnostic>,
    summary: serde_json::Value,
}

/// Order an extension's passes by their declared constraints: `after` /
/// `before` names become edges, and ties resolve by declaration order
/// (stable Kahn). Constraints referencing unknown passes — host phases like
/// "resolve", or other extensions' passes — are ignored; a constraint cycle
/// falls back to declaration order with a warning.
fn order_passes(passes: &[CompilerPassDescriptor]) -> Vec<CompilerPassDescriptor> {
    use std::collections::{HashMap, VecDeque};

    let index: HashMap<&str, usize> = passes
        .iter()
        .enumerate()
        .map(|(i, p)| (p.name.as_str(), i))
        .collect();
    let mut successors: Vec<Vec<usize>> = vec![Vec::new(); passes.len()];
    let mut indegree = vec![0usize; passes.len()];
    let mut cyclic_constraint = false;

    for (i, pass) in passes.iter().enumerate() {
        // (dependency name, dependency_runs_first): `after: X` means X runs
        // first; `before: X` means this pass runs first.
        let mut deps: Vec<(&str, bool)> = Vec::new();
        if let Some(after) = &pass.after {
            deps.push((after, true));
        }
        if let Some(before) = &pass.before {
            deps.push((before, false));
        }
        for (dep, dep_first) in deps {
            let Some(&dep_idx) = index.get(dep) else {
                continue; // unknown name: host phase or cross-extension
            };
            if dep == pass.name.as_str() {
                continue; // self-referential constraint: ignore
            }
            let (from, to) = if dep_first {
                (dep_idx, i)
            } else {
                (i, dep_idx)
            };
            if successors[from].contains(&to) {
                continue;
            }
            successors[from].push(to);
            indegree[to] += 1;
        }
    }

    let mut ready: VecDeque<usize> = (0..passes.len()).filter(|&i| indegree[i] == 0).collect();
    let mut order = Vec::with_capacity(passes.len());
    while let Some(i) = ready.pop_front() {
        order.push(i);
        for &to in &successors[i] {
            indegree[to] -= 1;
            if indegree[to] == 0 {
                ready.push_back(to);
            }
        }
    }
    if order.len() != passes.len() {
        cyclic_constraint = true;
    }

    let mut result: Vec<CompilerPassDescriptor> =
        order.into_iter().map(|i| passes[i].clone()).collect();
    if cyclic_constraint {
        eprintln!(
            "warning: extension pass constraints form a cycle; falling back to declaration order"
        );
        result = passes.to_vec();
    }
    result
}

/// Dispatch extension-declared compiler passes through the wasm runtime.
///
/// Each extension's describe payload lists `CompilerPassDescriptor`s; the
/// pass implementation lives in a `__pass_<name>` export that receives a
/// ValidationEntity snapshot and returns host Diagnostics. Traps (e.g. an
/// extension that declares a pass but never implemented the export) are
/// surfaced as warnings on the report rather than run failures.
fn run_extension_passes(
    manifests: &[specforge_registry::ManifestV2],
    input: &specforge_emitter::analyze::AnalysisContext,
    project_root: &Path,
    requested: &str,
) -> Vec<Report> {
    let ctx_graph = input.graph;
    if manifests.is_empty() {
        return Vec::new();
    }
    // Only the "all" sweep and exact `<extension>:<pass>` selections run
    // extension passes.
    let wants = |name: &str| requested == "all" || requested == name;

    let runtime = crate::pipeline::build_runtime(project_root);
    let host = ProtocolHost::new(&runtime);
    let raw_entities = specforge_emitter::compile::build_validation_entities(ctx_graph);
    let entities: Vec<serde_json::Value> = raw_entities
        .iter()
        .map(|e| {
            let testable = input
                .kind_registry
                .get(e.kind.as_str())
                .is_some_and(|entry| entry.supports_verify);
            serde_json::json!({
                "id": e.id,
                "kind": e.kind,
                "fields": e.fields,
                "incoming_edge_count": e.incoming_edge_count,
                "outgoing_edge_count": e.outgoing_edge_count,
                "span": e.span,
                "testable": testable,
            })
        })
        .collect();
    let edges: Vec<serde_json::Value> = ctx_graph
        .edges()
        .iter()
        .map(|e| {
            serde_json::json!({
                "source": e.source.as_str(),
                "target": e.target.as_str(),
                "label": e.label.as_str(),
            })
        })
        .collect();
    let payload = serde_json::json!({ "entities": entities, "edges": edges });
    let payload_bytes = match serde_json::to_vec(&payload) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("warning: cannot serialize entities for extension passes: {e}");
            return Vec::new();
        }
    };

    let mut reports = Vec::new();
    for manifest in manifests {
        let Ok(response) = host.describe(&manifest.name, "passes") else {
            continue;
        };
        let passes: Vec<CompilerPassDescriptor> = match serde_json::from_value(response.items) {
            Ok(p) => p,
            Err(_) => continue,
        };
        for pass in order_passes(&passes) {
            let report_name = format!("{}:{}", manifest.name, pass.name);
            if !wants(&report_name) {
                continue;
            }
            let export = format!("__pass_{}", pass.name);
            use specforge_wasm::runtime::WasmCallResult;
            match runtime.call_export(&manifest.name, &export, &payload_bytes) {
                WasmCallResult::Ok(bytes) => {
                    match serde_json::from_slice::<Vec<Diagnostic>>(&bytes) {
                        Ok(findings) => reports.push(Report {
                            name: report_name,
                            description: "extension compiler pass".to_string(),
                            findings,
                            summary: serde_json::json!({
                                "extension": manifest.name,
                                "pass": pass.name,
                                "entities_analyzed": entities.len(),
                            }),
                        }),
                        Err(e) => eprintln!(
                            "warning: extension pass '{}' returned malformed diagnostics: {e}",
                            report_name
                        ),
                    }
                }
                WasmCallResult::Trap(trap) => {
                    eprintln!(
                        "warning: extension pass '{}' did not execute: {}: {}",
                        report_name, trap.kind, trap.message
                    );
                }
            }
        }
    }
    reports
}

pub fn run(
    path: &Path,
    pass: Option<String>,
    json: bool,
    strict: bool,
    test_results: Option<&Path>,
) -> i32 {
    let ctx = pipeline::compile(path);

    let parsed_report = test_results.map(|report_path| {
        let raw = std::fs::read_to_string(report_path).unwrap_or_else(|e| {
            eprintln!("error: cannot read test results {}: {}", report_path.display(), e);
            std::process::exit(2);
        });
        serde_json::from_str::<specforge_emitter::analyze::TestReport>(&raw).unwrap_or_else(|e| {
            eprintln!(
                "error: invalid test results {}: {} (expected the RES-15 specforge-report.json shape)",
                report_path.display(),
                e
            );
            std::process::exit(2);
        })
    });

    let input = specforge_emitter::analyze::AnalysisContext {
        graph: &ctx.graph,
        kind_registry: &ctx.kind_registry,
        field_registry: &ctx.field_registry,
        project_root: Some(path),
        test_results: parsed_report.as_ref(),
    };
    let requested = pass.unwrap_or_else(|| "all".to_string());
    let mut selected: Vec<&str> = Vec::new();
    if requested == "all" {
        selected.extend(specforge_emitter::analyze::PASS_NAMES);
    } else if specforge_emitter::analyze::PASS_NAMES.contains(&requested.as_str()) {
        selected.push(requested.as_str());
    }
    if selected.is_empty() {
        eprintln!(
            "error: unknown analysis pass '{requested}' (available: {})",
            PASSES.join(", ")
        );
        return 2;
    }

    let sources = build_source_map(&ctx.spec_root, &ctx.resolved.files);
    let mut reports: Vec<Report> = Vec::new();
    let mut has_errors = false;
    for name in selected {
        let Some(report) = specforge_emitter::analyze::run_pass(&input, name) else {
            continue;
        };
        let (mut findings, summary) = (report.findings, report.summary);
        if strict {
            for d in &mut findings {
                if d.severity == Severity::Warning {
                    d.severity = Severity::Error;
                }
            }
        }
        if findings.iter().any(|d| d.severity == Severity::Error) {
            has_errors = true;
        }
        reports.push(Report {
            name: report.name.to_string(),
            description: report.description.to_string(),
            findings,
            summary,
        });
    }

    // Extension-owned passes run after the built-ins (see RES-25 ordering:
    // declared `after` constraints are advisory in this first version).
    reports.extend(run_extension_passes(
        &ctx.manifests,
        &input,
        path,
        &requested,
    ));

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
        for report in &reports {
            println!("analyze/{} — {}", report.name, report.description);
            if report.findings.is_empty() {
                println!("  no findings");
            } else {
                let mut rendered_findings = report.findings.clone();
                truncate_diagnostics(&mut rendered_findings);
                let rendered = render_diagnostics(&rendered_findings, &sources);
                if !rendered.is_empty() {
                    println!("{rendered}");
                } else {
                    for d in &rendered_findings {
                        println!("  [{d}]");
                    }
                }
            }
            println!("  summary: {}", report.summary);
            println!();
        }
        println!(
            "{}",
            diagnostic_summary_detailed(
                &reports
                    .iter()
                    .flat_map(|r| r.findings.iter().cloned())
                    .collect::<Vec<_>>(),
            )
        );
    }

    if has_errors { 1 } else { 0 }
}

#[cfg(test)]
mod order_tests {
    use super::*;
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
