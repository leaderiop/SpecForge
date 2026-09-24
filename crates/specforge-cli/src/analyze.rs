//! `specforge analyze` — static analysis passes over the compiled graph.
//!
//! Each pass inspects the compiled project (graph + extension registries)
//! and reports findings as standard diagnostics with `A`-codes. Passes are
//! pure functions of the compilation context, so the same shape can later
//! host extension-owned passes dispatched through the wasm protocol.
//!
//! The coverage pass implements the RES-15 three-layer traceability model:
//! intent (`verify` statements), linkage (`tests` fields pointing at
//! executable test files), and proof (test-runner results consumed from a
//! `specforge-report.json` via `--test-results`).

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use serde::Deserialize;
use specforge_common::{Diagnostic, Severity};
use specforge_emitter::compile::CompilationContext;
use specforge_parser::FieldValue;
use specforge_registry::ManifestFieldType;
use specforge_validator::{diagnostic_summary_detailed, render_diagnostics};

use crate::check::build_source_map;
use crate::pipeline;

/// Everything a pass may inspect. Built once per `analyze` invocation.
pub struct AnalyzeInput<'a> {
    ctx: &'a CompilationContext,
    /// Project root as given on the command line; `tests [...]` paths
    /// resolve against this (RES-15 paths are project-root-relative).
    project_root: &'a Path,
    /// Parsed `--test-results` report, when provided.
    test_results: Option<&'a TestReport>,
}

/// One analysis pass over the compiled project.
trait AnalyzePass {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn run(&self, input: &AnalyzeInput) -> (Vec<Diagnostic>, serde_json::Value);
}

/// Extract verify statements from a node's `verify` field, if any.
fn verify_statements(node: &specforge_graph::Node) -> &[specforge_parser::VerifyStatement] {
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
    } else if let Some((file, _line)) = link.rsplit_once(':')
        && file.contains('.')
        && !_line.is_empty()
        && _line.chars().all(|c| c.is_ascii_digit())
    {
        file.to_string()
    } else {
        link.to_string()
    }
}

// ── Layer 3: proof (specforge-report.json, RES-15) ─────────────────────────

#[derive(Debug, Deserialize)]
pub struct TestReport {
    #[allow(dead_code)]
    #[serde(default)]
    pub specforge: Option<String>,
    #[serde(default)]
    pub runner: Option<String>,
    #[serde(default)]
    pub results: BTreeMap<String, ReportedEntity>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // round-trip fields from the RES-15 report shape
pub struct ReportedEntity {
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub tests: Vec<ReportedTest>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // round-trip fields from the RES-15 report shape
pub struct ReportedTest {
    #[serde(default)]
    pub name: Option<String>,
    pub status: String,
    #[serde(default)]
    pub duration_ms: Option<f64>,
}

// ── passes ──────────────────────────────────────────────────────────────────

/// `coverage` — proof obligations + discharge tracking (RES-25 / RES-15).
///
/// Findings:
/// - A001: testable kind with no verify obligations (no intent)
/// - A002: invariant with no verify obligations (error when high-risk)
/// - A011: invariant that nothing references (orphan guarantee)
/// - A012: obligations declared but no `tests` linkage (unlinked intent, info)
/// - A013: `tests` linkage points at a file that does not exist
/// - A014: a linked test failed in the supplied test-results report
struct CoveragePass;

impl AnalyzePass for CoveragePass {
    fn name(&self) -> &'static str {
        "coverage"
    }

    fn description(&self) -> &'static str {
        "proof obligations, discharge linkage, and enforcement per entity"
    }

    fn run(&self, input: &AnalyzeInput) -> (Vec<Diagnostic>, serde_json::Value) {
        let ctx = input.ctx;
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
        // Anything referencing an invariant (behaviors' `invariants [...]`,
        // requires/ensures/maintains contract fields) creates an edge.
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
                let missing: Vec<String> = links
                    .iter()
                    .filter(|link| {
                        let rel = test_link_file_path(link);
                        !input.project_root.join(&rel).exists()
                    })
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
            if let Some(report) = input.test_results
                && let Some(entity) = report.results.get(id.as_str())
            {
                let failed: Vec<&str> = entity
                    .tests
                    .iter()
                    .filter(|t| t.status != "pass")
                    .map(|t| t.name.as_deref().unwrap_or("<unnamed>"))
                    .collect();
                if entity.tests.is_empty() {
                    // Present in the report but with no recorded tests.
                } else if failed.is_empty() {
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
                    continue;
                }
            }

            let stmts_len = stmts.len();
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

            if stmts_len > 0 {
                entities_with_obligations += 1;
            }
        }

        let obligations: usize = obligation_kinds.values().sum();
        let invariant_total: usize = invariants.values().map(|(t, _)| t).sum();
        let report_summary = match input.test_results {
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
            None => serde_json::json!(null),
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
}

/// `contracts` — requires/ensures/maintains contract coverage (RES-25).
///
/// A contract-bearing kind (any kind with registered reference fields such as
/// `requires`/`ensures`/`maintains`) whose entities declare none of them is
/// reported as unconstrained. Reference existence is already checked by the
/// compiler (E003) and is not repeated here.
struct ContractsPass;

/// Reference fields count as contract obligations when they target the
/// formal contract kinds (invariants and properties) — e.g. the formal
/// extension's requires/ensures/maintains/satisfies.
const CONTRACT_TARGET_KINDS: &[&str] = &["invariant", "property"];

impl ContractsPass {
    fn contract_fields(ctx: &CompilationContext) -> HashMap<&str, Vec<&str>> {
        let mut map: HashMap<&str, Vec<&str>> = HashMap::new();
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
                map.entry(kind).or_default().push(field);
            }
        }
        map
    }

    fn declared_contract_fields(node: &specforge_graph::Node, fields: &[&str]) -> usize {
        fields
            .iter()
            .filter(|f| {
                matches!(
                    node.fields.get(f),
                    Some(FieldValue::ReferenceList(items)) if !items.is_empty()
                )
            })
            .count()
    }
}

impl AnalyzePass for ContractsPass {
    fn name(&self) -> &'static str {
        "contracts"
    }

    fn description(&self) -> &'static str {
        "entities of contract-bearing kinds without requires/ensures/maintains obligations"
    }

    fn run(&self, input: &AnalyzeInput) -> (Vec<Diagnostic>, serde_json::Value) {
        let ctx = input.ctx;
        let contract_fields = Self::contract_fields(ctx);
        let mut findings = Vec::new();
        let mut contract_entities = 0usize;
        let mut unconstrained = 0usize;
        let mut obligation_refs = 0usize;

        for node in ctx.graph.nodes() {
            let Some(fields) = contract_fields.get(node.kind.raw.as_str()) else {
                continue;
            };
            let declared = Self::declared_contract_fields(node, fields);
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
}

const PASSES: &[&str] = &["all", "coverage", "contracts"];

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
        serde_json::from_str::<TestReport>(&raw).unwrap_or_else(|e| {
            eprintln!(
                "error: invalid test results {}: {} (expected the RES-15 specforge-report.json shape)",
                report_path.display(),
                e
            );
            std::process::exit(2);
        })
    });

    let input = AnalyzeInput {
        ctx: &ctx,
        project_root: path,
        test_results: parsed_report.as_ref(),
    };
    let coverage = CoveragePass;
    let contracts = ContractsPass;
    let requested = pass.unwrap_or_else(|| "all".to_string());
    let mut selected: Vec<&dyn AnalyzePass> = Vec::new();
    if requested == "all" {
        selected.push(&coverage);
        selected.push(&contracts);
    } else if requested == coverage.name() {
        selected.push(&coverage);
    } else if requested == contracts.name() {
        selected.push(&contracts);
    }
    if selected.is_empty() {
        eprintln!(
            "error: unknown analysis pass '{requested}' (available: {})",
            PASSES.join(", ")
        );
        return 2;
    }

    let sources = build_source_map(&ctx.spec_root, &ctx.resolved.files);

    struct Report {
        name: &'static str,
        description: &'static str,
        findings: Vec<Diagnostic>,
        summary: serde_json::Value,
    }

    let mut reports: Vec<Report> = Vec::new();
    let mut has_errors = false;
    for pass in &selected {
        let (mut findings, summary) = pass.run(&input);
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
            name: pass.name(),
            description: pass.description(),
            findings,
            summary,
        });
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
        for report in &reports {
            println!("analyze/{} — {}", report.name, report.description);
            if report.findings.is_empty() {
                println!("  no findings");
            } else {
                let rendered = render_diagnostics(&report.findings, &sources);
                if !rendered.is_empty() {
                    println!("{rendered}");
                } else {
                    for d in &report.findings {
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
