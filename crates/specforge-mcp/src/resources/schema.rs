use crate::protocol::{JsonRpcError, error_codes};
use crate::resources::{ReadOutcome, ResourceText};
use crate::state::McpState;

/// `specforge://schema`: the GraphProtocolSchema a full export embeds, the
/// same document `specforge.schema` returns unfiltered.
pub fn read(state: &McpState, root: Option<&std::path::Path>) -> ReadOutcome {
    let schema = crate::operations::project_schema(state.registries(), root);
    let schema_json = serde_json::to_string(&schema).map_err(|err| {
        JsonRpcError::new(
            error_codes::INTERNAL_ERROR,
            format!("schema serialization failed: {err}"),
        )
    })?;
    Ok(ResourceText::json("specforge://schema", schema_json))
}
