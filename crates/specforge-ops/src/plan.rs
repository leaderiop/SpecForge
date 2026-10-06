//! An agent plan checked against the graph: the trace tool's plan mode,
//! one operation over the project view (ADR 0015).

use serde::Serialize;
use serde_json::Value;
use specforge_project::coverage::{ProjectCoverage, ReportError};
use std::collections::{HashMap, HashSet};

use specforge_emitter::SCHEMA_VERSION;

use crate::OpError;
use crate::view::ProjectView;

/// How a plan falls short of the graph.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanOutcome {
    /// Plan entries that name an entity in the graph, in plan order.
    pub entries: Vec<String>,
    /// Every error, warning and ordering violation below, as a record.
    pub gaps: Vec<PlanGap>,
    /// Entries naming no entity (E003).
    pub errors: Vec<String>,
    /// Testable entities with obligations the plan has no entry for.
    pub warnings: Vec<String>,
    /// Entries listed before an entity they depend on.
    pub ordering_violations: Vec<String>,
}

/// Why a plan could not be checked.
#[derive(Debug, Clone, PartialEq)]
pub enum PlanError {
    /// The value is not an `AgentPlan`: why.
    NotAPlan(String),
    /// The recorded test report, which says which entities owe
    /// obligations, is there but unusable (E045).
    Report(ReportError),
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanError::NotAPlan(why) => f.write_str(why),
            PlanError::Report(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for PlanError {}

/// `invalid_input` for a value that is no plan; E045 for the report.
impl From<PlanError> for OpError {
    fn from(error: PlanError) -> Self {
        match error {
            PlanError::NotAPlan(why) => OpError::new("invalid_input", why),
            PlanError::Report(error) => error.diagnostic().into(),
        }
    }
}

/// One way a plan falls short of the graph. `source` is `"plan"` when the
/// plan itself is at fault rather than one of its entries.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlanGap {
    pub source: String,
    pub target: String,
    pub kind: PlanGapKind,
    /// The same text as the matching error, warning or violation.
    pub context: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanGapKind {
    /// An entry names no entity in the graph.
    UnresolvedEntity,
    /// A testable entity with obligations has no entry.
    MissingPlanEntry,
    /// An entry comes before an entity it depends on.
    Ordering,
}

impl PlanGapKind {
    pub fn as_str(self) -> &'static str {
        match self {
            PlanGapKind::UnresolvedEntity => "unresolved_entity",
            PlanGapKind::MissingPlanEntry => "missing_plan_entry",
            PlanGapKind::Ordering => "ordering",
        }
    }
}

/// The side of a gap that is the plan itself.
const PLAN: &str = "plan";

/// Check `plan`, an `AgentPlan` object or JSON text of one, against the
/// view's graph: entries naming no entity, testable entities with
/// obligations it has no entry for, and entries ordered before an entity
/// they depend on.
pub fn check(view: &ProjectView, plan: &Value) -> Result<PlanOutcome, PlanError> {
    let parsed;
    let plan = match plan {
        Value::String(text) => {
            parsed = serde_json::from_str::<Value>(text)
                .map_err(|e| PlanError::NotAPlan(format!("plan is not valid JSON: {e}")))?;
            &parsed
        }
        other => other,
    };
    let Some(entries) = plan.get("entries").and_then(Value::as_array) else {
        return Err(PlanError::NotAPlan(
            "plan must be an AgentPlan object with an entries array".into(),
        ));
    };
    for (i, entry) in entries.iter().enumerate() {
        if !entry.get("entity_id").is_some_and(Value::is_string) {
            return Err(PlanError::NotAPlan(format!(
                "plan.entries[{i}].entity_id must be a string"
            )));
        }
    }
    let coverage = view.coverage().map_err(PlanError::Report)?;
    Ok(validate(view, entries, &coverage))
}

fn validate(view: &ProjectView, entries: &[Value], coverage: &ProjectCoverage) -> PlanOutcome {
    let graph = view.graph;
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut ordering_violations = Vec::new();
    let mut validated_entries = Vec::new();
    let mut gaps = Vec::new();

    let mut plan_ids: Vec<String> = Vec::new();

    // Validate each entry
    for entry in entries {
        if let Some(id) = entry["entity_id"].as_str() {
            plan_ids.push(id.to_string());
            if graph.node(id).is_some() {
                validated_entries.push(id.to_string());
            } else {
                let message = format!(
                    "E003: unresolved entity '{}' in plan — not found in graph",
                    id
                );
                gaps.push(PlanGap {
                    source: PLAN.to_string(),
                    target: id.to_string(),
                    kind: PlanGapKind::UnresolvedEntity,
                    context: message.clone(),
                });
                errors.push(message);
            }
        }
    }
    let plan_id_set: HashSet<String> = plan_ids.iter().cloned().collect();

    // Testable entities that declare obligations, missing from the plan.
    for (record, standing) in coverage.entities().iter() {
        let id = &record.id;
        let obliged = coverage
            .verdict(id)
            .is_some_and(|verdict| verdict.obligations > 0);
        if standing.testable && obliged && !plan_id_set.contains(id.as_str()) {
            let message = format!(
                "testable entity '{id}' ({}) is not covered by the plan",
                record.kind
            );
            gaps.push(PlanGap {
                source: PLAN.to_string(),
                target: id.clone(),
                kind: PlanGapKind::MissingPlanEntry,
                context: message.clone(),
            });
            warnings.push(message);
        }
    }

    // Validate dependency ordering: if plan lists A before B, but graph has edge A->B
    // (A depends on B), that's fine. But if B->A exists (B depends on A) and B comes
    // before A in the plan, B would be implemented before its dependency A.
    // Build position map for plan ordering
    let position: HashMap<&str, usize> = plan_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();

    for edge in graph.edges() {
        // edge.source -> edge.target means source references target
        // So target should be implemented before source
        if let (Some(&src_pos), Some(&tgt_pos)) = (
            position.get(edge.source.as_str()),
            position.get(edge.target.as_str()),
        ) && tgt_pos > src_pos
        {
            let message = format!(
                "'{}' depends on '{}' (via {}), but '{}' appears later in the plan",
                edge.source, edge.target, edge.label, edge.target
            );
            gaps.push(PlanGap {
                source: edge.source.to_string(),
                target: edge.target.to_string(),
                kind: PlanGapKind::Ordering,
                context: message.clone(),
            });
            ordering_violations.push(message);
        }
    }

    PlanOutcome {
        entries: validated_entries,
        gaps,
        errors,
        warnings,
        ordering_violations,
    }
}

/// The plan check as a JSON report: `schema_version`, `errors`,
/// `warnings`, `ordering_violations`, `validated_entries`.
pub fn serialize_plan_result(result: &PlanOutcome) -> String {
    #[derive(Serialize)]
    struct Output<'a> {
        schema_version: &'static str,
        errors: &'a [String],
        warnings: &'a [String],
        ordering_violations: &'a [String],
        validated_entries: &'a [String],
    }

    let output = Output {
        schema_version: SCHEMA_VERSION,
        errors: &result.errors,
        warnings: &result.warnings,
        ordering_violations: &result.ordering_violations,
        validated_entries: &result.entries,
    };

    serde_json::to_string(&output).expect("serialization cannot fail")
}
