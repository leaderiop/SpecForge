//! Extension compiler passes: what an extension declares (`__describe
//! passes`), in which order they run, the input every `__pass_<name>`
//! export receives and how its answer is read. Check-phase passes run with
//! every compile ([`crate::check_passes`]); the others under
//! `specforge analyze` (`specforge_ops::analyze`). Findings are standard host
//! diagnostics.

use serde::Deserialize;
use specforge_common::Diagnostic;
use specforge_graph::Graph;
use specforge_registry::{FieldRegistry, KindRegistry};
use std::path::Path;

use crate::coverage::TestReport;

/// Everything a pass may inspect. Built once per analysis invocation.
pub struct AnalysisContext<'a> {
    pub graph: &'a Graph,
    pub kind_registry: &'a KindRegistry,
    pub field_registry: &'a FieldRegistry,
    /// The extensions' validation rules: which kinds must declare
    /// obligations (W004), so the coverage pass knows who is exempt.
    pub rules: &'a [(
        specforge_registry::validation_engine::ValidationRulePattern,
        String,
    )],
    /// Project root as given to the tool, when known.
    pub project_root: Option<&'a Path>,
    /// Parsed `--test-results` report, when provided; forwarded to extension
    /// passes (the coverage pass scores proof from it).
    pub test_results: Option<&'a TestReport>,
    /// Entity ids whose formal claims the prove pass ENTAILED from the
    /// declared bounds (RES-25). `None` when the prove pass did not run;
    /// a proved claim discharges `verify property` obligations without
    /// executable tests.
    pub proved_claims: Option<&'a std::collections::HashSet<String>>,
}

/// A diagnostic returned by an analysis pass, ready for host rendering.
pub type Finding = Diagnostic;

// ── Extension-owned compiler passes (WASM-only migration, Phase 4) ─────────

/// Result of one extension-owned compiler pass.
pub struct ExtensionPassReport {
    /// `<extension>:<pass>` identifier.
    pub name: String,
    pub findings: Vec<Finding>,
    pub summary: serde_json::Value,
}

/// Order an extension's passes by their declared constraints: `after` /
/// `before` names become edges, and ties resolve by declaration order
/// (stable Kahn). Constraints referencing unknown passes — host phases like
/// "resolve", or other extensions' passes — are ignored; a constraint cycle
/// falls back to declaration order with a warning.
pub fn order_passes(
    passes: &[specforge_protocol_types::CompilerPassDescriptor],
) -> Vec<specforge_protocol_types::CompilerPassDescriptor> {
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

    let mut result: Vec<specforge_protocol_types::CompilerPassDescriptor> =
        order.into_iter().map(|i| passes[i].clone()).collect();
    if cyclic_constraint {
        eprintln!(
            "warning: extension pass constraints form a cycle; falling back to declaration order"
        );
        result = passes.to_vec();
    }
    result
}

/// The phase a pass declares to run with every compile instead of under
/// `specforge analyze`: the compiled project runs it after the graph
/// checks, and its diagnostics are the compile's.
pub const CHECK_PHASE: &str = "check";

/// Whether `pass` runs with every compile ([`CHECK_PHASE`]).
pub fn is_check_phase(pass: &specforge_protocol_types::CompilerPassDescriptor) -> bool {
    pass.phase.as_deref() == Some(CHECK_PHASE)
}

/// The compiler passes `extension` declares (`__describe passes`), in the
/// order they run ([`order_passes`]). Empty when it declares none, or its
/// answer does not parse.
pub fn declared_passes(
    runtime: &dyn specforge_wasm::runtime::WasmRuntime,
    extension: &str,
) -> Vec<specforge_protocol_types::CompilerPassDescriptor> {
    match specforge_wasm::protocol::load_declaration(runtime, extension) {
        Ok(loaded) => order_passes(&loaded.declaration.passes),
        Err(_) => Vec::new(),
    }
}

/// The input every `__pass_<name>` export receives (the SDK's
/// `PassInput`): the entity snapshot, the resolved edges, and the test
/// results and proved claims when the caller has them.
pub fn pass_input(input: &AnalysisContext) -> serde_json::Value {
    let ctx_graph = input.graph;
    // How the coverage rule sees each entity: `testable` is its kind's flag
    // (a kind that merely accepts `verify` statements, a formal `property`,
    // does not count), and `exempt` says it owes no obligations of its own
    // (ADR 0004, D2-b), decided here from the registries.
    let registries = crate::coverage::CoverageRegistries {
        kinds: input.kind_registry,
        fields: input.field_registry,
        rules: input.rules,
    };
    let entities: Vec<serde_json::Value> = registries
        .entities(ctx_graph)
        .iter()
        .map(|(e, rule)| {
            serde_json::json!({
                "id": e.id,
                "kind": e.kind,
                "fields": e.fields,
                "incoming_edge_count": e.incoming_edge_count,
                "outgoing_edge_count": e.outgoing_edge_count,
                "span": e.span,
                "testable": rule.testable,
                "exempt": rule.exempt,
                "verify_kinds": e.verify_kinds,
                "verify_texts": e.verify_texts,
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
    let proved_claims: Option<Vec<&String>> = input.proved_claims.map(|claims| {
        let mut ids: Vec<&String> = claims.iter().collect();
        ids.sort();
        ids
    });
    serde_json::json!({
        "entities": entities,
        "edges": edges,
        "test_results": input.test_results,
        "proved_claims": proved_claims,
    })
}

/// Call `extension`'s `__pass_<pass>` export with `input` (a serialized
/// [`pass_input`]) and read its diagnostics, in canonical order. A
/// diagnostic with no span that names an `entity` of `graph` gets that
/// entity's span. Err: the export trapped, or its answer does not parse.
pub fn call_pass(
    runtime: &dyn specforge_wasm::runtime::WasmRuntime,
    extension: &str,
    pass: &str,
    input: &[u8],
    graph: &Graph,
) -> Result<(Vec<Diagnostic>, Option<PassSummary>), String> {
    use specforge_wasm::runtime::WasmCallResult;

    let export = format!("__pass_{pass}");
    match runtime.call_export(extension, &export, input) {
        WasmCallResult::Ok(bytes) => {
            let (mut findings, summary) = parse_pass_output(&bytes, graph)
                .map_err(|e| format!("returned malformed diagnostics: {e}"))?;
            // Canonical order for ALL extension passes (hardening-plan D4 /
            // R-6): guests that iterate HashMaps would otherwise leak
            // per-run order.
            findings.sort_by(|a, b| {
                a.code
                    .cmp(&b.code)
                    .then_with(|| {
                        a.span
                            .as_ref()
                            .map(|s| (s.file.as_str(), s.start_line))
                            .cmp(&b.span.as_ref().map(|s| (s.file.as_str(), s.start_line)))
                    })
                    .then_with(|| a.message.cmp(&b.message))
            });
            Ok((findings, summary))
        }
        WasmCallResult::Trap(trap) => {
            Err(format!("did not execute: {}: {}", trap.kind, trap.message))
        }
    }
}

/// Dispatch extension-declared compiler passes through the wasm runtime.
///
/// Each extension's describe payload lists `CompilerPassDescriptor`s; the
/// pass implementation lives in a `__pass_<name>` export that receives an
/// entity snapshot and returns host Diagnostics. Traps (e.g. an extension
/// that declares a pass but never implemented the export) are surfaced as
/// warnings rather than run failures. Check-phase passes
/// ([`is_check_phase`]) are not analyze passes: they run with every
/// compile.
pub fn run_extension_passes(
    manifests: &[specforge_registry::ManifestV2],
    input: &AnalysisContext,
    runtime: &dyn specforge_wasm::runtime::WasmRuntime,
    requested: &str,
) -> Vec<ExtensionPassReport> {
    if manifests.is_empty() {
        return Vec::new();
    }
    // Only the "all" sweep and exact `<extension>:<pass>` selections run
    // extension passes.
    let wants = |name: &str| requested == "all" || requested == name;

    let payload = pass_input(input);
    let entities_analyzed = payload["entities"].as_array().map_or(0, Vec::len);
    let payload_bytes = match serde_json::to_vec(&payload) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("warning: cannot serialize entities for extension passes: {e}");
            return Vec::new();
        }
    };

    let mut reports = Vec::new();
    for manifest in manifests {
        for pass in declared_passes(runtime, &manifest.name) {
            if is_check_phase(&pass) {
                continue;
            }
            let report_name = format!("{}:{}", manifest.name, pass.name);
            if !wants(&report_name) {
                continue;
            }
            match call_pass(
                runtime,
                &manifest.name,
                &pass.name,
                &payload_bytes,
                input.graph,
            ) {
                Ok((findings, pass_summary)) => {
                    let mut summary = serde_json::json!({
                        "extension": manifest.name,
                        "pass": pass.name,
                        "entities_analyzed": entities_analyzed,
                    });
                    if let (Some(base), Some(extra)) = (summary.as_object_mut(), pass_summary) {
                        base.extend(extra);
                    }
                    reports.push(ExtensionPassReport {
                        name: report_name,
                        findings,
                        summary,
                    })
                }
                Err(e) => eprintln!("warning: extension pass '{report_name}' {e}"),
            }
        }
    }
    reports
}

/// Keys an extension pass adds to its report summary.
pub type PassSummary = serde_json::Map<String, serde_json::Value>;

/// A pass returns either bare diagnostics or `{ diagnostics, summary }`
/// (the SDK's `PassOutput`); the summary's keys join the host's report
/// summary. A diagnostic may name the entity it is about (`entity`): with
/// no span of its own, it gets that entity's.
fn parse_pass_output(
    bytes: &[u8],
    graph: &Graph,
) -> Result<(Vec<Diagnostic>, Option<PassSummary>), serde_json::Error> {
    #[derive(Deserialize)]
    struct PassDiagnostic {
        #[serde(flatten)]
        diagnostic: Diagnostic,
        #[serde(default)]
        entity: Option<String>,
    }
    #[derive(Deserialize)]
    struct WithSummary {
        diagnostics: Vec<PassDiagnostic>,
        #[serde(default)]
        summary: PassSummary,
    }
    let (diagnostics, summary) = match serde_json::from_slice::<Vec<PassDiagnostic>>(bytes) {
        Ok(diagnostics) => (diagnostics, None),
        Err(_) => serde_json::from_slice::<WithSummary>(bytes)
            .map(|out| (out.diagnostics, Some(out.summary)))?,
    };
    let diagnostics = diagnostics
        .into_iter()
        .map(|PassDiagnostic { diagnostic, entity }| {
            let span = diagnostic.span.clone().or_else(|| {
                entity
                    .as_deref()
                    .and_then(|id| graph.node(id))
                    .map(|node| node.source_span.clone())
            });
            Diagnostic { span, ..diagnostic }
        })
        .collect();
    Ok((diagnostics, summary))
}

#[cfg(test)]
mod order_tests {
    use super::order_passes;
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
