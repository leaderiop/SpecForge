//! Every value the phase checks: each item of each written field or member whose declared path has checks, with where
//! it is written. The one place a field's owner gate is read (ADR 0350): a stage takes the ungated values or the gated
//! ones, and a gated value its owner state holds back is collected apart, with the status of its rows.

use std::collections::BTreeMap;
use std::sync::Arc;

use specforge_common::{Diagnostic, DiagnosticData, SourceSpan};
use specforge_protocol_types::{CheckStatus, OwnerState, Root};
use specforge_registry::entity::FieldRecord;
use specforge_registry::{CheckRegistry, EntitySnapshot, ResolvedCheck};

use super::Stage;

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
    /// The stage that collected it.
    pub stage: Stage,
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

/// What one stage collects of a snapshot.
#[derive(Default)]
pub(super) struct Collected<'a> {
    /// The values its checkers read, in source order.
    pub values: Vec<Value<'a>>,
    /// The gated values their owner state holds back, each with its rows' status: `Planned` while a planned value's
    /// owner is pending, `Closed` once a while-open value's owner is complete. No checker reads them.
    pub held: Vec<Held<'a>>,
    /// How many planned values wait for their owner, by `kind.path` (I213).
    pub waiting: BTreeMap<String, usize>,
}

/// A gated value its owner state holds back.
pub(super) struct Held<'a> {
    pub value: Value<'a>,
    /// Its rows' status: `Planned` or `Closed`.
    pub status: CheckStatus,
    /// Where a planned value's path starts when a check reads it as a path: its written form is still read
    /// (ADR 0350 D5).
    pub root: Option<Root>,
}

/// Every value of `entities` that has a check and that `stage` takes, in source order: the ungated values, or the gated
/// ones, each read against its holder's owner state on this snapshot (ADR 0329 D4).
pub(super) fn values<'a>(
    entities: &'a EntitySnapshot,
    checks: &'a CheckRegistry,
    stage: Stage,
    text_of: &dyn Fn(&str) -> Option<Arc<str>>,
) -> Collected<'a> {
    let mut collected = Collected::default();
    for record in entities.records() {
        if checks.of_kind(&record.kind).next().is_none() {
            continue;
        }
        // Read once per entity, and only when it holds a gated value.
        let mut pending = None;
        for field in &record.fields {
            for written in std::iter::once(field).chain(&field.members) {
                let owed = checks.on(&record.kind, &written.declared);
                if owed.is_empty() {
                    continue;
                }
                let held = match (stage, checks.gate(&record.kind, &written.declared)) {
                    (Stage::Ungated, None) => None,
                    (Stage::Gated, Some(gate)) => {
                        match (
                            gate,
                            *pending.get_or_insert_with(|| entities.is_pending(record)),
                        ) {
                            (OwnerState::Complete, true) => Some(CheckStatus::Planned),
                            (OwnerState::Pending, false) => Some(CheckStatus::Closed),
                            _ => None,
                        }
                    }
                    _ => continue,
                };
                let Some(status) = held else {
                    values_of(
                        &record.id,
                        written,
                        owed,
                        stage,
                        text_of,
                        &mut collected.values,
                    );
                    continue;
                };
                let mut values = Vec::new();
                values_of(&record.id, written, owed, stage, text_of, &mut values);
                let mut root = None;
                if status == CheckStatus::Planned {
                    *collected
                        .waiting
                        .entry(format!("{}.{}", record.kind, written.declared))
                        .or_default() += values.len();
                    root = checks.path_root(&record.kind, &written.declared);
                }
                collected.held.extend(values.into_iter().map(|value| Held {
                    value,
                    status,
                    root: root.clone(),
                }));
            }
        }
    }
    collected
}

fn values_of<'a>(
    entity: &'a str,
    written: &'a FieldRecord,
    checks: &'a [ResolvedCheck],
    stage: Stage,
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
            stage,
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
