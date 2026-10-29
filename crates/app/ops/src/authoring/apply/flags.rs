//! What a plan may not write (ADR 0289): checked per operation before anything is planned, so the refusal names the
//! operation and not a later E080. A computed value is never written; a harness value only by the extension that
//! declares the field or one that names it a peer; `verdict` is no field. An existing human-only entity is not edited,
//! and an entity holding a person's value is not deleted (ADR 0298).

use super::{COMPUTED_FIELD, HARNESS_FIELD, HUMAN_ONLY, Op, Origin, Surface, invalid};
use crate::OpError;
use crate::OpErrorKind;
use crate::authoring::edit::{Edit, Input, Step};
use crate::view::ProjectView;
use specforge_graph::{FieldForm, Found};
use specforge_parser::{FieldMap, FieldValue};
use specforge_protocol_types::FieldWriter;
use std::collections::BTreeMap;

/// The first part of a field path: before its first `.` or `[`.
fn top(path: &str) -> &str {
    path.split(['.', '[']).next().unwrap_or(path)
}

/// Where the kinds of entities are: as the project has them and as the plan's creates and renames leave them.
struct Kinds<'v> {
    view: &'v ProjectView<'v>,
    made: BTreeMap<String, String>,
}

impl Kinds<'_> {
    fn of(&self, id: &str) -> Option<String> {
        if let Some(kind) = self.made.get(id) {
            return Some(kind.clone());
        }
        let scopes = self.view.graph().scopes();
        match scopes.resolve(None, id, &|_| true) {
            Found::One(path) => scopes.kind(path.as_str()).map(|k| k.to_string()),
            _ => None,
        }
    }
}

/// `ops` over the project `view` reads, asked by `origin`. `plan` is the operations' numbers' source: a typed edit
/// of a core surface has none, and the refusal then names no number.
pub(crate) fn check(
    view: &ProjectView<'_>,
    origin: &Origin,
    ops: &[Op],
    plan: bool,
) -> Result<(), OpError> {
    let mut kinds = Kinds {
        view,
        made: BTreeMap::new(),
    };
    for op in ops {
        match &op.step {
            Step::Edit(edit) => edit_flags(view, origin, &mut kinds, op.number, plan, edit)?,
            // A verdict is checked by the judged rules before it is a step.
            Step::Verdict(_) => {}
            Step::Rename { entity, to } => {
                if let Some(kind) = kinds.of(entity) {
                    human_kind(view, origin, op.number, plan, &kind)?;
                    human_referrers(view, origin, op.number, plan, entity)?;
                    kinds.made.insert(to.clone(), kind);
                }
            }
        }
    }
    Ok(())
}

/// A verdict is a write to its entity and to the fields its rule binds: the same flags as any other write.
pub(crate) fn verdict(
    view: &ProjectView<'_>,
    origin: &Origin,
    step: &super::VerdictStep,
) -> Result<(), OpError> {
    let kinds = Kinds {
        view,
        made: BTreeMap::new(),
    };
    let Some(kind) = kinds.of(&step.entity) else {
        return Ok(());
    };
    human_kind(view, origin, step.number, true, &kind)?;
    for field in step.member.values.keys() {
        write(view, origin, step.number, true, &kind, field, None)?;
    }
    Ok(())
}

fn edit_flags(
    view: &ProjectView<'_>,
    origin: &Origin,
    kinds: &mut Kinds<'_>,
    number: usize,
    plan: bool,
    edit: &Edit,
) -> Result<(), OpError> {
    match edit {
        Edit::Create {
            kind, id, fields, ..
        } => {
            human_kind(view, origin, number, plan, kind)?;
            for (field, value) in fields {
                write(view, origin, number, plan, kind, field, Some(value))?;
            }
            kinds.made.insert(id.clone(), kind.clone());
            Ok(())
        }
        Edit::Set {
            entity_id,
            field,
            value,
            ..
        }
        | Edit::Append {
            entity_id,
            field,
            value,
        } => match kinds.of(entity_id) {
            Some(kind) => human_kind(view, origin, number, plan, &kind)
                .and_then(|()| write(view, origin, number, plan, &kind, field, Some(value))),
            None => Ok(()),
        },
        Edit::Unset {
            entity_id, field, ..
        }
        | Edit::Remove {
            entity_id, field, ..
        }
        | Edit::Link {
            entity_id, field, ..
        }
        | Edit::Unlink {
            entity_id, field, ..
        } => match kinds.of(entity_id) {
            Some(kind) => human_kind(view, origin, number, plan, &kind)
                .and_then(|()| write(view, origin, number, plan, &kind, field, None)),
            None => Ok(()),
        },
        Edit::Retitle { entity_id, .. }
        | Edit::AddVerify { entity_id, .. }
        | Edit::RemoveVerify { entity_id, .. }
        | Edit::Delete { entity_id, .. } => match kinds.of(entity_id) {
            Some(kind) => human_kind(view, origin, number, plan, &kind),
            None => Ok(()),
        }
        .and_then(|()| match edit {
            Edit::Delete {
                entity_id,
                unlink_references,
            } => {
                human_held(view, origin, number, plan, kinds.of(entity_id), entity_id)?;
                // The references it removes are writes to the entities that hold them.
                if *unlink_references {
                    human_referrers(view, origin, number, plan, entity_id)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }),
    }
}

/// Whether `origin` is an agent's: an extension, MCP, or the command line with nobody at a terminal. The editor's own
/// edits and a person at a terminal are not.
fn agent_facing(origin: &Origin) -> bool {
    match origin {
        Origin::Command { .. } | Origin::Collector { .. } | Origin::Hook { .. } => true,
        Origin::Core { surface } => match surface {
            Surface::Mcp => true,
            Surface::Cli { interactive } => !interactive,
            Surface::Lsp => false,
        },
    }
}

/// A write to an entity of `kind`, refused when the kind is human-only and `origin` an agent's.
fn human_kind(
    view: &ProjectView<'_>,
    origin: &Origin,
    number: usize,
    plan: bool,
    kind: &str,
) -> Result<(), OpError> {
    let human = view
        .registries()
        .kinds()
        .get(kind)
        .is_some_and(|entry| entry.is_human_only());
    if human && agent_facing(origin) {
        return Err(human_only(
            number,
            plan,
            format!("{kind} entities are written by a person"),
        ));
    }
    Ok(())
}

fn human_only(number: usize, plan: bool, what: String) -> OpError {
    let at = if plan {
        format!("operation {number}: ")
    } else {
        String::new()
    };
    let error = OpError::new(
        OpErrorKind::PermissionDenied,
        HUMAN_ONLY,
        format!("{at}{what}, in the editor or by hand, not through an agent's write"),
    )
    .with_suggestion("write it in your editor");
    if plan {
        error.with_data(serde_json::json!({"op": number}))
    } else {
        error
    }
}

/// A write of `field` (a path) of an entity of `kind`, asked by `origin`.
fn write(
    view: &ProjectView<'_>,
    origin: &Origin,
    number: usize,
    plan: bool,
    kind: &str,
    field: &str,
    value: Option<&Input>,
) -> Result<(), OpError> {
    let registries = view.registries();
    let name = top(field);
    let Some(entry) = registries.fields().get(kind, name) else {
        // `verdict` is written only by `set_verdict`: no field of a kind that declares none.
        if name == "verdict" && plan {
            return Err(invalid(
                number,
                format!("'verdict' is no field of {kind}: only set_verdict writes a verdict"),
            ));
        }
        return Ok(());
    };
    // The writer of the field, else of the innermost member the path names.
    let mut form = entry.form();
    let mut writer = form.writer();
    for key in field[name.len()..]
        .split(['.', '['])
        .filter(|s| !s.is_empty() && !s.ends_with(']'))
    {
        if let Some(member) = form.members().and_then(|m| m.get(key)) {
            form = member;
            writer = writer.or(form.writer());
        }
    }
    let source = entry.source_extension();
    enforce(view, origin, number, plan, kind, field, source, writer)?;
    match value {
        Some(value) => within(
            view, origin, number, plan, kind, field, source, form, writer, value,
        ),
        None => Ok(()),
    }
}

/// The members `value` writes inside `form` (a block or block list), each checked as a path to it would be.
#[allow(
    clippy::too_many_arguments,
    reason = "the walk carries the write's whole context"
)]
fn within(
    view: &ProjectView<'_>,
    origin: &Origin,
    number: usize,
    plan: bool,
    kind: &str,
    label: &str,
    source: &str,
    form: &FieldForm,
    inherited: Option<FieldWriter>,
    value: &Input,
) -> Result<(), OpError> {
    let Some(members) = form.members() else {
        return Ok(());
    };
    let items: Vec<Vec<(&str, Option<&Input>)>> = match value {
        Input::Entries(map) => vec![map.keys().map(|k| (k.as_str(), None)).collect()],
        Input::Block(map) => vec![map.iter().map(|(k, v)| (k.as_str(), Some(v))).collect()],
        Input::Blocks(list) => list
            .iter()
            .map(|map| map.iter().map(|(k, v)| (k.as_str(), Some(v))).collect())
            .collect(),
        _ => return Ok(()),
    };
    for item in items {
        for (key, nested) in item {
            let Some(member) = members.get(key) else {
                continue;
            };
            let label = format!("{label}.{key}");
            let writer = inherited.or(member.writer());
            enforce(view, origin, number, plan, kind, &label, source, writer)?;
            if let Some(nested) = nested {
                within(
                    view, origin, number, plan, kind, &label, source, member, writer, nested,
                )?;
            }
        }
    }
    Ok(())
}

/// A write of `field` refused by its `writer`, when `origin` may not write it.
#[allow(
    clippy::too_many_arguments,
    reason = "the refusal names the write's whole context"
)]
fn enforce(
    view: &ProjectView<'_>,
    origin: &Origin,
    number: usize,
    plan: bool,
    kind: &str,
    field: &str,
    source: &str,
    writer: Option<FieldWriter>,
) -> Result<(), OpError> {
    let at = if plan {
        format!("operation {number}: ")
    } else {
        String::new()
    };
    match writer {
        Some(FieldWriter::Computed) => Err(refuse(
            COMPUTED_FIELD,
            number,
            plan,
            format!(
                "{at}'{field}' of {kind} is computed by a pass of {source} and is never written"
            ),
        )),
        Some(FieldWriter::Harness) if !may_write_harness(view, origin, source) => Err(refuse(
            HARNESS_FIELD,
            number,
            plan,
            format!(
                "{at}'{field}' of {kind} is set by a command or collector of {source} (or of an extension that names it a peer), recorded in specforge-harness"
            ),
        )),
        Some(FieldWriter::Human) if agent_facing(origin) => Err(human_only(
            number,
            plan,
            format!("'{field}' of {kind} is written by a person"),
        )),
        _ => Ok(()),
    }
}

/// A delete of the entity `id` of `kind`, refused when it holds a value whose writer is human and `origin` is an
/// agent's: a written field whose writer is human, or a written block or block-list item with such a member. Computed
/// and harness values are not checked (an extension deletes its own records).
fn human_held(
    view: &ProjectView<'_>,
    origin: &Origin,
    number: usize,
    plan: bool,
    kind: Option<String>,
    id: &str,
) -> Result<(), OpError> {
    let Some(kind) = kind.filter(|_| agent_facing(origin)) else {
        return Ok(());
    };
    let Found::One(path) = view.graph().scopes().resolve(None, id, &|_| true) else {
        return Ok(());
    };
    let Some(node) = view.graph().node(path.as_str()) else {
        return Ok(());
    };
    let fields = view.registries().fields();
    let held = node.fields.entries().iter().find_map(|entry| {
        let form = fields.get(&kind, entry.key.as_str())?.form();
        held_in(form, form.writer(), &entry.value)
            .map(|inside| format!("{}{inside}", entry.key.as_str()))
    });
    match held {
        Some(at) => Err(human_only(
            number,
            plan,
            format!("'{at}' of {kind} '{id}' was written by a person"),
        )),
        None => Ok(()),
    }
}

/// The path inside `value` (empty for `value` itself) of a written member whose effective `writer` is human.
fn held_in(form: &FieldForm, writer: Option<FieldWriter>, value: &FieldValue) -> Option<String> {
    if writer == Some(FieldWriter::Human) {
        return Some(String::new());
    }
    let members = form.members()?;
    let maps: Vec<&FieldMap> = match value {
        FieldValue::Block(map) => vec![map],
        FieldValue::Blocks(items) => items.iter().map(|item| &item.fields).collect(),
        _ => return None,
    };
    maps.into_iter()
        .flat_map(FieldMap::entries)
        .find_map(|entry| {
            let member = members.get(entry.key.as_str())?;
            held_in(member, writer.or(member.writer()), &entry.value)
                .map(|inside| format!(".{}{inside}", entry.key.as_str()))
        })
}

/// A delete that unlinks references, or a rename, rewrites the fields of the entities that name `id`: refused when one
/// is a human-only kind or a human-only field and `origin` an agent's, so a person's entity is never changed as a side
/// effect.
fn human_referrers(
    view: &ProjectView<'_>,
    origin: &Origin,
    number: usize,
    plan: bool,
    id: &str,
) -> Result<(), OpError> {
    if !agent_facing(origin) {
        return Ok(());
    }
    let scopes = view.graph().scopes();
    let Found::One(path) = scopes.resolve(None, id, &|_| true) else {
        return Ok(());
    };
    for edge in view.graph().edges_to(path.as_str()) {
        let Some(kind) = scopes.kind(edge.source.as_str()).map(|k| k.to_string()) else {
            continue;
        };
        let human_field = view
            .registries()
            .fields()
            .get(&kind, top(edge.label.as_str()))
            .is_some_and(|entry| entry.form().writer() == Some(FieldWriter::Human));
        let human_kind = view
            .registries()
            .kinds()
            .get(&kind)
            .is_some_and(|entry| entry.is_human_only());
        if human_kind || human_field {
            return Err(human_only(
                number,
                plan,
                format!(
                    "{id} is named by '{}' of {}, a {kind} written by a person, which this change would rewrite",
                    edge.label, edge.source
                ),
            ));
        }
    }
    Ok(())
}

fn refuse(code: &'static str, number: usize, plan: bool, message: String) -> OpError {
    let error = OpError::new(OpErrorKind::InvalidInput, code, message);
    if plan {
        error.with_data(serde_json::json!({"op": number}))
    } else {
        error
    }
}

/// Whether `origin` may set a harness value of a field `declaring` declares: a command, collector or hook of that
/// extension, or of one whose handshake names it a peer dependency.
fn may_write_harness(view: &ProjectView<'_>, origin: &Origin, declaring: &str) -> bool {
    let (Origin::Command { extension }
    | Origin::Collector { extension }
    | Origin::Hook { extension }) = origin
    else {
        return false;
    };
    extension == declaring
        || view
            .registries()
            .declarations()
            .iter()
            .find(|d| d.name() == extension)
            .is_some_and(|d| {
                d.handshake
                    .peer_dependencies
                    .iter()
                    .any(|peer| peer.name == declaring)
            })
}
