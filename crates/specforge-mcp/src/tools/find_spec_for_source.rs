use serde_json::{Value, json};

use specforge_common::inference::anchors;

use crate::target::Call;
use crate::tool::{Handled, ToolOutcome};

#[derive(Debug, serde::Deserialize)]
pub struct Args {
    file_path: String,
}

pub fn call(call: &mut Call<'_>, args: Args) -> Handled {
    let file_path = args.file_path.as_str();

    let project = call.project()?;
    let manifest = match anchors::load_anchor_manifest(project.root) {
        Ok(m) => m,
        Err(e) => return Ok(super::manifest_error(e)),
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

    Ok(ToolOutcome::ok(result))
}
