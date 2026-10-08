use serde_json::{Value, json};

use specforge_ops::navigate;

use crate::args::Arguments;
use crate::target::ProjectRef;
use crate::tool::{McpError, ToolOutcome};

/// `specforge.find_implementation`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Entity ID to find implementations for
    entity_id: String,
}

/// The source items the anchors manifest anchors the entity to
/// (`specforge_ops::navigate::anchors_of_entity`), in manifest order.
pub fn call(project: &ProjectRef<'_>, args: Args) -> ToolOutcome {
    let anchors = match navigate::source_anchors(&project.view()) {
        Ok(anchors) => anchors,
        Err(error) => return McpError::from(error).into(),
    };

    let implementations: Vec<Value> = navigate::anchors_of_entity(&anchors, &args.entity_id)
        .into_iter()
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

    ToolOutcome::ok(json!({
        "entity_id": args.entity_id,
        "implementations": implementations,
        "count": implementations.len(),
    }))
}
