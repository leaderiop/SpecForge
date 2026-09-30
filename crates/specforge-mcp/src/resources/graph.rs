use serde_json::Value;
use specforge_ops::export::{Format, Request};

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

/// `specforge://graph` — full corpus, or scoped via query parameters
/// (C9-06): `?root=<entity_id>` scopes to a subgraph, `depth=<n>` bounds the
/// traversal, `kinds=a,b` filters node kinds, `max_tokens=<n>` budgets the
/// payload. It is `specforge export --format graph` through the same
/// function (ADR 0004 D3-a): the full graph embeds the schema, a scoped one
/// references it (`schema_ref`), and a budgeted one leaves it out.
pub fn read(state: &McpState, uri: &str, id: Option<Value>) -> JsonRpcResponse {
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
    match crate::operations::export_graph(state, &request) {
        Ok(payload) => {
            let contents: Value =
                serde_json::from_str(&payload).expect("graph emit always produces JSON");
            JsonRpcResponse::success(id, resource_contents(base, contents))
        }
        Err(err) => JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, err.message),
    }
}

fn resource_contents(uri: &str, contents: Value) -> Value {
    serde_json::json!({
        "contents": [{
            "uri": uri,
            "mimeType": "application/json",
            "text": contents.to_string()
        }]
    })
}
