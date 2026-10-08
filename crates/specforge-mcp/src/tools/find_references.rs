use serde_json::{Value, json};
use specforge_ops::navigate::{DIRECTION, Direction, Occurrence, ReferenceQuery};

use crate::args::Arguments;
use crate::tool::{Handled, ToolOutcome};
use specforge_ops::view::ProjectView;

/// `specforge.find_references`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Entity ID
    entity_id: String,
    /// Which references
    #[arg(choice = specforge_ops::navigate::DIRECTION)]
    direction: Direction,
    /// Also return the entity's own declaration (its name)
    include_declaration: bool,
}

/// `specforge.find_references`: each occurrence of the entity's ID, as
/// the LSP's references answer it (ADR 0016). Incoming by default, the
/// declaration only when asked for.
pub fn call(view: ProjectView<'_>, args: Args) -> Handled {
    let entity_id = args.entity_id.as_str();
    let direction = args.direction;
    let query = ReferenceQuery {
        direction,
        include_declaration: args.include_declaration,
    };
    let occurrences = super::navigator(view)
        .references(entity_id, query)
        .map_err(crate::tool::McpError::from)?;
    Ok(ToolOutcome::ok(json!({
        "entity_id": entity_id,
        "direction": DIRECTION.name_of(direction),
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
