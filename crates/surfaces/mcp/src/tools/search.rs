use crate::target::View;
use crate::tool::{CoreTool, Effect, ReplySpec, ToolGroup};
use specforge_ops::read::query::{EntityHits, FieldHolds, SearchField, SearchRequest, search};

use crate::args::Arguments;
use crate::refusal::ArgumentOf;
use crate::reply::{Answer, Answered};
use specforge_ops::view::ProjectView;

/// `specforge.search`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Search query
    query: String,
    /// Filter by kinds
    kinds: Vec<String>,
    /// Max results
    #[arg(default = specforge_ops::read::query::DEFAULT_SEARCH_LIMIT)]
    limit: usize,
    /// Only entities whose field of this name contains `value`; needs `value`
    field: Option<String>,
    /// With `field`: text the field contains, ignoring case; needs `field`
    value: Option<String>,
    /// Only entities that reference this entity ID (combined with the other filters)
    references: Option<String>,
}

/// `specforge.search`: the search read view (`specforge_ops::read::query::search`),
/// the entities the query finds, ranked as the LSP's workspace symbols and
/// completion rank them, over names and field text. Every filter is
/// ANDed: kinds, a field's text (`field` with `value`: one without the other
/// is refused by the operation, `FieldHolds::of`), and `references` (the entities that reference that one; one the
/// graph lacks is `entity_not_found` on `references`).
pub(crate) struct Tool;

impl ArgumentOf<SearchField> for Tool {
    fn argument(field: SearchField) -> Option<&'static str> {
        match field {
            SearchField::Referencing => Some("references"),
            SearchField::Field => Some("field"),
            SearchField::Value => Some("value"),
        }
    }
}

impl CoreTool for Tool {
    const NAME: &'static str = "specforge.search";
    const DESCRIPTION: &'static str = "Find entities by id, title or field text, ranked as the LSP ranks them (exact, prefix, substring, field text, then fuzzy)";
    const EFFECT: Effect = Effect::Reads {
        group: ToolGroup::Core,
    };
    const SPEC: ReplySpec = ReplySpec::Type("McpSearchResults");
    type Args = Args;
    type Input = View;
    type Returns = Answered<EntityHits>;

    fn call(view: ProjectView<'_>, args: Args) -> Answered<EntityHits> {
        let field =
            FieldHolds::of(args.field.as_deref(), args.value.as_deref()).map_err(Self::refused)?;
        let request = SearchRequest {
            text: &args.query,
            kinds: args.kinds.iter().map(String::as_str).collect(),
            field,
            referencing: args.references.as_deref(),
            limit: Some(args.limit),
        };
        Ok(Answer::noticed(
            search(&view, &request).map_err(Self::refused)?,
        ))
    }
}
