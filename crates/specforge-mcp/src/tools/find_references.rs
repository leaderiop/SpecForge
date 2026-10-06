use serde_json::{Value, json};
use specforge_ops::navigate::{Direction, Occurrence, ReferenceQuery};

use crate::args::lenient;
use crate::target::Call;
use crate::tool::{Handled, ToolOutcome};

#[derive(Debug, serde::Deserialize)]
pub struct Args {
    entity_id: String,
    #[serde(default, deserialize_with = "lenient")]
    direction: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    include_declaration: Option<bool>,
}

/// `specforge.find_references`: each occurrence of the entity's ID, as
/// the LSP's references answer it (ADR 0016). Incoming by default, the
/// declaration only when asked for.
pub fn call(call: &mut Call<'_>, args: Args) -> Handled {
    let entity_id = args.entity_id.as_str();
    let direction = match args.direction.as_deref() {
        None => Direction::Incoming,
        Some(name) => match Direction::parse(name) {
            Some(direction) => direction,
            None => {
                return Ok(ToolOutcome::invalid_input(
                    "direction",
                    format!(
                        "unknown direction '{name}': expected \"incoming\", \"outgoing\" or \"both\""
                    ),
                ));
            }
        },
    };
    let query = ReferenceQuery {
        direction,
        include_declaration: args.include_declaration.unwrap_or(false),
    };
    let occurrences = super::navigator(call)
        .references(entity_id, query)
        .map_err(|_| crate::tool::entity_not_found(entity_id))?;
    Ok(ToolOutcome::ok(json!({
        "entity_id": entity_id,
        "direction": direction.as_str(),
        "locations": occurrences.iter().map(location).collect::<Vec<Value>>(),
    })))
}

/// One occurrence as an `McpReferenceLocation`.
pub(crate) fn location(occurrence: &Occurrence) -> Value {
    json!({
        "referencing_entity_id": occurrence.holder,
        "referenced_entity_id": occurrence.target,
        "field": occurrence.field,
        "role": occurrence.role.as_str(),
        "precision": occurrence.precision.as_str(),
        "source_span": super::span_json(&occurrence.span),
    })
}
