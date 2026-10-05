use serde_json::{Value, json};

use specforge_common::inference::anchors;

use crate::target::Call;
use crate::tool::ToolOutcome;

#[derive(Debug, serde::Deserialize)]
pub struct Args {
    entity_id: String,
}

pub fn call(call: &mut Call<'_>, args: Args) -> ToolOutcome {
    let state = &*call.state;
    let entity_id = args.entity_id.as_str();

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
