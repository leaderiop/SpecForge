use serde_json::Value;
use specforge_emitter::{EmitFormat, EmitOptions, emit};

use crate::state::McpState;
use crate::tool::ToolOutcome;

pub fn call(state: &McpState, args: Value) -> ToolOutcome {
    let entity_id = match args.get("entity_id").and_then(|v| v.as_str()) {
        Some(e) => e,
        None => {
            return ToolOutcome::invalid_params("Missing required parameter: entity_id");
        }
    };

    let depth = args.get("depth").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("graph");
    let include_coverage = args
        .get("include_coverage")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let kinds: Vec<&str> = args
        .get("kinds")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    let unknown_kinds = super::unknown_kind_diagnostics(state, &kinds);

    let fmt = match format {
        "context" => EmitFormat::Context,
        "brief" => EmitFormat::Brief,
        _ => EmitFormat::Json,
    };
    let query_result = {
        let options = EmitOptions {
            format: fmt,
            scope: Some(entity_id),
            depth: Some(depth),
            kind_filter: kinds,
            field_registry: Some(&state.field_registry),
            ..EmitOptions::default()
        };
        emit(&state.graph, &options)
    };

    match query_result {
        Ok(json_str) => {
            let mut result: Value = serde_json::from_str(&json_str).unwrap_or(Value::Null);

            if include_coverage
                && let Some(nodes) = result.get_mut("nodes").and_then(|n| n.as_array_mut())
            {
                // The same classification `specforge.coverage` reports.
                let coverage = match super::coverage::project_coverage(state, "specforge.query") {
                    Ok(coverage) => coverage,
                    Err(outcome) => return outcome,
                };
                for node in nodes.iter_mut() {
                    let node_id = node.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let Some(verdict) = coverage.verdict(node_id) else {
                        continue;
                    };
                    let status = super::coverage::status_name(verdict.status());
                    node.as_object_mut()
                        .unwrap()
                        .insert("coverage_status".into(), Value::from(status));
                }
            }

            ToolOutcome::ok(result).with_diagnostics(unknown_kinds)
        }
        Err(err) => ToolOutcome::invalid_params(err.to_string()),
    }
}
