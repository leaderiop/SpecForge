use serde_json::json;

use crate::resources::{ReadOutcome, ResourceText};
use crate::state::McpState;

pub fn read(state: &McpState, kind: &str) -> ReadOutcome {
    let entities: Vec<serde_json::Value> = state
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
        .collect();

    let text = serde_json::to_string(&entities).unwrap();
    Ok(ResourceText::json(
        format!("specforge://entities/{}", kind),
        text,
    ))
}
