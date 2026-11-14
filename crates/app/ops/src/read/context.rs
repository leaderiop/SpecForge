//! A context route served for one entity (ADR 0356): the one function behind `specforge context`, the MCP
//! `specforge.context` tool and the `specforge://context/{+entity_id}?role=` resource. Every caller gets the same
//! pack for the same request: the route's sections read from the subject, fitted to the budget in declared order, and
//! a digest over exactly what was served.

use specforge_common::Sym;
use specforge_emitter::EmitterError;
use specforge_emitter::pack;
use specforge_project::context;
use specforge_registry::{ContextRoute, PlainDisk};

use super::export::{BRIEF, CONTEXT, Format, GRAPH};
use crate::options::OptionTable;
use crate::view::ProjectView;
use crate::{OpError, OpErrorKind, Refusal};

// One code each for trace and context routes: a route no loaded extension declares, a subject of another kind than
// the route's.
pub use super::trace::{ROUTE_OF_ANOTHER_KIND, UNKNOWN_ROUTE};

/// The formats of a pack's `graph` member: `context` by default.
pub const PACK_FORMAT: OptionTable<Format> = OptionTable {
    argument: "format",
    choices: &[CONTEXT, BRIEF, GRAPH],
    default: Some(Format::Context),
};

/// What a context read asks for.
#[derive(Debug, Clone, Copy)]
pub struct ContextRequest<'a> {
    /// The qualified route: `gsd/executor`.
    pub route: &'a str,
    /// The entity the route is read for.
    pub subject: &'a str,
    /// The token budget; `None`: the route's own.
    pub max_tokens: Option<usize>,
    /// The format of the pack's `graph` member: one of [`PACK_FORMAT`].
    pub format: Format,
}

/// The field of a context request a refusal can be about (ADR 0072).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextField {
    Route,
    Subject,
    MaxTokens,
}

/// A served pack: the document every surface answers, and what it is of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pack {
    /// The qualified route.
    pub route: String,
    /// The subject, by path.
    pub subject: String,
    /// The pack's digest.
    pub digest: String,
    /// The pack document, compact JSON.
    pub text: String,
}

/// The pack of `request.route` for `request.subject` over the view's project.
///
/// Refused: a subject the graph lacks (E003, about the subject); a route no loaded extension declares
/// (`unknown_route`, about the route); a subject of another kind than the route's (`route_of_another_kind`, about the
/// subject); a budget below the subject and the core sections (E062, about `max_tokens`).
pub fn context(
    view: &ProjectView,
    request: &ContextRequest,
) -> Result<Pack, Refusal<ContextField>> {
    // The subject first: with nothing served, a read about an entity is refused as no project (ADR 0025).
    let subject = view.entity(request.subject, ContextField::Subject)?.node;
    let route = named_route(view, request.route)?;
    if subject.kind.raw.as_str() != route.subject {
        return Err(OpError::new(
            OpErrorKind::InvalidInput,
            ROUTE_OF_ANOTHER_KIND,
            format!(
                "route '{}' reads a '{}', and '{}' is a '{}'",
                route.name, route.subject, subject.id.raw, subject.kind.raw
            ),
        )
        .about(ContextField::Subject));
    }
    let budget = request.max_tokens.unwrap_or(route.budget as usize);
    seal(view, route, subject.id.raw, budget, request.format).map_err(failure)
}

/// The route of the qualified `name`: refused, listing the routes, when no loaded extension declares it.
fn named_route<'v>(
    view: &ProjectView<'v>,
    name: &str,
) -> Result<&'v ContextRoute, Refusal<ContextField>> {
    let routes = view.registries().contexts();
    routes.named(name).ok_or_else(|| {
        let known: Vec<&str> = routes.iter().map(|route| route.name.as_str()).collect();
        let available = if known.is_empty() {
            "no loaded extension declares a context route".to_string()
        } else {
            format!("available: {}", known.join(", "))
        };
        OpError::new(
            OpErrorKind::InvalidInput,
            UNKNOWN_ROUTE,
            format!("Unknown context route '{name}' ({available})"),
        )
        .about(ContextField::Route)
    })
}

/// The pack of `route` for `subject` at `budget`, fitted and sealed over the project, and the document rendered.
fn seal(
    view: &ProjectView,
    route: &ContextRoute,
    subject: Sym,
    budget: usize,
    format: Format,
) -> Result<Pack, EmitterError> {
    let schema = view.schema();
    let packed = context::pack(
        view.project(),
        &PlainDisk,
        route,
        subject.as_str(),
        budget,
        format.emit_format(),
        &schema,
    )?;
    let text = pack::render(view.graph(), &packed.plan, &packed.cut, &packed.sealed)?;
    Ok(Pack {
        route: route.name.clone(),
        subject: subject.as_str().to_string(),
        digest: packed.digest,
        text,
    })
}

/// The emitter's failure as the operation's: a budget too small is invalid input about `max_tokens` with its code
/// (E062); a serialization failure is internal.
fn failure(error: EmitterError) -> Refusal<ContextField> {
    match (&error, error.code()) {
        (EmitterError::BudgetTooSmall { .. }, Some(code)) => {
            OpError::coded(OpErrorKind::InvalidInput, code, error.to_string())
                .about(ContextField::MaxTokens)
        }
        _ => OpError::new(OpErrorKind::Internal, "context_failed", error.to_string()).into(),
    }
}
