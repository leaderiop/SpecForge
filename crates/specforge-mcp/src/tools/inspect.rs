use serde_json::Value;
use specforge_graph::FieldValue;

use crate::state::McpState;
use crate::tool::ToolOutcome;

pub fn call(state: &McpState, args: Value) -> ToolOutcome {
    let entity_id = match args.get("entity_id").and_then(|v| v.as_str()) {
        Some(e) => e,
        None => {
            return ToolOutcome::invalid_params("Missing required parameter: entity_id");
        }
    };

    let node = match state.graph.node(entity_id) {
        Some(n) => n,
        None => {
            return ToolOutcome::failed(format!("Entity not found: {}", entity_id));
        }
    };

    let reference_count =
        state.graph.edges_to(entity_id).len() + state.graph.edges_from(entity_id).len();

    let contract = node.fields.get("contract").and_then(|v| match v {
        FieldValue::String(s) => Some(s.clone()),
        _ => None,
    });

    let obligations = specforge_emitter::coverage::obligations(node);
    let declared = !obligations.is_empty();
    // Whether the entity's kind counts toward coverage, as hover, the
    // schema and the outline say (ADR 0004, D2-d); `declared` says whether
    // the entity itself declares obligations.
    let testable = specforge_emitter::coverage::testable_kinds(&state.kind_registry)
        .contains(node.kind.raw.as_str());
    let verify_declarations: Option<Vec<String>> = declared.then(|| {
        obligations
            .iter()
            .map(|s| format!("{} {}", s.kind, s.description))
            .collect()
    });

    let references: Vec<String> = state
        .graph
        .edges_to(entity_id)
        .iter()
        .map(|e| e.source.to_string())
        .chain(
            state
                .graph
                .edges_from(entity_id)
                .iter()
                .map(|e| e.target.to_string()),
        )
        .collect();

    let entity_diagnostics: Vec<Value> = state
        .diagnostics
        .iter()
        .filter(|d| belongs_to(d, node))
        .map(|d| {
            serde_json::json!({
                "code": d.code,
                "severity": format!("{:?}", d.severity),
                "message": d.message
            })
        })
        .collect();

    // The same classification `specforge.coverage` reports.
    let coverage_status = match super::coverage::project_coverage(state, "specforge.inspect") {
        Ok(coverage) => super::coverage::status_name(coverage.status(entity_id)),
        Err(outcome) => return outcome,
    };

    let result = serde_json::json!({
        "entity_id": node.id.raw,
        "kind": node.kind.raw,
        "title": node.title,
        "testable": testable,
        "declared": declared,
        "reference_count": reference_count,
        "source_span": {
            "file": node.source_span.file,
            "start_line": node.source_span.start_line,
            "start_col": node.source_span.start_col,
            "end_line": node.source_span.end_line,
            "end_col": node.source_span.end_col,
        },
        "contract": contract,
        // Every field, whatever the kind names its text: an invariant's
        // `guarantee`, a decision's `rationale`, a feature's `description`.
        "fields": specforge_emitter::field_map_to_json(&node.fields),
        "verify_declarations": verify_declarations,
        "references": references,
        "coverage_status": coverage_status,
        "diagnostics": entity_diagnostics
    });

    ToolOutcome::ok(result)
}

/// Whether `diagnostic` is about `node`: its span lies within the node's,
/// or, without a span, its message names the node in quotes. A substring
/// match would give `task` the diagnostics of `task_id_uniqueness`.
pub(crate) fn belongs_to(
    diagnostic: &specforge_common::Diagnostic,
    node: &specforge_graph::Node,
) -> bool {
    let entity = &node.source_span;
    match &diagnostic.span {
        Some(span) => {
            span.file == entity.file
                && span.start_line >= entity.start_line
                && span.end_line <= entity.end_line
        }
        None => diagnostic.message.contains(&format!("'{}'", node.id.raw)),
    }
}
