use serde::Deserialize;
use serde_json::Value;
use specforge_emitter::{EmitFormat, EmitOptions, emit};

use crate::args::{lenient, strings};
use crate::state::McpState;
use crate::tool::ToolOutcome;

#[derive(Debug, Deserialize)]
pub struct Args {
    entity_id: String,
    #[serde(default, deserialize_with = "lenient")]
    depth: Option<u64>,
    #[serde(default, deserialize_with = "strings")]
    kinds: Vec<String>,
    #[serde(default, deserialize_with = "lenient")]
    format: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    include_coverage: Option<bool>,
}

pub fn call(state: &McpState, args: Args) -> ToolOutcome {
    let entity_id = args.entity_id.as_str();
    let depth = args.depth.unwrap_or(1) as usize;
    let format = args.format.as_deref().unwrap_or("graph");
    let include_coverage = args.include_coverage.unwrap_or(false);

    let kinds: Vec<&str> = args.kinds.iter().map(String::as_str).collect();
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
        Err(err) => super::emitter_error(err, entity_id),
    }
}
