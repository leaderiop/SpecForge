use serde::Serialize;
use serde_json::Value;
use specforge_graph::Graph;
use std::collections::{HashMap, HashSet};

use crate::json::SCHEMA_VERSION;

#[derive(Debug)]
pub struct PlanValidationResult {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub ordering_violations: Vec<String>,
    pub validated_entries: Vec<String>,
    /// Every error, warning and ordering violation above, as a record.
    pub gaps: Vec<PlanGap>,
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

pub fn validate_plan(graph: &Graph, plan: &Value, testable_kinds: &[&str]) -> PlanValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut ordering_violations = Vec::new();
    let mut validated_entries = Vec::new();
    let mut gaps = Vec::new();

    let entries = plan["entries"].as_array().cloned().unwrap_or_default();

    let mut plan_ids: Vec<String> = Vec::new();

    // Validate each entry
    for entry in &entries {
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

    // Check for testable entities missing from plan
    let testable_set: HashSet<&str> = testable_kinds.iter().copied().collect();
    for node in graph.nodes() {
        if !testable_set.contains(node.kind.raw.as_str()) {
            continue;
        }
        let has_verify = !specforge_graph::obligations(node).is_empty();
        if has_verify && !plan_id_set.contains(node.id.raw.as_str()) {
            let message = format!(
                "testable entity '{}' ({}) is not covered by the plan",
                node.id.raw, node.kind.raw
            );
            gaps.push(PlanGap {
                source: PLAN.to_string(),
                target: node.id.raw.to_string(),
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

    PlanValidationResult {
        errors,
        warnings,
        ordering_violations,
        validated_entries,
        gaps,
    }
}

pub fn serialize_plan_result(result: &PlanValidationResult) -> String {
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
        validated_entries: &result.validated_entries,
    };

    serde_json::to_string(&output).expect("serialization cannot fail")
}
