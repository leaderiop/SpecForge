use serde::Serialize;
use serde_json::Value;
use specforge_graph::{FieldValue, Graph};
use std::collections::{HashMap, HashSet};

use crate::json::SCHEMA_VERSION;

#[derive(Debug)]
pub struct PlanValidationResult {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub ordering_violations: Vec<String>,
    pub validated_entries: Vec<String>,
}

pub fn validate_plan(
    graph: &Graph,
    plan: &Value,
    testable_kinds: &[&str],
) -> PlanValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut ordering_violations = Vec::new();
    let mut validated_entries = Vec::new();

    let entries = plan["entries"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let mut plan_ids: Vec<String> = Vec::new();
    

    // Validate each entry
    for entry in &entries {
        if let Some(id) = entry["entity_id"].as_str() {
            plan_ids.push(id.to_string());
            if graph.node(id).is_some() {
                validated_entries.push(id.to_string());
            } else {
                errors.push(format!("E003: unresolved entity '{}' in plan — not found in graph", id));
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
        let has_verify = matches!(
            node.fields.get("verify"),
            Some(FieldValue::VerifyList(stmts)) if !stmts.is_empty()
        );
        if has_verify && !plan_id_set.contains(node.id.raw.as_str()) {
            warnings.push(format!(
                "testable entity '{}' ({}) is not covered by the plan",
                node.id.raw, node.kind.raw
            ));
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
        if let (Some(&src_pos), Some(&tgt_pos)) =
            (position.get(edge.source.as_str()), position.get(edge.target.as_str()))
            && tgt_pos > src_pos {
                ordering_violations.push(format!(
                    "'{}' depends on '{}' (via {}), but '{}' appears later in the plan",
                    edge.source, edge.target, edge.label, edge.target
                ));
            }
    }

    PlanValidationResult {
        errors,
        warnings,
        ordering_violations,
        validated_entries,
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

    serde_json::to_string_pretty(&output).expect("serialization cannot fail")
}
