use serde_json::{Value, json};
use specforge_ops::query::{FieldHolds, Hit, SearchRequest, search};

use crate::args::Arguments;
use crate::target::Call;
use crate::tool::ToolOutcome;

/// `specforge.search`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Search query
    query: String,
    /// Filter by kinds
    kinds: Vec<String>,
    /// Max results
    #[arg(default = specforge_ops::query::DEFAULT_SEARCH_LIMIT)]
    limit: usize,
    /// Only entities whose field of this name contains `value`; needs `value`
    field: Option<String>,
    /// With `field`: text the field contains, ignoring case; needs `field`
    value: Option<String>,
    /// Only entities that reference this entity ID (combined with the other filters)
    references: Option<String>,
}

/// `specforge.search`: the search read view (`specforge_ops::query::search`),
/// the entities the query finds, ranked as the LSP's workspace symbols and
/// completion rank them, over names and string fields. Every filter is
/// ANDed: kinds, a field's text (`field` with `value`: one without the other
/// is refused), and `references` (the entities that reference that one).
pub fn call(call: &mut Call<'_>, args: Args) -> ToolOutcome {
    let field = match (args.field.as_deref(), args.value.as_deref()) {
        (Some(field), Some(value)) => Some(FieldHolds { field, value }),
        (None, None) => None,
        (Some(_), None) => {
            return ToolOutcome::invalid_input(
                "value",
                "'field' needs 'value': the text the field must contain",
            );
        }
        (None, Some(_)) => {
            return ToolOutcome::invalid_input(
                "field",
                "'value' needs 'field': the field whose text it must be in",
            );
        }
    };
    let request = SearchRequest {
        text: &args.query,
        kinds: args.kinds.iter().map(String::as_str).collect(),
        field,
        referencing: args.references.as_deref(),
        limit: Some(args.limit),
    };
    let outcome = search(&call.view(), &request);
    let results: Vec<Value> = outcome.hits.iter().map(hit_json).collect();
    ToolOutcome::ok(Value::Array(results)).with_diagnostics(outcome.notices)
}

/// One hit as `specforge.search` reports it.
fn hit_json(hit: &Hit) -> Value {
    let found = &hit.found;
    let mut result = json!({
        "entity_id": found.node.id.raw,
        "kind": found.node.kind.raw,
        "title": found.node.title,
        "file_path": found.node.source_span.file,
        "line": found.node.source_span.start_line,
        "score": found.score,
        "match_field": found.on.field_name(),
    });
    if let Some(snippet) = &hit.snippet {
        result["match_snippet"] = Value::String(snippet.clone());
    }
    result
}
