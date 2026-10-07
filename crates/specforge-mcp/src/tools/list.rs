use serde_json::{Map, Value, json};
use specforge_graph::Graph;

use crate::args::Arguments;
use crate::target::Call;
use crate::tool::ToolOutcome;

/// `specforge.list`'s arguments. A `where`, `limit` or `offset` of the wrong
/// type is invalid input, an `isError` result (ADR 0004 D4-a), as every
/// argument of a wrong type is: never a silently unfiltered list.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Filter by entity kind (e.g. 'feature', 'behavior')
    kind: Option<String>,
    /// Only entities whose fields hold these values, e.g. {"status": "done"}
    r#where: Option<Map<String, Value>>,
    /// Return at most this many entities
    limit: Option<usize>,
    /// Skip this many entities first
    offset: Option<usize>,
}

/// The entities of `kind` (every entity without one) whose fields hold what
/// `where` asks, sorted by id, then paged by `offset` and `limit`. Domain
/// free: any kind, any field (an extension's own list commands, such as
/// `specforge.product.features`, render their kinds their way).
pub fn call(call: &mut Call<'_>, args: Args) -> ToolOutcome {
    let Args {
        kind,
        r#where,
        limit,
        offset,
    } = args;
    let kind = kind.as_deref().filter(|k| !k.is_empty());
    let wanted = r#where.unwrap_or_default();
    // Unpaged unless asked: no offset is the first entity, no limit all.
    let entities = entities(
        call.view().graph(),
        kind,
        &wanted,
        offset.unwrap_or(0),
        limit.unwrap_or(usize::MAX),
    );
    ToolOutcome::ok(Value::Array(entities))
}

/// The rows [`call`] answers and `specforge://entities/{kind}` reads: the
/// graph's entities of `kind` (all of them without one) whose fields hold
/// what `wanted` asks, sorted by id, then paged by `offset` and `limit`.
pub(crate) fn entities(
    graph: &Graph,
    kind: Option<&str>,
    wanted: &Map<String, Value>,
    offset: usize,
    limit: usize,
) -> Vec<Value> {
    graph
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
        .skip(offset)
        .take(limit)
        .map(|n| {
            json!({
                "id": n.id.raw.as_str(),
                "kind": n.kind.raw.as_str(),
                "title": n.title.as_deref().unwrap_or(""),
            })
        })
        .collect()
}
