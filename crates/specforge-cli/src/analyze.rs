//! `specforge analyze` — static analysis passes over the compiled graph.
//!
//! Each pass inspects the compiled project (graph + extension registries)
//! and reports findings as standard diagnostics with `A`-codes. Passes are
//! pure functions of the compilation context, so the same shape can later
//! host extension-owned passes dispatched through the wasm protocol.

use std::collections::HashMap;
use std::path::Path;

use specforge_common::{Diagnostic, Severity};
use specforge_emitter::compile::CompilationContext;
use specforge_parser::FieldValue;
use specforge_registry::ManifestFieldType;
use specforge_validator::{diagnostic_summary_detailed, render_diagnostics};

use crate::check::build_source_map;
use crate::pipeline;

/// One analysis pass over the compiled project.
trait AnalyzePass {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn run(&self, ctx: &CompilationContext) -> (Vec<Diagnostic>, serde_json::Value);
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

/// `coverage` — proof-obligation inventory (RES-25 coverage model).
///
/// Every `verify` statement is a proof obligation. Reports entities of
/// testable kinds and invariants that declare none, plus a risk-weighted
/// invariant table.
struct CoveragePass;

impl AnalyzePass for CoveragePass {
    fn name(&self) -> &'static str {
        "coverage"
    }

    fn description(&self) -> &'static str {
        "proof obligations per entity; unverified testable kinds and invariants"
    }

    fn run(&self, ctx: &CompilationContext) -> (Vec<Diagnostic>, serde_json::Value) {
        let testable: HashMap<&str, bool> = ctx
            .kind_registry
            .iter()
            .map(|(k, e)| (k.as_str(), e.supports_verify))
            .collect();

        let mut findings = Vec::new();
        let mut obligation_kinds: HashMap<String, usize> = HashMap::new();
        let mut testable_total = 0usize;
        let mut testable_verified = 0usize;
        // risk -> (total invariants, invariants without any obligation)
        let mut invariants: HashMap<String, (usize, usize)> = HashMap::new();

        for node in ctx.graph.nodes() {
            let stmts = verify_statements(node);
            for stmt in stmts {
                *obligation_kinds.entry(stmt.kind.clone()).or_default() += 1;
            }

            let kind = node.kind.raw.as_str();
            if testable.get(kind).copied().unwrap_or(false) {
                testable_total += 1;
                if stmts.is_empty() {
                    findings.push(
                        Diagnostic::warning(
                            "A001",
                            format!(
                                "{} '{}' declares no verify obligations",
                                node.kind.raw, node.id.raw
                            ),
                        )
                        .with_span(node.source_span.clone())
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
                if stmts.is_empty() {
                    entry.1 += 1;
                    findings.push(
                        Diagnostic::warning(
                            "A002",
                            format!("invariant '{}' declares no verify obligations", node.id.raw),
                        )
                        .with_span(node.source_span.clone())
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
        }

        let obligations: usize = obligation_kinds.values().sum();
        let summary = serde_json::json!({
            "testable_total": testable_total,
            "testable_verified": testable_verified,
            "obligations": obligations,
            "obligation_kinds": obligation_kinds,
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

    fn run(&self, ctx: &CompilationContext) -> (Vec<Diagnostic>, serde_json::Value) {
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

pub fn run(path: &Path, pass: Option<String>, json: bool, strict: bool) -> i32 {
    let ctx = pipeline::compile(path);

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
        let (mut findings, summary) = pass.run(&ctx);
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
