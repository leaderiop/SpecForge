use serde_json::Value;
use specforge_ops::export::{Format, Request};

use specforge_ops::view::ProjectView;

use crate::resources::{ReadOutcome, ResourceText};
use crate::tool::McpError;

/// `specforge://graph` — full corpus, or scoped via query parameters
/// (C9-06): `?root=<entity_id>` scopes to a subgraph, `depth=<n>` bounds the
/// traversal, `kinds=a,b` filters node kinds, `max_tokens=<n>` budgets the
/// payload. It is `specforge export --format graph` through the same
/// function (ADR 0004 D3-a): the full graph embeds the schema, a scoped one
/// references it (`schema_ref`), and a budgeted one leaves it out.
pub fn read(view: &ProjectView, uri: &str) -> ReadOutcome {
    let (_, query) = crate::resources::split_query(uri);
    let parsed = crate::resources::parse_query(query);

    let request = Request {
        format: Some(Format::Graph),
        scope: parsed.root,
        depth: parsed.depth,
        kinds: parsed.kinds,
        max_tokens: parsed.max_tokens,
        ..Request::default()
    };
    match specforge_ops::export::export(view, &request) {
        Ok(payload) => {
            let contents: Value =
                serde_json::from_str(&payload).expect("graph emit always produces JSON");
            Ok(ResourceText::json(contents.to_string()))
        }
        // The operation's failure: a scope the graph has no entity for is
        // `entity_not_found` with its E003, a budget too small `invalid_input`
        // with its E062.
        Err(error) => Err(Box::new(McpError::from(error))),
    }
}
