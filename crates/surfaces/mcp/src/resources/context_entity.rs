use specforge_ops::read::context::{ContextRequest, context};
use specforge_ops::read::export::Format;
use specforge_ops::view::ProjectView;

use super::view::{ViewQuery, check_entity_id, exported};
use crate::resource::{CoreResource, ReadOutcome, ResourceText};
use crate::subscriptions::Watched;
use crate::tool::McpError;
use crate::uri_template::Matched;

/// `specforge://context/{+entity_id}`: the context export scoped to an entity, as `context?scope=` is: everything it
/// reaches; with `?role=`, the pack that context route declares for it (ADR 0356), the text `specforge.context`
/// answers.
pub struct ContextEntityResource;

impl CoreResource for ContextEntityResource {
    const URI: &'static str = "specforge://context/{+entity_id}";
    const NAME: &'static str = "context_entity";
    const DESCRIPTION: &'static str = "Context-optimized subgraph of everything an entity reaches, each entity whole (a depth query narrows it); with role, the pack that route declares for it";
    const CHANGES_WITH: Watched = Watched::Content;

    fn read(view: ProjectView<'_>, uri: &Matched<'_>) -> ReadOutcome {
        let query = ViewQuery::parse_with_role(uri)?;
        let entity_id = query.scope.as_deref().unwrap_or_default();
        check_entity_id(entity_id)?;
        let Some(role) = query.role.as_deref() else {
            return exported(view, &query, Format::Context);
        };
        let request = ContextRequest {
            route: role,
            subject: entity_id,
            max_tokens: query.max_tokens,
            format: Format::Context,
        };
        context(&view, &request)
            .map(|pack| ResourceText::new("application/json", pack.text))
            .map_err(|refusal| Box::new(McpError::refused(refusal, ViewQuery::context_argument)))
    }
}
