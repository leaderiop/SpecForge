//! The members of a typed block field (ADR 0285): each a field of the block, read as a field is.

use std::borrow::Cow;

use specforge_common::SourceSpan;
use specforge_parser::{BlockItem, FieldEntry, FieldMap, FieldValue};
use specforge_protocol_types::FieldType;

use super::{FieldForm, FieldForms, Reading};

/// A typed `block` or `block_list` field's members, in declared order, each with the form that reads its value.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BlockMembers(Vec<(String, FieldForm)>);

impl BlockMembers {
    pub fn new(members: Vec<(String, FieldForm)>) -> Self {
        Self(members)
    }

    /// The member `name`, if declared.
    pub fn get(&self, name: &str) -> Option<&FieldForm> {
        self.0
            .iter()
            .find(|(member, _)| member == name)
            .map(|(_, form)| form)
    }

    /// Every member and its form, in declared order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &FieldForm)> {
        self.0.iter().map(|(name, form)| (name.as_str(), form))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Where a value sits inside a field (ADR 0285): `requires` at the top, `requires.phase` declared, and
/// `requires[1].phase` written, the 0-based item of a `block_list`. Field and member names hold neither `.` nor `[`,
/// so the path reads back unambiguously.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemberPath(String);

impl MemberPath {
    /// The path of a top-level field.
    pub fn of(field: &str) -> Self {
        Self(field.to_string())
    }

    fn member(&self, name: &str) -> Self {
        Self(format!("{}.{name}", self.0))
    }

    fn item(&self, index: usize) -> Self {
        Self(format!("{}[{index}]", self.0))
    }

    /// The path as written, with its item indices.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The declared path: the same without its item indices, what [`FieldForms::reading_at`] takes.
    pub fn declared(&self) -> String {
        declared_path(&self.0)
    }
}

impl std::fmt::Display for MemberPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// `path` without its `[i]` indices.
pub fn declared_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut skipping = false;
    for c in path.chars() {
        match c {
            '[' => skipping = true,
            ']' => skipping = false,
            c if !skipping => out.push(c),
            _ => {}
        }
    }
    out
}

/// One typed value inside a field: where it sits, the form that reads it and the entry that writes it.
#[derive(Debug, Clone)]
pub struct Member<'f, 'e> {
    pub path: MemberPath,
    pub form: &'f FieldForm,
    pub entry: &'e FieldEntry,
}

impl FieldForms {
    /// Every typed value inside `entry`, as the graph build stores it, the entry itself first: its member path, its
    /// form and its entry. A `block` with members descends into the members it declares; a `block_list` into each
    /// item; an open `block` and an undeclared field yield nothing below (ADR 0285). A key a typed block does not
    /// declare is not yielded (W020 reports it).
    pub fn walk<'f, 'e>(
        &'f self,
        kind: &str,
        entry: &'e FieldEntry,
    ) -> impl Iterator<Item = Member<'f, 'e>> {
        let mut out = Vec::new();
        if let Reading::Declared(form) = self.reading(kind, entry.key.as_str()) {
            let path = MemberPath::of(entry.key.as_str());
            out.push(Member {
                path: path.clone(),
                form,
                entry,
            });
            descend(form, &path, &entry.value, &mut out);
        }
        out.into_iter()
    }

    /// How the declared member path `path` (`requires.phase`, no item indices) reads on `kind`: the field's form, or
    /// its member's. A path that names no declared field or member is `Undeclared`; a top-level name is
    /// [`Self::reading`].
    pub fn reading_at(&self, kind: &str, path: &str) -> Reading<'_> {
        let mut parts = path.split('.');
        let Some(first) = parts.next() else {
            return Reading::Undeclared;
        };
        let top = self.reading(kind, first);
        parts.fold(top, |reading, name| match reading {
            Reading::Declared(form) => form
                .members()
                .and_then(|members| members.get(name))
                .map_or(Reading::Undeclared, Reading::Declared),
            other => other,
        })
    }
}

fn descend<'f, 'e>(
    form: &'f FieldForm,
    path: &MemberPath,
    value: &'e FieldValue,
    out: &mut Vec<Member<'f, 'e>>,
) {
    let Some(members) = form.members() else {
        return;
    };
    let mut block = |path: &MemberPath, map: &'e FieldMap| {
        for entry in map.entries() {
            let Some(member) = members.get(entry.key.as_str()) else {
                continue;
            };
            let path = path.member(entry.key.as_str());
            out.push(Member {
                path: path.clone(),
                form: member,
                entry,
            });
            descend(member, &path, &entry.value, out);
        }
    };
    match value {
        FieldValue::Block(map) => block(path, map),
        FieldValue::Blocks(items) => {
            for (i, item) in items.iter().enumerate() {
                block(&path.item(i), &item.fields);
            }
        }
        _ => {}
    }
}

/// `value` with each member of a typed block, or of each item of a block list, stored as its form coerces it; `None`
/// when `form` types no members or nothing changes. A single block on a `block_list` is its one item and `[]` is no
/// item (ADR 0285). Idempotent.
pub(super) fn coerce_members(
    form: &FieldForm,
    value: &FieldValue,
    span: Option<&SourceSpan>,
) -> Option<FieldValue> {
    let is_list = form.field_type() == FieldType::BlockList;
    let (value, widened) = match value {
        FieldValue::Block(map) if is_list => (
            Cow::Owned(FieldValue::Blocks(vec![BlockItem {
                fields: map.clone(),
                span: span?.clone(),
            }])),
            true,
        ),
        FieldValue::List(items) if is_list && items.is_empty() => {
            return Some(FieldValue::Blocks(Vec::new()));
        }
        value => (Cow::Borrowed(value), false),
    };
    let stored = form.members().and_then(|members| match value.as_ref() {
        FieldValue::Block(map) => coerce_map(map, members).map(FieldValue::Block),
        FieldValue::Blocks(items) => {
            let mut changed = false;
            let items: Vec<BlockItem> = items
                .iter()
                .map(|item| match coerce_map(&item.fields, members) {
                    Some(fields) => {
                        changed = true;
                        BlockItem {
                            fields,
                            span: item.span.clone(),
                        }
                    }
                    None => item.clone(),
                })
                .collect();
            changed.then_some(FieldValue::Blocks(items))
        }
        _ => None,
    });
    match (stored, widened) {
        (Some(stored), _) => Some(stored),
        (None, true) => Some(value.into_owned()),
        (None, false) => None,
    }
}

fn coerce_map(map: &FieldMap, members: &BlockMembers) -> Option<FieldMap> {
    let mut out = map.clone();
    let mut changed = false;
    for entry in out.entries_mut() {
        if let Some(form) = members.get(entry.key.as_str())
            && let Some(value) = form.coerce(&entry.value, entry.value_span.as_ref())
        {
            entry.value = value;
            changed = true;
        }
    }
    changed.then_some(out)
}

/// The JSON Schema of a typed block's members: an object closed to them, with the required ones listed.
pub(super) fn object_schema(
    members: &BlockMembers,
    defs: &mut specforge_common::shape::Definitions,
) -> serde_json::Value {
    let properties: serde_json::Map<String, serde_json::Value> = members
        .iter()
        .map(|(name, form)| (name.to_string(), form.json_schema(defs)))
        .collect();
    let required: Vec<&str> = members
        .iter()
        .filter(|(_, form)| form.required())
        .map(|(name, _)| name)
        .collect();
    let mut schema = serde_json::json!({
        "type": "object",
        "properties": properties,
        "additionalProperties": false,
    });
    if !required.is_empty() {
        schema["required"] = serde_json::json!(required);
    }
    schema
}
