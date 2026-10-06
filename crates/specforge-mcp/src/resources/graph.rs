use serde_json::Value;
use specforge_ops::export::{Format, Request};

use specforge_ops::view::ProjectView;

use crate::resources::{ReadOutcome, ResourceText, entity_not_found, invalid_params};

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
        // The export's message leads with its diagnostic code, as a
        // resource refusal always did; a scope the graph has no entity
        // for (E003) names the root.
        Err(err) => {
            let message = if crate::tool::is_diagnostic_code(&err.code) {
                format!("{}: {}", err.code, err.message)
            } else {
                err.message
            };
            if err.code == "E003" {
                Err(entity_not_found(message, parsed.root.unwrap_or_default()))
            } else {
                Err(invalid_params(message))
            }
        }
    }
}
