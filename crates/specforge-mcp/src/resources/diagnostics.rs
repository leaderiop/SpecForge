use crate::resources::{ReadOutcome, ResourceText};
use crate::state::McpState;

pub fn read(state: &McpState) -> ReadOutcome {
    let json_str = specforge_common::serialize_diagnostics(&state.diagnostics());
    Ok(ResourceText::json("specforge://diagnostics", json_str))
}
