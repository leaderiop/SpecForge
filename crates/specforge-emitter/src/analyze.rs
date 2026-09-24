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
        } else if !stmts.is_empty() {
            // Obligations declared but no implementation connected
            // (RES-15: unlinked intent). Info until adoption matures.
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
