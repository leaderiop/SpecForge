//! Lowering an extension's edit plan (ADR 0289): each operation becomes a typed [`Edit`] or a rename, with the
//! revision of the file it was planned on kept aside. A malformed operation is `invalid_plan`, naming its number
//! (from 1).

use super::{Op, invalid};
use crate::OpError;
use crate::authoring::edit::{Edit, Input, Obligation, Step};
use crate::view::ProjectView;
use serde_json::Value;
use specforge_graph::Found;
use specforge_protocol_types::{EditPlan, PlanOp};
use std::collections::BTreeMap;

/// The plan, lowered: its operations in order, the revision it names for each file, and the numbers of the
/// `set_verdict` operations (the judged-rules port decides those, T13).
pub(super) struct Lowered {
    pub ops: Vec<Op>,
    pub files: BTreeMap<String, String>,
    pub verdicts: Vec<usize>,
}

/// Where the entities of the plan are: the file of each, as the project has them and as the plan's creates and
/// renames leave them.
struct Places<'v> {
    view: &'v ProjectView<'v>,
    made: BTreeMap<String, String>,
}

impl Places<'_> {
    fn file_of(&self, id: &str) -> Option<String> {
        if let Some(file) = self.made.get(id) {
            return Some(file.clone());
        }
        match self.view.graph().scopes().resolve(None, id, &|_| true) {
            Found::One(path) => self
                .view
                .graph()
                .node(path.as_str())
                .map(|node| node.source_span.file.to_string()),
            _ => None,
        }
    }
}

/// Lower `plan`, over the project `view` reads.
pub(super) fn lower(view: &ProjectView<'_>, plan: &EditPlan) -> Result<Lowered, OpError> {
    let mut places = Places {
        view,
        made: BTreeMap::new(),
    };
    let mut out = Lowered {
        ops: Vec::new(),
        files: BTreeMap::new(),
        verdicts: Vec::new(),
    };
    for (n, op) in plan.ops.iter().enumerate() {
        let number = n + 1;
        let step = lower_op(view, &mut places, &mut out, number, op)?;
        if let Some(step) = step {
            out.ops.push(Op { number, step });
        }
    }
    Ok(out)
}

/// The revision `op` names for `file`, kept; two different ones for a file are refused.
fn note(out: &mut Lowered, number: usize, file: &str, revision: &str) -> Result<(), OpError> {
    match out.files.get(file) {
        Some(known) if known != revision => Err(invalid(
            number,
            format!(
                "it names the revision {revision} of {file}, which an earlier operation names as {known}"
            ),
        )),
        _ => {
            out.files.insert(file.to_string(), revision.to_string());
            Ok(())
        }
    }
}

/// An operation on `entity` with `revision`: the revision is noted for the entity's file. An entity the plan
/// made needs none; one nobody knows is refused later, as `not_found`.
fn on_entity(
    places: &Places<'_>,
    out: &mut Lowered,
    number: usize,
    entity: &str,
    revision: &str,
) -> Result<(), OpError> {
    let Some(file) = places.file_of(entity) else {
        return Ok(());
    };
    if places.made.contains_key(entity) {
        return Ok(());
    }
    if revision.is_empty() {
        return Err(invalid(
            number,
            format!("it names no revision of {file}, the file of {entity}"),
        ));
    }
    note(out, number, &file, revision)
}

fn input(number: usize, what: &str, value: &Value) -> Result<Input, OpError> {
    serde_json::from_value(value.clone()).map_err(|_| {
        invalid(
            number,
            format!("the value of {what} is not a text, number, flag, list or block"),
        )
    })
}

fn lower_op(
    view: &ProjectView<'_>,
    places: &mut Places<'_>,
    out: &mut Lowered,
    number: usize,
    op: &PlanOp,
) -> Result<Option<Step>, OpError> {
    let edit = |edit| Ok(Some(Step::Edit(edit)));
    match op {
        PlanOp::Create {
            kind,
            id,
            title,
            file,
            revision,
            fields,
            verify,
        } => {
            let bad = std::path::Path::new(file).is_absolute()
                || file.split('/').any(|part| part == ".." || part.is_empty())
                || !file.ends_with(".spec")
                || !view.env().sources().holds_key(file);
            if bad {
                return Err(invalid(
                    number,
                    format!("'{file}' is not a .spec file inside the project's spec root"),
                ));
            }
            if revision.is_empty() {
                return Err(invalid(number, format!("it names no revision of {file}")));
            }
            note(out, number, file, revision)?;
            let mut inputs = BTreeMap::new();
            for (name, value) in fields {
                inputs.insert(name.clone(), input(number, name, value)?);
            }
            places.made.insert(id.clone(), file.clone());
            edit(Edit::Create {
                kind: kind.clone(),
                id: id.clone(),
                title: title.clone(),
                file: file.clone(),
                fields: inputs,
                verify: verify
                    .iter()
                    .map(|o| Obligation {
                        kind: o.kind.clone(),
                        text: o.text.clone(),
                    })
                    .collect(),
            })
        }
        PlanOp::Set {
            entity,
            field,
            value,
            revision,
        } => {
            on_entity(places, out, number, entity, revision)?;
            edit(Edit::Set {
                entity_id: entity.clone(),
                field: field.clone(),
                value: input(number, field, value)?,
                annotations: None,
            })
        }
        PlanOp::Unset {
            entity,
            field,
            revision,
        } => {
            on_entity(places, out, number, entity, revision)?;
            edit(Edit::Unset {
                entity_id: entity.clone(),
                field: field.clone(),
            })
        }
        PlanOp::Append {
            entity,
            field,
            value,
            revision,
        } => {
            on_entity(places, out, number, entity, revision)?;
            edit(Edit::Append {
                entity_id: entity.clone(),
                field: field.clone(),
                value: input(number, field, value)?,
            })
        }
        PlanOp::Remove {
            entity,
            field,
            index,
            revision,
        } => {
            on_entity(places, out, number, entity, revision)?;
            edit(Edit::Remove {
                entity_id: entity.clone(),
                field: field.clone(),
                index: *index,
            })
        }
        PlanOp::Link {
            entity,
            field,
            target,
            revision,
        } => {
            on_entity(places, out, number, entity, revision)?;
            edit(Edit::Link {
                entity_id: entity.clone(),
                field: field.clone(),
                target: target.clone(),
            })
        }
        PlanOp::Unlink {
            entity,
            field,
            target,
            revision,
        } => {
            on_entity(places, out, number, entity, revision)?;
            edit(Edit::Unlink {
                entity_id: entity.clone(),
                field: field.clone(),
                target: target.clone(),
            })
        }
        PlanOp::Retitle {
            entity,
            title,
            revision,
        } => {
            on_entity(places, out, number, entity, revision)?;
            edit(Edit::Retitle {
                entity_id: entity.clone(),
                title: title.clone(),
            })
        }
        PlanOp::AddVerify {
            entity,
            kind,
            text,
            revision,
        } => {
            on_entity(places, out, number, entity, revision)?;
            edit(Edit::AddVerify {
                entity_id: entity.clone(),
                kind: kind.clone(),
                text: text.clone(),
            })
        }
        PlanOp::RemoveVerify {
            entity,
            kind,
            text,
            revision,
        } => {
            on_entity(places, out, number, entity, revision)?;
            edit(Edit::RemoveVerify {
                entity_id: entity.clone(),
                kind: kind.clone(),
                text: text.clone(),
            })
        }
        PlanOp::Delete {
            entity,
            revision,
            unlink_references,
        } => {
            on_entity(places, out, number, entity, revision)?;
            edit(Edit::Delete {
                entity_id: entity.clone(),
                unlink_references: *unlink_references,
            })
        }
        PlanOp::Rename {
            entity,
            to,
            revision,
        } => {
            on_entity(places, out, number, entity, revision)?;
            if let Some(file) = places.file_of(entity) {
                places.made.insert(to.clone(), file);
            }
            Ok(Some(Step::Rename {
                entity: entity.clone(),
                to: to.clone(),
            }))
        }
        PlanOp::SetVerdict { .. } => {
            out.verdicts.push(number);
            Ok(None)
        }
    }
}
