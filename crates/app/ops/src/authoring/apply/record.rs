//! `record_context` (ADR 0356 D11): a context route's pack recorded on an entity. The host computes what it writes, so
//! no origin can forge a record. In order: the route is registered (`unknown_route`); it records the entity's kind
//! (`not_recorded`); the origin is a command, collector or hook of the route's extension or of one naming it a peer,
//! or a core surface (`origin`); the record's selector derives exactly one subject (`no_subject`,
//! `several_subjects`); the pack fits its budget (`budget`). Each is `record_refused` with that `reason`. The write is
//! a set of the context field, which the ledger records as it records a harness value.

use super::{Origin, flags, invalid};
use crate::authoring::edit::{Edit, Input, Step};
use crate::read::context::{ContextRequest, context};
use crate::read::export::Format;
use crate::view::ProjectView;
use crate::{OpError, OpErrorKind};
use serde_json::json;
use specforge_common::codes;
use specforge_graph::Found;
use specforge_project::select::Selection;
use specforge_registry::{ContextRecord, ContextRoute};
use std::collections::BTreeMap;

/// A record the host refuses: the data names the operation (in a plan) and the `reason`.
pub const RECORD_REFUSED: &str = "record_refused";

/// A `record_context` operation as the plan spelled it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordOp {
    pub number: usize,
    pub entity: String,
    pub route: String,
    pub max_tokens: Option<u32>,
}

/// `record_refused` for `reason`, about operation `number` of a plan.
pub(crate) fn refused(number: usize, reason: &str, message: impl std::fmt::Display) -> OpError {
    OpError::new(
        OpErrorKind::InvalidInput,
        RECORD_REFUSED,
        format!("operation {number}: {message}"),
    )
    .with_data(json!({"op": number, "reason": reason}))
}

/// What a record of `route` on one entity reads: the record of the entity's kind and the entity's path.
pub struct Recorded<'v> {
    pub route: &'v ContextRoute,
    pub record: &'v ContextRecord,
    /// The recorded entity, by path.
    pub entity: String,
}

/// The record `route` (qualified) keeps of `entity` (an id or a path), refused `unknown_route` or `not_recorded`;
/// an entity the project lacks is `invalid_plan`.
pub fn recorded<'v>(
    view: &ProjectView<'v>,
    number: usize,
    route: &str,
    entity: &str,
) -> Result<Recorded<'v>, OpError> {
    let Some(route) = view.registries().contexts().named(route) else {
        return Err(refused(
            number,
            "unknown_route",
            format!("no loaded extension declares the context route '{route}'"),
        ));
    };
    let node = match view.graph().find(entity) {
        Found::One(path) => view.graph().node(path.as_str()).map(|node| (path, node)),
        _ => None,
    };
    let Some((path, node)) = node else {
        return Err(invalid(
            number,
            format!("'{entity}' is no entity of the project"),
        ));
    };
    let kind = node.kind.raw.as_str();
    let Some(record) = route.records.iter().find(|record| record.kind == kind) else {
        return Err(refused(
            number,
            "not_recorded",
            format!(
                "the route '{}' records no field of a '{kind}', and '{entity}' is one",
                route.name
            ),
        ));
    };
    Ok(Recorded {
        route,
        record,
        entity: path.as_str().to_string(),
    })
}

impl Recorded<'_> {
    /// The one subject the record's selector derives from the entity, refused `no_subject` or `several_subjects`.
    pub fn subject(&self, view: &ProjectView<'_>, number: usize) -> Result<String, OpError> {
        let subjects = Selection::of(view.graph(), view.registries())
            .entities(&self.entity, &self.record.subject);
        let mut subjects = subjects.into_iter();
        match (subjects.next(), subjects.next()) {
            (Some(subject), None) => Ok(subject),
            (None, _) => Err(refused(
                number,
                "no_subject",
                format!(
                    "'{}' reaches no '{}' by '{}', so the route '{}' has no subject to read",
                    self.entity, self.route.subject, self.record.subject, self.route.name
                ),
            )),
            (Some(first), Some(second)) => Err(refused(
                number,
                "several_subjects",
                format!(
                    "'{}' reaches several '{}' by '{}' ({first}, {second}, …): a record reads one",
                    self.entity, self.route.subject, self.record.subject
                ),
            )),
        }
    }
}

/// Check `op` as D11 orders it, asked by `origin`, and give the step that writes the record.
pub(crate) fn check(
    view: &ProjectView<'_>,
    origin: &Origin,
    op: &RecordOp,
) -> Result<Step, OpError> {
    let recorded = recorded(view, op.number, &op.route, &op.entity)?;
    let route = recorded.route;
    if !matches!(origin, Origin::Core { .. })
        && !flags::may_write_harness(view, origin, &route.extension)
    {
        return Err(refused(
            op.number,
            "origin",
            format!(
                "only a command, collector or hook of {} (or of an extension that names it a peer), or a core \
                 surface, records the route '{}'",
                route.extension, route.name
            ),
        ));
    }
    let subject = recorded.subject(view, op.number)?;
    let budget = op.max_tokens.unwrap_or(route.budget);
    // shortcut: the pack is computed over the project the plan is applied to, before the plan's other operations;
    // a plan that also edits what the pack serves records the pack it read, and the next check reports it stale.
    let request = ContextRequest {
        route: &route.name,
        subject: &subject,
        max_tokens: Some(budget as usize),
        format: Format::Context,
    };
    let pack = context(view, &request).map_err(|refusal| {
        let error = refusal.into_error();
        if error.code == codes::E062.id() {
            refused(op.number, "budget", error.message)
        } else {
            error
        }
    })?;
    let value = BTreeMap::from([
        ("route".to_string(), Input::One(route.name.clone())),
        ("digest".to_string(), Input::One(pack.digest)),
        ("max_tokens".to_string(), Input::Number(budget.into())),
    ]);
    Ok(Step::Edit(Edit::Set {
        entity_id: recorded.entity,
        field: recorded.record.field.clone(),
        value: Input::Block(value),
        annotations: None,
    }))
}
