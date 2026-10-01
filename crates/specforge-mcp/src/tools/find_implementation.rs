use serde_json::{Value, json};

use specforge_common::inference::anchors;

use crate::state::McpState;
use crate::tool::ToolOutcome;

#[derive(Debug, serde::Deserialize)]
pub struct Args {
    #[serde(default, deserialize_with = "crate::args::lenient")]
    entity_id: Option<String>,
}

pub fn call(state: &McpState, args: Args) -> ToolOutcome {
    let entity_id = match args.entity_id.as_deref() {
        Some(id) => id,
        None => {
            return ToolOutcome::ok(json!({"error": "Missing required parameter: entity_id"}));
        }
    };

    let project_root = match &state.project_root {
        Some(p) => p.clone(),
        None => {
            return ToolOutcome::ok(json!({"error": "No project root available"}));
        }
    };

    let manifest = match anchors::load_anchor_manifest(&project_root) {
        Ok(m) => m,
        Err(e) => {
            return ToolOutcome::ok(json!({"error": e}));
        }
    };

    let sources: Vec<Value> = manifest
        .anchors
        .iter()
        .filter(|a| a.entity_id == entity_id)
        .map(|a| {
            json!({
                "file": a.file,
                "line": a.line,
                "symbol_name": a.symbol_name,
                "item_kind": a.item_kind,
                "scanner": a.scanner,
            })
        })
        .collect();

    let result = json!({
        "entity_id": entity_id,
        "implementations": sources,
        "count": sources.len(),
    });

    ToolOutcome::ok(result)
}
