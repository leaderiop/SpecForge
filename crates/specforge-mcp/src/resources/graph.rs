use serde_json::Value;
use specforge_ops::export::{Format, Request};

use crate::resources::{ReadOutcome, ResourceText, invalid_params};
use crate::state::McpState;

/// `specforge://graph` — full corpus, or scoped via query parameters
/// (C9-06): `?root=<entity_id>` scopes to a subgraph, `depth=<n>` bounds the
/// traversal, `kinds=a,b` filters node kinds, `max_tokens=<n>` budgets the
/// payload. It is `specforge export --format graph` through the same
/// function (ADR 0004 D3-a): the full graph embeds the schema, a scoped one
/// references it (`schema_ref`), and a budgeted one leaves it out.
pub fn read(state: &McpState, root: Option<&std::path::Path>, uri: &str) -> ReadOutcome {
    let (base, query) = crate::resources::split_query(uri);
    let parsed = crate::resources::parse_query(query);

    let request = Request {
        format: Some(Format::Graph),
        scope: parsed.root,
        depth: parsed.depth,
        kinds: parsed.kinds,
        max_tokens: parsed.max_tokens,
        ..Request::default()
    };
    match crate::operations::export_graph(state.graph(), state.registries(), root, &request) {
        Ok(payload) => {
            let contents: Value =
                serde_json::from_str(&payload).expect("graph emit always produces JSON");
            Ok(ResourceText::json(base, contents.to_string()))
        }
        Err(err) => Err(invalid_params(err.message)),
    }
}
