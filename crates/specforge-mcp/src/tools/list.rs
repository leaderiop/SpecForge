use serde_json::{Map, Value, json};
use specforge_ops::query::{ListRequest, Listing, list};

use crate::args::Arguments;
use crate::tool::ToolOutcome;
use specforge_ops::view::ProjectView;

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

/// `specforge.list`: the list read view (`specforge_ops::query::list`), the
/// entities of `kind` (every entity without one) whose fields hold what
/// `where` asks, sorted by id, then paged by `offset` and `limit`. Domain
/// free: any kind, any field (an extension's own list commands, such as
/// `specforge.product.features`, render their kinds their way). A kind the
/// project does not know lists nothing and is reported (I020).
pub fn call(view: ProjectView<'_>, args: Args) -> ToolOutcome {
    let request = ListRequest {
        kind: args.kind.as_deref(),
        fields: args.r#where.as_ref(),
        offset: args.offset.unwrap_or(0),
        limit: args.limit,
    };
    let listing = list(&view, &request);
    let rows = rows(&listing);
    ToolOutcome::ok(rows).with_diagnostics(listing.notices)
}

/// `[{id, kind, title}]`: the one presenter of a listing, the tool's and
/// `specforge://entities/{kind}`'s.
pub(crate) fn rows(listing: &Listing) -> Value {
    Value::Array(
        listing
            .entities
            .iter()
            .map(|node| {
                json!({
                    "id": node.id.raw.as_str(),
                    "kind": node.kind.raw.as_str(),
                    "title": node.title.as_deref().unwrap_or(""),
                })
            })
            .collect(),
    )
}
