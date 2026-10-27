//! What a plan may not write (ADR 0289): checked per operation before anything is planned, so the refusal names the
//! operation and not a later E080. A computed value is never written; a harness value only by the extension that
//! declares the field or one that names it a peer; `verdict` is no field.

use super::{COMPUTED_FIELD, HARNESS_FIELD, Op, Origin, invalid};
use crate::OpError;
use crate::OpErrorKind;
use crate::authoring::edit::{Edit, Step};
use crate::view::ProjectView;
use specforge_graph::Found;
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
            Step::Rename { entity, to } => {
                if let Some(kind) = kinds.of(entity) {
                    kinds.made.insert(to.clone(), kind);
                }
            }
        }
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
            for field in fields.keys() {
                write(view, origin, number, plan, kind, field)?;
            }
            kinds.made.insert(id.clone(), kind.clone());
            Ok(())
        }
        Edit::Set {
            entity_id, field, ..
        }
        | Edit::Unset {
            entity_id, field, ..
        }
        | Edit::Append {
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
            Some(kind) => write(view, origin, number, plan, &kind, field),
            None => Ok(()),
        },
        Edit::Retitle { .. }
        | Edit::AddVerify { .. }
        | Edit::RemoveVerify { .. }
        | Edit::Delete { .. } => Ok(()),
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
                "{at}'{field}' of {kind} is computed by a pass of {} and is never written",
                entry.source_extension()
            ),
        )),
        Some(FieldWriter::Harness)
            if !may_write_harness(view, origin, entry.source_extension()) =>
        {
            Err(refuse(
                HARNESS_FIELD,
                number,
                plan,
                format!(
                    "{at}'{field}' of {kind} is set by a command or collector of {} (or of an extension that names it a peer), recorded in specforge-harness",
                    entry.source_extension()
                ),
            ))
        }
        _ => Ok(()),
    }
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
