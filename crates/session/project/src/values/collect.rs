//! Every value the phase checks: each item of each written field or member whose declared path has checks, with where
//! it is written.

use std::sync::Arc;

use specforge_common::{Diagnostic, DiagnosticData, SourceSpan};
use specforge_registry::entity::FieldRecord;
use specforge_registry::{CheckRegistry, EntitySnapshot, ResolvedCheck};

/// One written value with the checks it owes.
pub(super) struct Value<'a> {
    pub entity: &'a str,
    /// The written path of the field or member (`links[1].href`): unique in the entity, as a declared path is not.
    pub field: &'a str,
    /// Its place in a list; 0 for a single value.
    pub item: usize,
    pub text: &'a str,
    pub span: Option<SourceSpan>,
    /// Where its text starts in the source, when the source holds it as written (no escape in it), so a finding's
    /// byte range inside the text maps to a span.
    pub inner: Option<(usize, usize)>,
    pub checks: &'a [ResolvedCheck],
}

impl Value<'_> {
    /// `diagnostic`, placed at `span` on this value's entity.
    pub(super) fn places(&self, diagnostic: &mut Diagnostic, span: Option<SourceSpan>) {
        diagnostic.span = span;
        diagnostic.data = Some(Box::new(DiagnosticData::Subject {
            entity: self.entity.to_string(),
        }));
    }
}

/// Every value of `entities` that has a check, in source order.
pub(super) fn values<'a>(
    entities: &'a EntitySnapshot,
    checks: &'a CheckRegistry,
    text_of: &dyn Fn(&str) -> Option<Arc<str>>,
) -> Vec<Value<'a>> {
    let mut values = Vec::new();
    for record in entities.records() {
        if checks.of_kind(&record.kind).next().is_none() {
            continue;
        }
        for field in &record.fields {
            for written in std::iter::once(field).chain(&field.members) {
                let owed = checks.on(&record.kind, &written.declared);
                if !owed.is_empty() {
                    values_of(&record.id, written, owed, text_of, &mut values);
                }
            }
        }
    }
    values
}

fn values_of<'a>(
    entity: &'a str,
    written: &'a FieldRecord,
    checks: &'a [ResolvedCheck],
    text_of: &dyn Fn(&str) -> Option<Arc<str>>,
    values: &mut Vec<Value<'a>>,
) {
    let mut push = |item: usize, text: &'a str, span: Option<&SourceSpan>| {
        values.push(Value {
            entity,
            field: &written.key,
            item,
            text,
            span: span.cloned(),
            inner: span.and_then(|span| inner(span, text, text_of)),
            checks,
        });
    };
    match &written.items {
        Some(items) => {
            for (item, text) in items.iter().enumerate() {
                let span = written.item_spans.get(item).or(written.value_span.as_ref());
                push(item, text, span);
            }
        }
        None => push(0, &written.text, written.value_span.as_ref()),
    }
}

/// The line and column `text` starts at inside `span`, when the span's source, without its quotes, is `text`.
fn inner(
    span: &SourceSpan,
    text: &str,
    text_of: &dyn Fn(&str) -> Option<Arc<str>>,
) -> Option<(usize, usize)> {
    if span.start_line != span.end_line {
        return None;
    }
    let source = text_of(span.file.as_str())?;
    let line = source.lines().nth(span.start_line.checked_sub(1)?)?;
    let raw = line.get(span.start_col.checked_sub(1)?..span.end_col.checked_sub(1)?)?;
    let unquoted = raw.strip_prefix('"').and_then(|raw| raw.strip_suffix('"'));
    match unquoted {
        Some(unquoted) if unquoted == text => Some((span.start_line, span.start_col + 1)),
        None if raw == text => Some((span.start_line, span.start_col)),
        _ => None,
    }
}
