//! Typed edits of a typed block or a block list (ADR 0285): `set` and `unset` of a member path, `append` and
//! `remove`. A member path is read-modify-written: the field's value is read as a tree (each scalar keeps the text
//! it was written with), the tree is changed at the path, and the whole field is written again, so no edit patches
//! text inside a value. What the caller gives is spelled by the member's declared form.

use super::block::{Block, Site, spelled};
use super::defaults::{self, Change};
use super::path::{self, FieldPath, Seg};
use super::write::{self, items_given, not_editable};
use super::{EditField, Input};
use crate::Refusal;
use crate::view::ProjectView;
use specforge_graph::field_form::{FieldForm, Reading, spell};
use specforge_parser::{FieldEntry, FieldMap, FieldValue};
use specforge_protocol_types::FieldType;
use std::collections::BTreeMap;

/// A field's value as a block writes it.
enum Tree {
    /// A scalar or a list of words, as written.
    Raw(String),
    /// A typed block's members, in written order.
    Block(Vec<(String, Tree)>),
    /// A block list's items.
    Blocks(Vec<Vec<(String, Tree)>>),
}

fn members_text(members: &[(String, Tree)]) -> String {
    if members.is_empty() {
        return "{}".to_string();
    }
    let each: Vec<String> = members
        .iter()
        .map(|(key, tree)| format!("{key} {}", tree.text()))
        .collect();
    format!("{{ {} }}", each.join(" "))
}

impl Tree {
    fn text(&self) -> String {
        match self {
            Tree::Raw(text) => text.clone(),
            Tree::Block(members) => members_text(members),
            Tree::Blocks(items) if items.is_empty() => "[]".to_string(),
            Tree::Blocks(items) => {
                let each: Vec<String> = items.iter().map(|item| members_text(item)).collect();
                format!("[\n{}\n]", each.join("\n"))
            }
        }
    }

    fn empty(form: &FieldForm) -> Tree {
        match form.field_type() {
            FieldType::BlockList => Tree::Blocks(Vec::new()),
            _ => Tree::Block(Vec::new()),
        }
    }
}

/// The members of `map` as written; `None` when a value has no span to read its text from.
fn read_members(block: &Block, map: &FieldMap) -> Option<Vec<(String, Tree)>> {
    map.entries()
        .iter()
        .map(|entry| Some((entry.key.to_string(), read(block, entry)?)))
        .collect()
}

fn read(block: &Block, entry: &FieldEntry) -> Option<Tree> {
    Some(match &entry.value {
        FieldValue::Block(map) => Tree::Block(read_members(block, map)?),
        FieldValue::Blocks(items) => Tree::Blocks(
            items
                .iter()
                .map(|item| read_members(block, &item.fields))
                .collect::<Option<_>>()?,
        ),
        _ => Tree::Raw(block.text_of(entry.value_span.as_ref()?)),
    })
}

/// The members `input` gives a block: an entries or a block object.
fn object(input: &Input, path: &str) -> Result<BTreeMap<String, Input>, Refusal<EditField>> {
    match input {
        Input::Entries(map) => Ok(map
            .iter()
            .map(|(k, v)| (k.clone(), Input::One(v.clone())))
            .collect()),
        Input::Block(map) => Ok(map.clone()),
        _ => Err(not_editable(
            EditField::Value,
            format!("'{path}' is a block: give its members as an object"),
        )),
    }
}

/// The members of a block of `form` that `input` gives, in the order `form` declares them.
fn grow(
    site: &Site<'_>,
    path: &str,
    form: &FieldForm,
    input: &Input,
) -> Result<Vec<(String, Tree)>, Refusal<EditField>> {
    let declared = form.members().expect("a typed block");
    let mut given = object(input, path)?;
    let mut out = Vec::new();
    for (name, member) in declared.iter() {
        if let Some(value) = given.remove(name) {
            out.push((
                name.to_string(),
                member_tree(site, &format!("{path}.{name}"), member, &value)?,
            ));
        }
    }
    if let Some(unknown) = given.keys().next() {
        let names: Vec<&str> = declared.iter().map(|(n, _)| n).collect();
        return Err(not_editable(
            EditField::Value,
            format!(
                "'{unknown}' is not a member of '{path}' (members: {})",
                names.join(", ")
            ),
        ));
    }
    Ok(out)
}

/// `input` as the value of a member `path` of `form`'s type.
fn member_tree(
    site: &Site<'_>,
    path: &str,
    form: &FieldForm,
    input: &Input,
) -> Result<Tree, Refusal<EditField>> {
    let field_type = form.field_type();
    Ok(match field_type {
        FieldType::Block if form.members().is_some() => Tree::Block(grow(site, path, form, input)?),
        FieldType::BlockList => Tree::Blocks(items(site, path, form, input)?),
        FieldType::ReferenceList | FieldType::StringList | FieldType::EnumList => {
            let words = items_given(input, path, field_type)?;
            let written: Vec<String> = words
                .iter()
                .map(|w| spell::value(form, &spelled(site, path, &Reading::Declared(form), w)))
                .collect();
            Tree::Raw(write::list(&written))
        }
        FieldType::Block => {
            return Err(not_editable(
                EditField::Value,
                format!("'{path}' is an open block; it takes no typed member"),
            ));
        }
        FieldType::Integer | FieldType::Bool
            if matches!(input, Input::Number(_) | Input::Bool(_)) =>
        {
            // Written as given; the member's form validates it when the project is read back (E061 for a float).
            let (written, wants) = match input {
                Input::Number(n) => (n.to_string(), FieldType::Integer),
                Input::Bool(b) => (b.to_string(), FieldType::Bool),
                _ => unreachable!("matched above"),
            };
            if wants != field_type {
                return Err(not_editable(
                    EditField::Value,
                    format!(
                        "'{path}' is {}, not {}",
                        type_word(field_type),
                        type_word(wants)
                    ),
                ));
            }
            Tree::Raw(written)
        }
        _ => {
            let text = write::one(input, path, "value")?;
            let text = spelled(site, path, &Reading::Declared(form), text);
            Tree::Raw(spell::value(form, &text))
        }
    })
}

fn type_word(field_type: FieldType) -> &'static str {
    if field_type == FieldType::Integer {
        "an integer"
    } else {
        "a boolean"
    }
}

/// The items `input` gives a block list.
fn items(
    site: &Site<'_>,
    path: &str,
    form: &FieldForm,
    input: &Input,
) -> Result<Vec<Vec<(String, Tree)>>, Refusal<EditField>> {
    match input {
        Input::Blocks(items) => items
            .iter()
            .map(|item| grow(site, path, form, &Input::Block(item.clone())))
            .collect(),
        Input::Many(none) if none.is_empty() => Ok(Vec::new()),
        _ => Err(not_editable(
            EditField::Value,
            format!("'{path}' is a block list: give an array of objects"),
        )),
    }
}

/// `input` written as the whole value of the typed field `name` read as `reading`; `None` when the field is not
/// typed (the plain writer takes it).
pub(super) fn written(
    site: &Site<'_>,
    name: &str,
    reading: &Reading<'_>,
    input: &Input,
) -> Option<Result<String, Refusal<EditField>>> {
    let Reading::Declared(form) = reading else {
        return None;
    };
    form.members()?;
    Some(member_tree(site, name, form, input).map(|tree| tree.text()))
}

/// The refusal of an edit that does not fit the value it addresses.
fn refused(message: String) -> Refusal<EditField> {
    not_editable(EditField::Field, message)
}

/// The form at `name` among the members of `form`'s block.
fn member_of<'f>(
    form: &'f FieldForm,
    name: &str,
    path: &str,
) -> Result<&'f FieldForm, Refusal<EditField>> {
    form.members()
        .and_then(|members| members.get(name))
        .ok_or_else(|| refused(format!("'{name}' is not a member of '{path}'")))
}

enum Op<'i> {
    Set(&'i Input),
    Unset,
    Append(&'i Input),
    Remove(usize),
}

/// `op` applied at `segs` below `tree`, whose members `form` declares. Whether anything changed.
fn apply(
    site: &Site<'_>,
    path: &str,
    tree: &mut Tree,
    form: &FieldForm,
    segs: &[Seg],
    op: &Op<'_>,
) -> Result<bool, Refusal<EditField>> {
    let Some((first, rest)) = segs.split_first() else {
        let Tree::Blocks(held) = tree else {
            return Err(refused(format!(
                "'{path}' is not a block list: change a word list with link and unlink, a block with set"
            )));
        };
        return match op {
            Op::Append(input) => {
                held.push(grow(site, path, form, input)?);
                Ok(true)
            }
            Op::Remove(index) if *index < held.len() => {
                held.remove(*index);
                Ok(true)
            }
            Op::Remove(index) => Err(refused(format!(
                "'{path}' has {} items: there is none at index {index}",
                held.len()
            ))),
            Op::Set(_) | Op::Unset => Err(refused(format!("'{path}' is a list: address an item"))),
        };
    };
    match (first, tree) {
        (Seg::Index(i), Tree::Blocks(held)) => {
            let at = format!("{path}[{i}]");
            if rest.is_empty() {
                return match op {
                    Op::Set(input) if *i < held.len() => {
                        held[*i] = grow(site, &at, form, input)?;
                        Ok(true)
                    }
                    Op::Set(_) => Err(refused(format!(
                        "'{at}' is past the end ({} items): use append to add an item",
                        held.len()
                    ))),
                    Op::Unset => Err(refused(format!("'{at}' is an item: use remove to drop it"))),
                    Op::Append(_) | Op::Remove(_) => {
                        Err(refused(format!("'{at}' is an item, not a block list")))
                    }
                };
            }
            if *i >= held.len() {
                return Err(refused(format!(
                    "'{at}' is past the end ({} items): use append to add an item",
                    held.len()
                )));
            }
            let mut item = Tree::Block(std::mem::take(&mut held[*i]));
            let changed = apply(site, &at, &mut item, form, rest, op);
            if let Tree::Block(members) = item {
                held[*i] = members;
            }
            changed
        }
        (Seg::Index(_), _) => Err(refused(format!(
            "'{path}' is not a block list, so it has no items"
        ))),
        (Seg::Key(name), Tree::Block(members)) => {
            let at = format!("{path}.{name}");
            let member = member_of(form, name, path)?;
            let held = members.iter().position(|(key, _)| key == name);
            if rest.is_empty() {
                match op {
                    Op::Set(input) => {
                        let new = member_tree(site, &at, member, input)?;
                        match held {
                            Some(n) => members[n].1 = new,
                            None => {
                                members.push((name.clone(), new));
                                let order: Vec<&str> = form
                                    .members()
                                    .into_iter()
                                    .flat_map(|m| m.iter())
                                    .map(|(n, _)| n)
                                    .collect();
                                members.sort_by_key(|(key, _)| {
                                    order.iter().position(|n| n == key).unwrap_or(usize::MAX)
                                });
                            }
                        }
                        return Ok(true);
                    }
                    Op::Unset => {
                        return Ok(held.map(|n| members.remove(n)).is_some());
                    }
                    Op::Append(_) | Op::Remove(_) => {}
                }
            }
            // Below a member: it holds a block or a block list; one not written yet is empty, so an append can add
            // its first item and a set of its member can write it.
            if member.members().is_none() {
                return Err(refused(format!("'{at}' has no members")));
            }
            let n = match held {
                Some(n) => n,
                None if matches!(op, Op::Unset) || matches!(op, Op::Remove(_)) => return Ok(false),
                None => {
                    members.push((name.clone(), Tree::empty(member)));
                    members.len() - 1
                }
            };
            let changed = apply(site, &at, &mut members[n].1, member, rest, op)?;
            if !changed && held.is_none() {
                members.remove(n);
            }
            Ok(changed)
        }
        (Seg::Key(name), Tree::Blocks(_)) => Err(refused(format!(
            "'{path}' is a block list: address an item first, as '{path}[0].{name}'"
        ))),
        (Seg::Key(name), Tree::Raw(_)) => Err(refused(format!("'{path}' has no member '{name}'"))),
    }
}

/// `field` of `site`'s entity changed by `op` at `segs`: the whole field is written again.
fn change(
    site: &Site<'_>,
    text: &str,
    field: &str,
    segs: &[Seg],
    op: Op<'_>,
    annotations: Option<&[String]>,
) -> Result<String, Refusal<EditField>> {
    let Reading::Declared(form) = site.reading_toward(field, None)? else {
        return Err(refused(format!("'{field}' takes any text; use set")));
    };
    if form.members().is_none() {
        return Err(refused(format!("'{field}' is not a block or a block list")));
    }
    defaults::change(site, text, field, |block, _| {
        let unreadable = || {
            refused(format!(
                "'{field}' of '{}' holds text an edit cannot read back; edit it by hand in {}",
                site.id, site.key
            ))
        };
        let mut tree = match block.entry(field) {
            Some(entry) => read(block, entry).ok_or_else(unreadable)?,
            None => Tree::empty(form),
        };
        if !apply(site, field, &mut tree, form, segs, &op)? {
            return Ok(Change::Unchanged);
        }
        if matches!(&tree, Tree::Block(m) if m.is_empty())
            || matches!(&tree, Tree::Blocks(i) if i.is_empty())
        {
            return Ok(Change::Emptied);
        }
        block.place(site, field, &tree.text(), annotations)?;
        Ok(Change::Changed)
    })
}

/// `set` of a member path.
pub fn set(
    site: &Site<'_>,
    text: &str,
    field: &str,
    segs: &[Seg],
    input: &Input,
    annotations: Option<&[String]>,
) -> Result<String, Refusal<EditField>> {
    change(site, text, field, segs, Op::Set(input), annotations)
}

/// `unset` of a member path.
pub fn unset(
    site: &Site<'_>,
    text: &str,
    field: &str,
    segs: &[Seg],
) -> Result<String, Refusal<EditField>> {
    change(site, text, field, segs, Op::Unset, None)
}

/// A new last item `input` of the block list `path` names.
pub fn append(
    site: &Site<'_>,
    text: &str,
    path: &str,
    input: &Input,
) -> Result<String, Refusal<EditField>> {
    list_edit(site, text, path, Op::Append(input))
}

/// The item `index` of the block list `path` names, removed.
pub fn remove(
    site: &Site<'_>,
    text: &str,
    path: &str,
    index: usize,
) -> Result<String, Refusal<EditField>> {
    list_edit(site, text, path, Op::Remove(index))
}

fn list_edit(
    site: &Site<'_>,
    text: &str,
    path: &str,
    op: Op<'_>,
) -> Result<String, Refusal<EditField>> {
    match path::resolve(site, path, None)? {
        FieldPath::Member { field, segs } => change(site, text, field, &segs, op, None),
        // A whole field: a block list, or a field that is none.
        FieldPath::Field { name, reading } => match reading {
            Reading::Declared(form) if form.field_type() == FieldType::BlockList => {
                change(site, text, name, &[], op, None)
            }
            Reading::Declared(form) if form.field_type().is_list() => Err(refused(format!(
                "'{name}' is a list of words: use link and unlink, not append and remove"
            ))),
            _ => Err(refused(format!("'{name}' is not a block list"))),
        },
        FieldPath::Entry { field, entry } => Err(refused(format!(
            "'{field}.{entry}' is an entry of an open block, not a block list"
        ))),
    }
}

/// Whether a value for `path` of the entity `entity_id` is JSON: the path ends at a typed block or a block list
/// (the field, an item `f[0]` or a member), by the declaration and never by the text given (ADR 0285). False for a
/// path the project cannot resolve; the edit then refuses it with its own message.
pub fn takes_json(view: &ProjectView<'_>, entity_id: &str, path: &str) -> bool {
    let kind = match view.graph().find(entity_id) {
        specforge_graph::Found::One(found) => view.graph().node(found.as_str()),
        _ => None,
    };
    let (Some(node), Some((field, segs))) = (kind, path::segments(path)) else {
        return false;
    };
    let Reading::Declared(mut form) = view
        .registries()
        .field_forms()
        .reading(node.kind.raw.as_str(), field)
    else {
        return false;
    };
    for seg in &segs {
        match seg {
            Seg::Index(_) => {}
            Seg::Key(name) => match form.members().and_then(|members| members.get(name)) {
                Some(member) => form = member,
                None => return false,
            },
        }
    }
    form.members().is_some()
}
