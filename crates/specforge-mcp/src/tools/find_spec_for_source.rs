use serde_json::{Value, json};

use specforge_common::inference::anchors;

use crate::state::McpState;
use crate::tool::ToolOutcome;

#[derive(Debug, serde::Deserialize)]
pub struct Args {
    #[serde(default, deserialize_with = "crate::args::lenient")]
    file_path: Option<String>,
}

pub fn call(state: &McpState, args: Args) -> ToolOutcome {
    let file_path = match args.file_path.as_deref() {
        Some(p) => p,
        None => {
            return ToolOutcome::ok(json!({"error": "Missing required parameter: file_path"}));
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

    let entities: Vec<Value> = manifest
        .anchors
        .iter()
        .filter(|a| a.file == file_path)
        .map(|a| {
            json!({
                "entity_id": a.entity_id,
                "line": a.line,
                "symbol_name": a.symbol_name,
                "item_kind": a.item_kind,
                "confidence": a.confidence,
            })
        })
        .collect();

    let result = json!({
        "file_path": file_path,
        "entities": entities,
        "count": entities.len(),
    });

    ToolOutcome::ok(result)
}
