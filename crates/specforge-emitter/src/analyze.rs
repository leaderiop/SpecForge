//! Shared analysis passes over a compiled project.
//!
//! One implementation serves every surface: `specforge analyze` (CLI) and
//! the `specforge.analyze` MCP tool run the same passes over the same
//! `AnalysisContext`. Findings are standard host diagnostics with `A`-codes.

use serde::Deserialize;
use specforge_common::{Diagnostic, Severity};
use specforge_graph::Graph;
use specforge_parser::{FieldValue, VerifyStatement};
use specforge_registry::{FieldRegistry, KindRegistry, ManifestFieldType};
use std::collections::HashMap;
use std::path::Path;

/// Everything a pass may inspect. Built once per analysis invocation.
pub struct AnalysisContext<'a> {
    pub graph: &'a Graph,
    pub kind_registry: &'a KindRegistry,
    pub field_registry: &'a FieldRegistry,
    /// Project root as given to the tool; `tests [...]` paths resolve
    /// against this. Linkage existence checks are skipped when absent.
    pub project_root: Option<&'a Path>,
    /// Parsed `--test-results` report, when provided (RES-15 layer 3).
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

pub const PASS_NAMES: &[&str] = &["coverage", "contracts"];

// ── Layer 3: proof (specforge-report.json, RES-15) ─────────────────────────

#[derive(Debug, Deserialize)]
pub struct TestReport {
    #[serde(default)]
    pub runner: Option<String>,
    #[serde(default)]
    pub results: std::collections::BTreeMap<String, ReportedEntity>,
}

#[derive(Debug, Deserialize)]
pub struct ReportedEntity {
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub tests: Vec<ReportedTest>,
}

#[derive(Debug, Deserialize)]
pub struct ReportedTest {
    #[serde(default)]
    pub name: Option<String>,
    pub status: String,
    #[serde(default)]
    pub duration_ms: Option<f64>,
}

// ── helpers ─────────────────────────────────────────────────────────────────

fn verify_statements(node: &specforge_graph::Node) -> &[VerifyStatement] {
    match node.fields.get("verify") {
        Some(FieldValue::VerifyList(stmts)) => stmts,
        _ => &[],
    }
}

fn risk_of(node: &specforge_graph::Node) -> String {
    match node.fields.get("risk") {
        Some(FieldValue::Identifier(r)) => r.clone(),
        Some(FieldValue::String(r)) => r.clone(),
        _ => "unspecified".to_string(),
    }
}

/// The `tests [...]` field as raw path strings (RES-15 Layer 2 linkage).
fn test_links(node: &specforge_graph::Node) -> Vec<String> {
    match node.fields.get("tests") {
        Some(FieldValue::StringList(items)) => items.clone(),
        Some(FieldValue::ReferenceList(items)) => items.clone(),
        _ => Vec::new(),
    }
}

/// Strip the runner-specific suffixes RES-15 allows on test links:
/// `tests/x.go::TestCreateUser` and `tests/x.ts:45` both point at
/// `tests/x.go` / `tests/x.ts` on disk.
fn test_link_file_path(link: &str) -> String {
    if let Some((file, _name)) = link.split_once("::") {
        file.to_string()
    } else if let Some((file, line)) = link.rsplit_once(':')
        && file.contains('.')
        && !line.is_empty()
        && line.chars().all(|c| c.is_ascii_digit())
    {
        file.to_string()
    } else {
        link.to_string()
    }
}

// ── coverage ────────────────────────────────────────────────────────────────

/// `coverage` — proof obligations + discharge tracking (RES-25 / RES-15).
///
/// Findings:
/// - A001: testable kind with no verify obligations (no intent)
/// - A002: invariant with no verify obligations (error when high-risk)
/// - A011: invariant that nothing references (orphan guarantee)
/// - A012: obligations declared but no `tests` linkage (unlinked intent, info)
/// - A013: `tests` linkage points at a file that does not exist
/// - A014: a linked test failed in the supplied test-results report
pub fn pass_coverage(ctx: &AnalysisContext) -> (Vec<Finding>, serde_json::Value) {
    let testable: HashMap<&str, bool> = ctx
        .kind_registry
        .iter()
        .map(|(k, e)| (k.as_str(), e.supports_verify))
        .collect();

    let mut findings = Vec::new();
    let mut invariant_orphans = 0usize;
    let mut obligation_kinds: HashMap<String, usize> = HashMap::new();
    let mut testable_total = 0usize;
    let mut testable_verified = 0usize;
    // risk -> (total invariants, invariants without any obligation)
    let mut invariants: HashMap<String, (usize, usize)> = HashMap::new();
    // Incoming edge count per invariant id: the enforcement mapping.
    let mut invariant_refs: HashMap<&str, usize> = HashMap::new();
    // Discharge funnel (RES-15 layers): intent -> linkage -> proof.
    let mut entities_with_obligations = 0usize;
    let mut entities_with_test_links = 0usize;
    let mut broken_test_links = 0usize;
    let mut entities_proven = 0usize;
    let mut report_failures = 0usize;
    let mut formally_discharged_entities = 0usize;

    for edge in ctx.graph.edges() {
        if let Some(node) = ctx.graph.node(edge.target.as_str())
            && node.kind.raw.as_str() == "invariant"
        {
            *invariant_refs.entry(edge.target.as_str()).or_default() += 1;
        }
    }

    for node in ctx.graph.nodes() {
        let stmts = verify_statements(node);
        for stmt in stmts {
            *obligation_kinds.entry(stmt.kind.clone()).or_default() += 1;
        }

        let kind = node.kind.raw.as_str();
        let span = node.source_span.clone();
        let id = node.id.raw.to_string();

        // Layer 2: linkage — `tests [...]` paths must exist on disk.
        let links = test_links(node);
        if !links.is_empty() {
            entities_with_test_links += 1;
            if let Some(root) = ctx.project_root {
                let missing: Vec<String> = links
                    .iter()
                    .filter(|link| !root.join(test_link_file_path(link)).exists())
                    .cloned()
                    .collect();
                if !missing.is_empty() {
                    broken_test_links += missing.len();
                    findings.push(
                        Diagnostic::warning(
                            "A013",
                            format!(
                                "{} '{}' links tests that do not exist: {}",
                                kind,
                                id,
                                missing.join(", ")
                            ),
                        )
                        .with_span(span.clone())
                        .with_suggestion(
                            "fix the paths in the tests field (they resolve from the project root)",
                        ),
                    );
                }
            }
        } else if !stmts.is_empty() && !formally_discharged(ctx, &id, stmts) {
            // Obligations declared but no implementation connected
            // (RES-15: unlinked intent). Info until adoption matures.
            // A proved formal claim discharges `verify property` duties,
            // so the linkage hint does not apply to it.
            findings.push(
                Diagnostic::info(
                    "A012",
                    format!(
                        "{} '{}' declares {} verify obligation(s) but no tests linkage",
                        kind,
                        id,
                        stmts.len()
                    ),
                )
                .with_span(span.clone())
                .with_suggestion("add a `tests [...]` field pointing at the executable tests"),
            );
        }

        // Layer 3: proof — compare against the test-results report.
        if let Some(report) = ctx.test_results
            && let Some(entity) = report.results.get(id.as_str())
            && !entity.tests.is_empty()
        {
            let failed: Vec<&str> = entity
                .tests
                .iter()
                .filter(|t| t.status != "pass")
                .map(|t| t.name.as_deref().unwrap_or("<unnamed>"))
                .collect();
            if failed.is_empty() {
                entities_proven += 1;
            } else {
                report_failures += failed.len();
                findings.push(
                    Diagnostic::error(
                        "A014",
                        format!(
                            "{} '{}' has {} failing test(s) in the test results: {}",
                            kind,
                            id,
                            failed.len(),
                            failed.join(", ")
                        ),
                    )
                    .with_span(span.clone()),
                );
            }
        }

        if formally_discharged(ctx, &id, stmts) {
            formally_discharged_entities += 1;
        }

        if testable.get(kind).copied().unwrap_or(false) {
            testable_total += 1;
            if stmts.is_empty() {
                findings.push(
                    Diagnostic::warning(
                        "A001",
                        format!("{} '{}' declares no verify obligations", kind, id),
                    )
                    .with_span(span.clone())
                    .with_suggestion("add a `verify unit` or `verify property` statement"),
                );
            } else {
                testable_verified += 1;
            }
        }

        if kind == "invariant" {
            let risk = risk_of(node);
            let entry = invariants.entry(risk.clone()).or_insert((0, 0));
            entry.0 += 1;
            if invariant_refs
                .get(node.id.raw.as_str())
                .copied()
                .unwrap_or(0)
                == 0
            {
                invariant_orphans += 1;
                findings.push(
                    Diagnostic::warning(
                        "A011",
                        format!(
                            "invariant '{}' is an orphan guarantee: nothing references it",
                            id
                        ),
                    )
                    .with_span(span.clone())
                    .with_suggestion(
                        "reference it from a behavior (invariants list, requires, ensures, or maintains) or drop the invariant",
                    ),
                );
            }
            if stmts.is_empty() {
                entry.1 += 1;
                findings.push(
                    Diagnostic::warning(
                        "A002",
                        format!("invariant '{}' declares no verify obligations", id),
                    )
                    .with_span(span.clone())
                    .with_suggestion(match risk.as_str() {
                        "high" => {
                            "high-risk invariant: add at least one `verify property` obligation"
                        }
                        _ => "add a `verify property` or `verify unit` obligation",
                    }),
                );
                if risk == "high"
                    && let Some(d) = findings.last_mut()
                {
                    d.severity = Severity::Error;
                }
            }
        }

        if !stmts.is_empty() {
            entities_with_obligations += 1;
        }
    }

    let obligations: usize = obligation_kinds.values().sum();
    let invariant_total: usize = invariants.values().map(|(t, _)| t).sum();
    let report_summary = match ctx.test_results {
        Some(report) => {
            let recorded: usize = report.results.values().map(|e| e.tests.len()).sum();
            let failed: usize = report
                .results
                .values()
                .flat_map(|e| e.tests.iter())
                .filter(|t| t.status != "pass")
                .count();
            serde_json::json!({
                "runner": report.runner,
                "entities_recorded": report.results.len(),
                "tests_recorded": recorded,
                "tests_failed": failed,
                "entities_proven": entities_proven,
            })
        }
        None => serde_json::Value::Null,
    };
    let summary = serde_json::json!({
        "testable_total": testable_total,
        "testable_verified": testable_verified,
        "obligations": obligations,
        "obligation_kinds": obligation_kinds,
        "invariant_enforced": invariant_total - invariant_orphans,
        "invariant_orphans": invariant_orphans,
        "discharge_funnel": {
            "entities_with_obligations": entities_with_obligations,
            "entities_with_test_links": entities_with_test_links,
            "broken_test_links": broken_test_links,
            "entities_proven": entities_proven,
            "report_failures": report_failures,
            "formally_discharged": formally_discharged_entities,
        },
        "test_results": report_summary,
        "invariants": invariants
            .iter()
            .map(|(risk, (total, unverified))| serde_json::json!({
                "risk": risk,
                "total": total,
                "unverified": unverified,
            }))
            .collect::<Vec<_>>(),
    });
    (findings, summary)
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

    let mut findings = Vec::new();
    let mut contract_entities = 0usize;
    let mut unconstrained = 0usize;
    let mut obligation_refs = 0usize;

    for node in ctx.graph.nodes() {
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

/// Run one named pass. Returns `None` for unknown pass names.
/// True when the entity declares `verify property` obligations and the
/// prove pass entailed its formal claim from the declared bounds.
///
/// `stmts` are the entity's verify statements.
fn formally_discharged(ctx: &AnalysisContext, id: &str, stmts: &[VerifyStatement]) -> bool {
    let Some(proved) = ctx.proved_claims else {
        return false;
    };
    stmts.iter().any(|s| s.kind == "property") && proved.contains(id)
}

pub fn run_pass(ctx: &AnalysisContext, pass: &str) -> Option<PassReport> {
    let (name, description, findings, summary) = match pass {
        "coverage" => {
            let (findings, summary) = pass_coverage(ctx);
            (
                "coverage",
                "proof obligations, discharge linkage, and enforcement per entity",
                findings,
                summary,
            )
        }
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
                    match serde_json::from_slice::<Vec<Diagnostic>>(&bytes) {
                        Ok(findings) => reports.push(ExtensionPassReport {
                            name: report_name,
                            findings,
                            summary: serde_json::json!({
                                "extension": manifest.name,
                                "pass": pass.name,
                                "entities_analyzed": entities.len(),
                            }),
                        }),
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
