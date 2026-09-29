//! Shared analysis passes over a compiled project.
//!
//! One implementation serves every surface: `specforge analyze` (CLI) and
//! the `specforge.analyze` MCP tool run the same passes over the same
//! `AnalysisContext`. Findings are standard host diagnostics with `A`-codes.

use serde::Deserialize;
use specforge_common::Diagnostic;
use specforge_graph::Graph;
use specforge_parser::FieldValue;
use specforge_registry::{FieldRegistry, KindRegistry, ManifestFieldType};
use std::collections::HashMap;
use std::path::Path;

/// Everything a pass may inspect. Built once per analysis invocation.
pub struct AnalysisContext<'a> {
    pub graph: &'a Graph,
    pub kind_registry: &'a KindRegistry,
    pub field_registry: &'a FieldRegistry,
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

/// Result of one analysis pass.
pub struct PassReport {
    pub name: &'static str,
    pub description: &'static str,
    pub findings: Vec<Finding>,
    pub summary: serde_json::Value,
}

/// Built-in passes. Coverage is owned by `@specforge/testing`
/// (`@specforge/testing:coverage`, ADR 0002).
pub const PASS_NAMES: &[&str] = &["contracts"];

/// The extension pass that owns proof coverage (and the `--min` gate).
pub const COVERAGE_PASS: &str = "@specforge/testing:coverage";

// ── Layer 3: proof (specforge-report.json, RES-15) ─────────────────────────

#[derive(Debug, Deserialize, serde::Serialize)]
pub struct TestReport {
    #[serde(default)]
    pub runner: Option<String>,
    #[serde(default)]
    pub results: std::collections::BTreeMap<String, ReportedEntity>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
pub struct ReportedEntity {
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub tests: Vec<ReportedTest>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
pub struct ReportedTest {
    #[serde(default)]
    pub name: Option<String>,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<f64>,
    /// The `verify` obligation the test proves, when it says so.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verify: Option<String>,
    /// The collector that recorded the test.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner: Option<String>,
}

// ── contracts ───────────────────────────────────────────────────────────────

/// Reference fields count as contract obligations when they target the
/// formal contract kinds (invariants and properties) — e.g. the formal
/// extension's requires/ensures/maintains/satisfies.
const CONTRACT_TARGET_KINDS: &[&str] = &["invariant", "property"];

/// `contracts` — requires/ensures/maintains contract coverage (RES-25).
///
/// A contract-bearing kind (any kind with registered reference fields such as
/// `requires`/`ensures`/`maintains`) whose entities declare none of them is
/// reported as unconstrained. Reference existence is already checked by the
/// compiler (E003) and is not repeated here.
pub fn pass_contracts(ctx: &AnalysisContext) -> (Vec<Finding>, serde_json::Value) {
    let mut contract_fields: HashMap<&str, Vec<&str>> = HashMap::new();
    for (kind, field, entry) in ctx.field_registry.iter() {
        if !matches!(
            entry.field_type,
            ManifestFieldType::Reference | ManifestFieldType::ReferenceList
        ) {
            continue;
        }
        if entry
            .target_kind
            .as_deref()
            .is_some_and(|t| CONTRACT_TARGET_KINDS.contains(&t))
        {
            contract_fields.entry(kind).or_default().push(field);
        }
    }
    for fields in contract_fields.values_mut() {
        // Suggestions render this list — keep it registry-order-independent
        // (hardening-plan D4 / R-6).
        fields.sort_unstable();
    }

    let mut findings = Vec::new();
    let mut contract_entities = 0usize;
    let mut unconstrained = 0usize;
    let mut obligation_refs = 0usize;

    // Sorted node order: findings must not depend on HashMap seeding (R-6).
    let mut nodes: Vec<_> = ctx.graph.nodes();
    nodes.sort_by_key(|n| n.id.raw);
    for node in nodes {
        let Some(fields) = contract_fields.get(node.kind.raw.as_str()) else {
            continue;
        };
        let declared = fields
            .iter()
            .filter(|f| {
                matches!(
                    node.fields.get(f),
                    Some(FieldValue::ReferenceList(items)) if !items.is_empty()
                )
            })
            .count();
        if declared > 0 {
            contract_entities += 1;
            obligation_refs += declared;
        } else {
            unconstrained += 1;
            findings.push(
                Diagnostic::info(
                    "A010",
                    format!(
                        "{} '{}' declares no contract obligations",
                        node.kind.raw, node.id.raw
                    ),
                )
                .with_span(node.source_span.clone())
                .with_suggestion(format!(
                    "add requires/ensures/maintains references (registered for '{}': {})",
                    node.kind.raw,
                    fields.join(", ")
                )),
            );
        }
    }

    let summary = serde_json::json!({
        "contract_entities": contract_entities,
        "unconstrained_entities": unconstrained,
        "obligation_references": obligation_refs,
    });
    (findings, summary)
}

/// Run one named built-in pass. Returns `None` for unknown pass names.
pub fn run_pass(ctx: &AnalysisContext, pass: &str) -> Option<PassReport> {
    let (name, description, findings, summary) = match pass {
        "contracts" => {
            let (findings, summary) = pass_contracts(ctx);
            (
                "contracts",
                "entities of contract-bearing kinds without requires/ensures/maintains obligations",
                findings,
                summary,
            )
        }
        _ => return None,
    };
    Some(PassReport {
        name,
        description,
        findings,
        summary,
    })
}

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

/// Dispatch extension-declared compiler passes through the wasm runtime.
///
/// Each extension's describe payload lists `CompilerPassDescriptor`s; the
/// pass implementation lives in a `__pass_<name>` export that receives an
/// entity snapshot and returns host Diagnostics. Traps (e.g. an extension
/// that declares a pass but never implemented the export) are surfaced as
/// warnings rather than run failures.
pub fn run_extension_passes(
    manifests: &[specforge_registry::ManifestV2],
    input: &AnalysisContext,
    runtime: &dyn specforge_wasm::runtime::WasmRuntime,
    requested: &str,
) -> Vec<ExtensionPassReport> {
    use specforge_wasm::protocol::ProtocolHost;
    use specforge_wasm::runtime::WasmCallResult;

    let ctx_graph = input.graph;
    if manifests.is_empty() {
        return Vec::new();
    }
    // Only the "all" sweep and exact `<extension>:<pass>` selections run
    // extension passes.
    let wants = |name: &str| requested == "all" || requested == name;

    let host = ProtocolHost::new(runtime);
    let raw_entities = crate::compile::build_validation_entities(ctx_graph);
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
    let payload = serde_json::json!({
        "entities": entities,
        "edges": edges,
        "test_results": input.test_results,
        "proved_claims": proved_claims,
    });
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
        let passes: Vec<specforge_protocol_types::CompilerPassDescriptor> =
            match serde_json::from_value(response.items) {
                Ok(p) => p,
                Err(_) => continue,
            };
        for pass in order_passes(&passes) {
            let report_name = format!("{}:{}", manifest.name, pass.name);
            if !wants(&report_name) {
                continue;
            }
            let export = format!("__pass_{}", pass.name);
            match runtime.call_export(&manifest.name, &export, &payload_bytes) {
                WasmCallResult::Ok(bytes) => {
                    match parse_pass_output(&bytes) {
                        Ok((mut findings, pass_summary)) => {
                            // Canonical order for ALL extension passes
                            // (hardening-plan D4 / R-6): guests that iterate
                            // HashMaps would otherwise leak per-run order.
                            findings.sort_by(|a, b| {
                                a.code
                                    .cmp(&b.code)
                                    .then_with(|| {
                                        a.span
                                            .as_ref()
                                            .map(|s| (s.file.as_str(), s.start_line))
                                            .cmp(
                                                &b.span
                                                    .as_ref()
                                                    .map(|s| (s.file.as_str(), s.start_line)),
                                            )
                                    })
                                    .then_with(|| a.message.cmp(&b.message))
                            });
                            let mut summary = serde_json::json!({
                                "extension": manifest.name,
                                "pass": pass.name,
                                "entities_analyzed": entities.len(),
                            });
                            if let (Some(base), Some(extra)) =
                                (summary.as_object_mut(), pass_summary)
                            {
                                base.extend(extra);
                            }
                            reports.push(ExtensionPassReport {
                                name: report_name,
                                findings,
                                summary,
                            })
                        }
                        Err(e) => eprintln!(
                            "warning: extension pass '{report_name}' returned malformed diagnostics: {e}"
                        ),
                    }
                }
                WasmCallResult::Trap(trap) => {
                    eprintln!(
                        "warning: extension pass '{report_name}' did not execute: {}: {}",
                        trap.kind, trap.message
                    );
                }
            }
        }
    }
    reports
}

/// Keys an extension pass adds to its report summary.
type PassSummary = serde_json::Map<String, serde_json::Value>;

/// A pass returns either bare diagnostics or `{ diagnostics, summary }`
/// (the SDK's `PassOutput`); the summary's keys join the host's report summary.
fn parse_pass_output(
    bytes: &[u8],
) -> Result<(Vec<Diagnostic>, Option<PassSummary>), serde_json::Error> {
    #[derive(Deserialize)]
    struct WithSummary {
        diagnostics: Vec<Diagnostic>,
        #[serde(default)]
        summary: PassSummary,
    }
    match serde_json::from_slice::<Vec<Diagnostic>>(bytes) {
        Ok(findings) => Ok((findings, None)),
        Err(_) => serde_json::from_slice::<WithSummary>(bytes)
            .map(|out| (out.diagnostics, Some(out.summary))),
    }
}
