use serde_json::{Value, json};

use specforge_common::inference::anchors;

use crate::target::Call;
use crate::tool::ToolOutcome;

#[derive(Debug, serde::Deserialize)]
pub struct Args {
    file_path: String,
}

pub fn call(call: &mut Call<'_>, args: Args) -> ToolOutcome {
    let state = &*call.state;
    let file_path = args.file_path.as_str();

    let project_root = match state.project_root() {
        Some(p) => p.to_path_buf(),
        None => {
            return ToolOutcome::no_project("No project root available");
        }
    };

    let manifest = match anchors::load_anchor_manifest(&project_root) {
        Ok(m) => m,
        Err(e) => return super::manifest_error(e),
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
