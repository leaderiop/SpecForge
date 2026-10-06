use specforge_ops::view::ProjectView;

use crate::protocol::{JsonRpcError, error_codes};
use crate::resources::{ReadOutcome, ResourceText};

/// `specforge://schema`: the GraphProtocolSchema a full export embeds, the
/// same document `specforge.schema` returns unfiltered.
pub fn read(view: &ProjectView) -> ReadOutcome {
    let schema = view.versioned_schema();
    let schema_json = serde_json::to_string(&schema).map_err(|err| {
        JsonRpcError::new(
            error_codes::INTERNAL_ERROR,
            format!("schema serialization failed: {err}"),
        )
    })?;
    Ok(ResourceText::json(schema_json))
}
