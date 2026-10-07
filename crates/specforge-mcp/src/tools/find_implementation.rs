use serde_json::{Value, json};

use specforge_common::inference::anchors;

use crate::args::Arguments;
use crate::target::Call;
use crate::tool::{Handled, ToolOutcome};

/// `specforge.find_implementation`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Entity ID to find implementations for
    entity_id: String,
}

pub fn call(call: &mut Call<'_>, args: Args) -> Handled {
    let entity_id = args.entity_id.as_str();

    let project = call.project()?;
    let manifest = match anchors::load_anchor_manifest(project.root) {
        Ok(m) => m,
        Err(e) => return Ok(super::manifest_error(e)),
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

    Ok(ToolOutcome::ok(result))
}
