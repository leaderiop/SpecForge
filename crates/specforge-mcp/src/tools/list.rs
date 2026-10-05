use serde_json::{Map, Value, json};

use crate::state::McpState;
use crate::tool::ToolOutcome;

#[derive(Debug, serde::Deserialize)]
pub struct Args {
    #[serde(default, deserialize_with = "crate::args::lenient")]
    kind: Option<String>,
    /// Field name to the value the field must hold. Unlike `kind`, read
    /// strictly: a `where`, `limit` or `offset` of the wrong type (a
    /// negative or fractional count) is invalid input, an `isError` result
    /// (ADR 0004 D4-a), never a silently unfiltered list.
    #[serde(default, rename = "where")]
    where_fields: Option<Map<String, Value>>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    offset: Option<usize>,
}

/// The entities of `kind` (every entity without one) whose fields hold what
/// `where` asks, sorted by id, then paged by `offset` and `limit`. Domain
/// free: any kind, any field (an extension's own list commands, such as
/// `specforge.product.features`, render their kinds their way).
pub fn call(state: &McpState, args: Args) -> ToolOutcome {
    let kind = args.kind.as_deref().filter(|k| !k.is_empty());
    let wanted = args.where_fields.unwrap_or_default();
    let entities: Vec<Value> = state
        .graph()
        .nodes()
        .into_iter()
        .filter(|n| kind.is_none_or(|k| n.kind.raw.as_str() == k))
        .filter(|n| {
            wanted.iter().all(|(field, value)| {
                n.fields
                    .entries()
                    .iter()
                    .find(|e| e.key.as_str() == field)
                    .is_some_and(|e| &specforge_emitter::field_value_to_json(&e.value) == value)
            })
        })
        .skip(args.offset.unwrap_or(0))
        .take(args.limit.unwrap_or(usize::MAX))
        .map(|n| {
            json!({
                "id": n.id.raw.as_str(),
                "kind": n.kind.raw.as_str(),
                "title": n.title.as_deref().unwrap_or(""),
            })
        })
        .collect();

    ToolOutcome::ok(Value::Array(entities))
}
