//! Extension compiler passes: the input every `__pass_<name>` export
//! receives and how its answer is read. What an extension declares, and in
//! which order its passes run, is the registry build's
//! (`specforge_registry::RegistryBuild::passes`). Check-phase passes run with
//! every compile ([`crate::check_passes`]); the others under
//! `specforge analyze` (`specforge_ops::analyze`). Findings are standard host
//! diagnostics.

use specforge_common::Diagnostic;
use specforge_graph::Graph;
use specforge_protocol_types::{
    PassEdge, PassEntity, PassInput, PassOutput, PassSpan, PassTestResults,
};
use specforge_registry::{FieldRegistry, KindRegistry};
use specforge_wasm::{CallError, ExtensionCalls, Operation};
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

/// The input every `__pass_<name>` export receives (the protocol's
/// `PassInput`): the entity snapshot, the resolved edges, and the test
/// results and proved claims when the caller has them. Each entity carries
/// how the coverage rule sees it: `testable` is its kind's flag (a kind
/// that merely accepts `verify` statements, a formal `property`, does not
/// count), and `exempt` says it owes no obligations of its own (ADR 0004,
/// D2-b), decided here from the registries.
pub fn pass_input(input: &AnalysisContext) -> PassInput {
    let registries = crate::coverage::CoverageRegistries {
        kinds: input.kind_registry,
        fields: input.field_registry,
        rules: input.rules,
    };
    let entities = registries
        .entities(input.graph)
        .into_iter()
        .map(|(e, rule)| PassEntity {
            id: e.id,
            kind: e.kind,
            fields: e.fields.into_iter().collect(),
            incoming_edge_count: e.incoming_edge_count,
            outgoing_edge_count: e.outgoing_edge_count,
            span: Some(PassSpan {
                file: e.span.file.as_str().to_string(),
                start_line: e.span.start_line,
                start_col: e.span.start_col,
                end_line: e.span.end_line,
                end_col: e.span.end_col,
            }),
            testable: rule.testable,
            exempt: rule.exempt,
            verify_kinds: e.verify_kinds,
            verify_texts: e.verify_texts,
        })
        .collect();
    let edges = input
        .graph
        .edges()
        .iter()
        .map(|e| PassEdge {
            source: e.source.as_str().to_string(),
            target: e.target.as_str().to_string(),
            label: e.label.as_str().to_string(),
        })
        .collect();
    let proved_claims = input.proved_claims.map(|claims| {
        let mut ids: Vec<String> = claims.iter().cloned().collect();
        ids.sort();
        ids
    });
    PassInput {
        entities,
        edges,
        test_results: input.test_results.map(PassTestResults::from),
        proved_claims,
        previous: None,
    }
}

/// The host diagnostics of a pass's answer: in canonical order, a
/// span-less one naming an entity of `graph` given that entity's span.
pub fn pass_findings(output: PassOutput, graph: &Graph) -> Vec<Diagnostic> {
    specforge_wasm::pass_diagnostics(output, |id| {
        graph.node(id).map(|node| node.source_span.clone())
    })
}

/// Run the extensions' analyze passes (`passes` as the registry build
/// ordered them; check-phase passes run with every compile instead)
/// through the wasm runtime: each `__pass_<name>` export receives the
/// entity snapshot and returns host diagnostics. A pass that fails (it
/// traps, or answers what does not parse) has a report of its own with one
/// E028 finding, so the analysis fails.
pub fn run_extension_passes(
    passes: &[specforge_registry::DeclaredPass],
    input: &AnalysisContext,
    runtime: &dyn specforge_wasm::runtime::WasmRuntime,
    requested: &str,
) -> Vec<ExtensionPassReport> {
    if passes.is_empty() {
        return Vec::new();
    }
    // Only the "all" sweep and exact `<extension>:<pass>` selections run
    // extension passes.
    let wants = |name: &str| requested == "all" || requested == name;

    let payload = pass_input(input);
    let entities_analyzed = payload.entities.len();
    let encoded = ExtensionCalls::encode(&payload);
    let calls = ExtensionCalls::new(runtime);

    let mut reports = Vec::new();
    for declared in passes.iter().filter(|p| !p.is_check_phase()) {
        let report_name = declared.full_name();
        if !wants(&report_name) {
            continue;
        }
        let pass = &declared.pass;
        let mut summary = serde_json::Map::new();
        summary.insert("extension".into(), declared.extension.clone().into());
        summary.insert("pass".into(), pass.name.clone().into());
        summary.insert("entities_analyzed".into(), entities_analyzed.into());
        let answer = match &encoded {
            Ok(encoded) => calls.run_pass(&declared.extension, &pass.name, encoded),
            Err(failure) => Err(CallError::new(
                Operation::Pass,
                &declared.extension,
                &format!("__pass_{}", pass.name),
                failure.clone(),
            )),
        };
        let findings = match answer {
            Ok(output) => {
                let extra = output.summary.clone();
                summary.extend(extra);
                pass_findings(output, input.graph)
            }
            Err(error) => {
                summary.insert("failed".into(), true.into());
                vec![error.diagnostic()]
            }
        };
        reports.push(ExtensionPassReport {
            name: report_name,
            findings,
            summary: serde_json::Value::Object(summary),
        });
    }
    reports
}

/// Keys an extension pass adds to its report summary.
pub type PassSummary = serde_json::Map<String, serde_json::Value>;
