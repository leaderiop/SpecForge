//! What an edit's field argument names (ADR 0151, amended by round 16 plan 09, ADR 0285): a field of the entity's
//! kind, `field.entry`, one entry of an open block field the kind declares, or a member path into a typed block or
//! block list (`completion.audit`, `requires[0].phase`). Every verb reads its argument here first; the kind's
//! answers stay the registry's, asked through `Site::reading_toward` (ADR 0157 D4).

use super::EditField;
use super::block::{Block, Site};
use super::write::{NOT_EDITABLE, not_editable};
use crate::{OpError, OpErrorKind, Refusal};
use specforge_graph::field_form::{Reading, is_word};
use specforge_parser::{FieldEntry, FieldValue};
use specforge_protocol_types::FieldType;

/// An edit's field argument, read against the entity's kind.
pub enum FieldPath<'p, 'r> {
    /// A field the kind declares, or a member of a kind that takes any field (never `Reading::Undeclared`).
    Field { name: &'p str, reading: Reading<'r> },
    /// `field.entry`: one entry of the open block field `field`, which the kind declares.
    Entry { field: &'p str, entry: &'p str },
    /// A member path into the typed block or block list `field`: the segments after the field.
    Member { field: &'p str, segs: Vec<Seg> },
}

/// One step of a member path after its field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seg {
    /// `.member`
    Key(String),
    /// `[i]`, base 10 without a leading zero (RFC 6901's array index).
    Index(usize),
}

/// The field a path starts with: the part before its first `.` or `[`.
pub fn top(path: &str) -> &str {
    path.split(['.', '[']).next().unwrap_or(path)
}

/// `path` as its field and the steps after it; `None` when a name is not one identifier or an index is not
/// a base-10 number without a leading zero.
fn segments(path: &str) -> Option<(&str, Vec<Seg>)> {
    let field = top(path);
    if !is_word(field) {
        return None;
    }
    let mut rest = &path[field.len()..];
    let mut segs = Vec::new();
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('.') {
            let name = top(after);
            if !is_word(name) {
                return None;
            }
            segs.push(Seg::Key(name.to_string()));
            rest = &after[name.len()..];
        } else {
            let after = rest.strip_prefix('[')?;
            let (digits, tail) = after.split_once(']')?;
            let plain = digits == "0" || (!digits.starts_with('0') && !digits.is_empty());
            if !plain || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            segs.push(Seg::Index(digits.parse().ok()?));
            rest = tail;
        }
    }
    Some((field, segs))
}

/// `path` read against `site`'s kind. `target` is a link's or unlink's value: an undeclared field is then told as the
/// other side of a relation by the target's kind (ADR 0157 D1).
///
/// Refused, before any text is read: `not_editable` when a dotted path's parts are not each one identifier;
/// `unknown_field` (with its near miss) or `other_side` when the field (the part before the dot) is not the kind's;
/// `not_editable` when a path's field is not a declared block field.
pub fn resolve<'p, 'r>(
    site: &'r Site<'_>,
    path: &'p str,
    target: Option<&str>,
) -> Result<FieldPath<'p, 'r>, Refusal<EditField>> {
    let Some((field, segs)) = segments(path) else {
        return Err(not_editable(
            EditField::Field,
            format!(
                "'{path}' is not a field or a field.entry path: each part is one identifier (a member path into a block list is `field[0].member`, the index with no leading zero)"
            ),
        ));
    };
    if segs.is_empty() {
        let reading = site.reading_toward(path, target)?;
        return Ok(FieldPath::Field {
            name: path,
            reading,
        });
    }
    match site.reading_toward(field, target)? {
        Reading::Declared(form) if form.members().is_some() => {
            Ok(FieldPath::Member { field, segs })
        }
        Reading::Declared(form) if form.field_type() == FieldType::Block => match segs.as_slice() {
            [Seg::Key(_)] => Ok(FieldPath::Entry {
                field,
                entry: &path[field.len() + 1..],
            }),
            _ => Err(not_editable(
                EditField::Field,
                format!(
                    "'{path}' is not a field or a field.entry path: '{field}' is an open block, its entries are `{field}.name`"
                ),
            )),
        },
        _ => Err(not_editable(
            EditField::Field,
            format!("'{field}' is not a block field, so it has no member in '{path}'"),
        )),
    }
}

/// The refusal of `verb` (`link`, `unlink`, `create`), which takes a whole field, given `field.entry`.
pub fn whole_field(verb: &str, field: &str, entry: &str) -> Refusal<EditField> {
    let suggestion = match verb {
        "create" => format!("give '{field}' its entries as name: text"),
        _ => format!("an entry holds one text: use set or unset on '{field}.{entry}'"),
    };
    OpError::new(
        OpErrorKind::InvalidInput,
        NOT_EDITABLE,
        format!(
            "{verb} takes a whole field; '{field}.{entry}' is an entry of the block field '{field}'"
        ),
    )
    .with_suggestion(suggestion)
    .about(EditField::Field)
}

/// The block field `field` and its entry `entry`, as the block writes them: `None` when the block does not write
/// `field`; refused when it writes `field` as something other than a block (the compile's E061).
pub fn nested<'b>(
    site: &Site<'_>,
    block: &'b Block,
    field: &str,
    entry: &str,
) -> Result<Option<(&'b FieldEntry, Option<&'b FieldEntry>)>, Refusal<EditField>> {
    let Some(outer) = block.entry(field) else {
        return Ok(None);
    };
    let FieldValue::Block(map) = &outer.value else {
        return Err(not_editable(
            EditField::Field,
            format!(
                "'{field}' of '{}' is written as a value, not a block; fix it by hand or set it whole",
                site.id
            ),
        ));
    };
    Ok(Some((outer, map.entry(entry))))
}
