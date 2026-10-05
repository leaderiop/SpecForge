use serde_json::json;

use crate::target::Call;
use crate::tool::{Handled, ToolOutcome};

#[derive(Debug, serde::Deserialize)]
pub struct Args {
    entity_id: String,
}

/// `specforge.find_definition`: where the entity is declared. `line` and
/// `column` are its name's (where a cursor goes); `source_span` is its
/// block, `name_span` its name (the block when the name could not be
/// read: `precision` says which).
pub fn call(call: &mut Call<'_>, args: Args) -> Handled {
    let entity_id = args.entity_id.as_str();
    let definition = super::navigator(call)
        .definition(entity_id)
        .map_err(|_| super::entity_not_found(entity_id))?;
    Ok(ToolOutcome::ok(json!({
        "entity_id": definition.id,
        "file_path": definition.name.file,
        "line": definition.name.start_line,
        "column": definition.name.start_col,
        "source_span": super::span_json(&definition.block),
        "name_span": super::span_json(&definition.name),
        "precision": definition.precision.as_str(),
    })))
}
