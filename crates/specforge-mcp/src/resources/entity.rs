use specforge_emitter::{EmitFormat, EmitOptions, emit};

use crate::resources::{ReadOutcome, ResourceText, invalid_params};
use crate::state::McpState;

pub fn read(state: &McpState, entity_id: &str) -> ReadOutcome {
    if entity_id.is_empty() {
        return Err(invalid_params("Malformed entity ID: must not be empty"));
    }
    // A malformed ID (the 400 case) is told apart from a well-formed one
    // that names no entity (the 404 case).
    if !entity_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | ':' | '-'))
    {
        return Err(invalid_params(format!(
            "Malformed entity ID: {:?} may only contain letters, digits, '_', '.', ':' and '-'",
            entity_id
        )));
    }

    let options = EmitOptions {
        format: EmitFormat::Json,
        scope: Some(entity_id),
        // The entity and its immediate neighbors, not everything reachable.
        depth: Some(1),
        ..EmitOptions::default()
    };

    match emit(state.graph(), &options) {
        Ok(json_str) => {
            let uri = format!("specforge://graph/{}", entity_id);
            Ok(ResourceText::json(uri, json_str))
        }
        Err(_) => Err(invalid_params(format!("Entity not found: {}", entity_id))),
    }
}
