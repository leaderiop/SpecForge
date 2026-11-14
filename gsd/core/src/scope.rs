//! The scope helpers the passes share: the scope an entity's id puts it in, the quick plan it names, and what owns it.

use specforge_extension_sdk::prelude::*;

/// The scope an entity is in: its id without the last `/`-segment.
pub fn scope_of(id: &str) -> &str {
    id.rsplit_once('/').map_or("", |(scope, _)| scope)
}

/// The quick plan a context, decision or research names in `plan`: its resolved edge.
pub fn quick_plan<'a>(input: &'a PassInput, id: &str) -> Option<&'a str> {
    input
        .edges
        .iter()
        .find(|e| e.source == id && e.label == "plan")
        .map(|e| e.target.as_str())
}

/// What owns a context, decision or research: its quick plan, else its scope (its phase, or `""` for the project).
pub fn owner<'a>(input: &'a PassInput, entity: &'a PassEntity) -> &'a str {
    quick_plan(input, &entity.id).unwrap_or_else(|| scope_of(&entity.id))
}
