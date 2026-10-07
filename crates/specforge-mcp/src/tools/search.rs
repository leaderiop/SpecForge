use serde_json::{Value, json};
use specforge_ops::navigate::{EntityQuery, MatchScope, MatchedOn, find_entities, snippet};

use crate::args::Arguments;
use crate::target::Call;
use crate::tool::ToolOutcome;

/// Characters of a matched field's text shown on each side of the match.
const SNIPPET_WIDTH: usize = 40;

/// `specforge.search`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Search query
    query: String,
    /// Filter by kinds
    kinds: Vec<String>,
    /// Max results
    #[arg(default = 20)]
    limit: usize,
    /// Only search a specific field
    field: Option<String>,
    /// Exact field value filter (with field)
    value: Option<String>,
    /// Only entities that reference this entity ID (combined with the other filters)
    references: Option<String>,
}

/// `specforge.search`: the entities the query finds, ranked as the LSP's
/// workspace symbols and completion rank them, over names and string
/// fields (`specforge_ops::navigate::find_entities`). Every filter is
/// ANDed: kinds, a field's value, and `references` (the entities that
/// reference that one).
pub fn call(call: &mut Call<'_>, args: Args) -> ToolOutcome {
    let view = call.view();
    let kinds: Vec<&str> = args.kinds.iter().map(String::as_str).collect();
    let unknown_kinds = super::unknown_kind_diagnostics(&view, &kinds);
    let query = EntityQuery {
        text: &args.query,
        scope: MatchScope::NamesAndText,
        kinds: &kinds,
        field_contains: args.field.as_deref().zip(args.value.as_deref()),
        referencing: args.references.as_deref(),
        limit: Some(args.limit),
    };
    let results: Vec<Value> = find_entities(view.graph(), &query)
        .iter()
        .map(|m| {
            let mut result = json!({
                "entity_id": m.node.id.raw,
                "kind": m.node.kind.raw,
                "title": m.node.title,
                "file_path": m.node.source_span.file,
                "line": m.node.source_span.start_line,
                "score": m.score,
                "match_field": m.on.field_name(),
            });
            if let MatchedOn::Field(field) = m.on
                && let Some(text) = snippet(m.node, field, &args.query, SNIPPET_WIDTH)
            {
                result["match_snippet"] = Value::String(text);
            }
            result
        })
        .collect();
    ToolOutcome::ok(Value::Array(results)).with_diagnostics(unknown_kinds)
}
