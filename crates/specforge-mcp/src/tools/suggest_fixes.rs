use serde_json::Value;

use crate::state::McpState;
use crate::tool::ToolOutcome;

#[derive(Debug, serde::Deserialize)]
pub struct Args {
    #[serde(default, deserialize_with = "crate::args::lenient")]
    entity_id: Option<String>,
    #[serde(default, deserialize_with = "crate::args::lenient")]
    file_path: Option<String>,
    #[serde(default, deserialize_with = "crate::args::lenient")]
    diagnostic_code: Option<String>,
}

pub fn call(state: &McpState, args: Args) -> ToolOutcome {
    let entity = match args.entity_id.as_deref() {
        Some(entity_id) => match state.graph.node(entity_id) {
            Some(node) => Some(node),
            None => {
                return ToolOutcome::invalid_params(format!("Entity not found: {entity_id}"));
            }
        },
        None => None,
    };
    let file_path = args.file_path.as_deref();
    let code = args.diagnostic_code.as_deref();

    let suggestions: Vec<Value> = state
        .diagnostics
        .iter()
        .filter(|d| entity.is_none_or(|node| super::inspect::belongs_to(d, node)))
        .filter(|d| {
            file_path.is_none_or(|file| d.span.as_ref().is_some_and(|span| span.file == file))
        })
        .filter(|d| code.is_none_or(|code| d.code == code))
        .filter_map(|d| {
            d.suggestion.as_ref().map(|sug| {
                serde_json::json!({
                    "title": sug,
                    "kind": "quickfix",
                    "diagnostic_code": d.code,
                    "edits": []
                })
            })
        })
        .collect();

    ToolOutcome::ok(Value::Array(suggestions))
}
