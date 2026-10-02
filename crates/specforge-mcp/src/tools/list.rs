use serde_json::{Value, json};

use crate::state::McpState;
use crate::tool::ToolOutcome;

#[derive(Debug, serde::Deserialize)]
pub struct Args {
    #[serde(default, deserialize_with = "crate::args::lenient")]
    kind: Option<String>,
}

pub fn call(state: &McpState, args: Args) -> ToolOutcome {
    let kind = args.kind.as_deref().unwrap_or("");

    let entities: Vec<Value> = if kind.is_empty() {
        // No kind filter: return all entities
        state
            .graph()
            .nodes()
            .iter()
            .map(|n| {
                json!({
                    "id": n.id.raw.as_str(),
                    "kind": n.kind.raw.as_str(),
                    "title": n.title.as_deref().unwrap_or(""),
                })
            })
            .collect()
    } else {
        // Filter by kind
        state
            .graph()
            .nodes_by_kind(kind)
            .iter()
            .map(|n| {
                json!({
                    "id": n.id.raw.as_str(),
                    "kind": n.kind.raw.as_str(),
                    "title": n.title.as_deref().unwrap_or(""),
                })
            })
            .collect()
    };

    ToolOutcome::ok(Value::Array(entities))
}
