//! What every plan pass reads the same way: the plans, and the phase each one is in.

use std::collections::BTreeMap;

use specforge_extension_sdk::prelude::*;

pub fn of_kind<'a>(input: &'a PassInput, kind: &'a str) -> impl Iterator<Item = &'a PassEntity> {
    input.entities.iter().filter(move |e| e.kind == kind)
}

/// The id of the phase a plan is in, through its resolved `phase` edge; `None` for a quick plan (and for a `phase`
/// that names nothing, which the host reports).
pub fn phase_of<'a>(input: &'a PassInput, plan: &str) -> Option<&'a str> {
    input
        .edges
        .iter()
        .find(|e| e.source == plan && e.label == "phase")
        .map(|e| e.target.as_str())
}

/// The last `/`-segment of an id: its name.
pub fn name_of(id: &str) -> &str {
    id.rsplit('/').next().unwrap_or(id)
}

/// The entities of `kind` scoped by `plan` (a task, a checkpoint or a must-have of it), in id order.
pub fn children<'a>(input: &'a PassInput, plan: &str, kind: &'a str) -> Vec<&'a PassEntity> {
    let mut found: Vec<&PassEntity> = of_kind(input, kind)
        .filter(|e| crate::digest::scope_of(&e.id) == plan)
        .collect();
    found.sort_by(|a, b| a.id.cmp(&b.id));
    found
}

/// The text of a JSON scalar, as the author wrote it.
fn scalar<V: std::fmt::Display>(value: &V, as_str: Option<&str>) -> String {
    as_str.map_or_else(|| value.to_string(), str::to_string)
}

/// The items of a list of strings (`values`): empty when the field is unwritten or not a list.
pub fn strings(entity: &PassEntity, field: &str) -> Vec<String> {
    entity
        .values
        .get(field)
        .and_then(|v| v.as_array())
        .map(|items| items.iter().map(|i| scalar(i, i.as_str())).collect())
        .unwrap_or_default()
}

/// The rows of a block list (`values`), each member as its text; a member that is itself a list or block is its
/// compact JSON.
pub fn rows(entity: &PassEntity, field: &str) -> Vec<BTreeMap<String, String>> {
    entity
        .values
        .get(field)
        .and_then(|v| v.as_array())
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.as_object())
                .map(|row| {
                    row.iter()
                        .map(|(name, v)| (name.clone(), scalar(v, v.as_str())))
                        .collect()
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The members of a block (`values`) as their text; empty when the field is unwritten.
pub fn block(entity: &PassEntity, field: &str) -> BTreeMap<String, String> {
    entity
        .values
        .get(field)
        .and_then(|v| v.as_object())
        .map(|members| {
            members
                .iter()
                .map(|(name, v)| (name.clone(), scalar(v, v.as_str())))
                .collect()
        })
        .unwrap_or_default()
}

/// A written, non-blank text field.
pub fn text<'a>(entity: &'a PassEntity, field: &str) -> Option<&'a str> {
    entity
        .fields
        .get(field)
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
}
