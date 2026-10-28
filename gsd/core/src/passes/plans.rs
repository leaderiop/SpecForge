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

fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// `\b<needle>\b` in `haystack` (JavaScript's ASCII word boundary), case-insensitively for ASCII letters when `fold`.
/// The needle starts and ends with a word character (an id, a phrase).
pub fn has_word(haystack: &str, needle: &str, fold: bool) -> bool {
    let (haystack, needle) = if fold {
        (haystack.to_ascii_lowercase(), needle.to_ascii_lowercase())
    } else {
        (haystack.to_string(), needle.to_string())
    };
    let bytes = haystack.as_bytes();
    haystack.match_indices(&needle).any(|(at, found)| {
        let before = at.checked_sub(1).map(|i| bytes[i]);
        let after = bytes.get(at + found.len()).copied();
        !before.is_some_and(is_word) && !after.is_some_and(is_word)
    })
}

/// What a JSON value holds, for [`texts`]: a string, children, or nothing to read.
enum Shape<'a, V> {
    Text(&'a str),
    Children(Vec<&'a V>),
    Nothing,
}

fn walk<'a, V>(value: &'a V, shape: &impl Fn(&'a V) -> Shape<'a, V>, out: &mut Vec<String>) {
    match shape(value) {
        Shape::Text(text) => out.push(text.to_string()),
        Shape::Children(children) => children.into_iter().for_each(|c| walk(c, shape, out)),
        Shape::Nothing => {}
    }
}

/// Every string an entity writes, field by field: the structured value of a list or block (its strings, recursively)
/// when the host sends one, the field text otherwise. The pass input carries no title, so a title is not here.
pub fn texts(entity: &PassEntity) -> Vec<String> {
    let mut out = Vec::new();
    for (name, text) in &entity.fields {
        match entity.values.get(name) {
            Some(value) => walk(
                value,
                &|v| {
                    if let Some(text) = v.as_str() {
                        Shape::Text(text)
                    } else if let Some(items) = v.as_array() {
                        Shape::Children(items.iter().collect())
                    } else if let Some(members) = v.as_object() {
                        Shape::Children(members.values().collect())
                    } else {
                        Shape::Nothing
                    }
                },
                &mut out,
            ),
            None => out.push(text.clone()),
        }
    }
    out
}
