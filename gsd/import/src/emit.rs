//! The records to one edit plan (ADR 0391 D9). The guest refuses what the host would, and reports it, before it
//! answers: a host refusal of the plan (E080) would drop the JSON report. A record is dropped, never stubbed: an id the
//! grammar refuses, an id given twice, a reference to nothing, a file outside `spec/gsd/`, a required field with no
//! source (`kinds::REQUIRED`: a reader puts each `filled` value in the fields first), a title or field holding a
//! character no `.spec` file holds; a record naming a dropped one is reported "parent not imported". Past a cap the
//! plan is refused "truncated at N".
//!
//! A record's id is its graph path (`v1.0/01`): records, references and collisions key by it, and the create takes its
//! last segment.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;
use specforge_extension_sdk::prelude::*;

use crate::ids;
use crate::kinds::{rank, required};
use crate::limits::{MAX_ENTRIES, MAX_OPS};
use crate::records::{Record, Records};
use crate::report::{Entry, Outcome, Report, hidden};

/// What [`emit`] answers: the plan, how many entities of each kind it creates, and the records it creates.
pub type Emitted<'r> = (PlanBuilder, BTreeMap<&'static str, usize>, Vec<&'r Record>);

/// The plan built on `plan` from what of `records` passes, how many entities of each kind it creates, and the records
/// it creates.
pub fn emit<'r>(
    records: &'r Records,
    graph: &CommandGraph,
    mut plan: PlanBuilder,
    report: &mut Report,
) -> Result<Emitted<'r>, String> {
    if records.len() > MAX_OPS {
        return Err(format!(
            "truncated at {MAX_OPS}: .planning/ gives {} entities, more than one plan holds; nothing is imported",
            records.len()
        ));
    }
    let mut ordered: Vec<&Record> = records.iter().collect();
    ordered.sort_by(|a, b| (rank(a.kind), a.kind, &a.id).cmp(&(rank(b.kind), b.kind, &b.id)));
    let every: BTreeSet<&str> = ordered.iter().map(|r| r.id.as_str()).collect();
    let refuse = |record: &Record, why: String, report: &mut Report| {
        report.push(Entry::at(
            &record.source,
            record.kind,
            Outcome::Unmapped,
            format!("{} {}: {why}: not imported", record.kind, record.id),
        ));
    };
    // The ids, files, required fields and collisions: what a record is alone.
    let mut kept: Vec<&Record> = Vec::new();
    let mut taken: BTreeMap<&str, &Record> = BTreeMap::new();
    for record in ordered {
        let missing = required(record.kind)
            .iter()
            .find(|field| !record.fields.contains_key(**field));
        // A scope prefix is the path of an entity the record names, never text a document wrote (`A/B-01`).
        let scoped = record
            .id
            .rsplit_once('/')
            .is_none_or(|(scope, _)| record.refs.iter().any(|r| r == scope));
        if !(scoped
            && ids::valid(record.kind, local(&record.id))
            && record.id.split('/').all(ids::name))
        {
            refuse(record, format!("not an id a {} takes", record.kind), report);
        } else if !ids::file(&record.file) {
            refuse(
                record,
                format!("its file {} is not under spec/gsd/", record.file),
                report,
            );
        } else if let Some(field) = missing {
            refuse(record, format!("required {field} has no source"), report);
        } else if let Some(field) = unwritable(record) {
            refuse(
                record,
                format!(
                    "its {field} holds a control or bidirectional character, which no .spec file holds"
                ),
                report,
            );
        } else if let Some(first) = taken.get(record.id.as_str()) {
            refuse(
                record,
                format!(
                    "the id is {} {}'s already (at {})",
                    first.kind, first.id, first.source
                ),
                report,
            );
        } else if let Some(node) = graph.node(&record.id) {
            refuse(
                record,
                format!("the graph already holds {} {}", node.kind, node.id),
                report,
            );
        } else {
            taken.insert(&record.id, record);
            kept.push(record);
        }
    }
    // The references, until no drop leaves another record naming nothing.
    loop {
        let alive: BTreeSet<&str> = kept.iter().map(|&r| r.id.as_str()).collect();
        let before = kept.len();
        kept.retain(|record| {
            let missing = record
                .refs
                .iter()
                .find(|id| !alive.contains(id.as_str()) && graph.node(id).is_none());
            match missing {
                None => true,
                Some(id) if every.contains(id.as_str()) => {
                    refuse(record, format!("parent not imported ({})", id), report);
                    false
                }
                Some(id) => {
                    refuse(
                        record,
                        format!(
                            "it names {}, which neither the import nor the graph holds",
                            id
                        ),
                        report,
                    );
                    false
                }
            }
        });
        if kept.len() == before {
            break;
        }
    }
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for record in &kept {
        report.push(Entry::at(
            &record.source,
            record.kind,
            Outcome::Mapped,
            format!("{} {}", record.kind, record.id),
        ));
        plan.create(
            record.kind,
            local(&record.id),
            &record.title,
            &record.file,
            record.fields.clone(),
            Vec::new(),
        );
        *counts.entry(record.kind).or_default() += 1;
    }
    if report.len() > MAX_ENTRIES {
        return Err(format!(
            "truncated at {MAX_ENTRIES}: the report would hold {} entries; nothing is imported",
            report.len()
        ));
    }
    Ok((plan, counts, kept))
}

/// The title, or the first field, of `record` whose text holds a character no `.spec` file holds: a control or
/// bidirectional character but a tab, a line feed and a carriage return before one. The parser refuses such a file,
/// so the host would refuse the whole plan (`block_unreadable`).
fn unwritable(record: &Record) -> Option<&str> {
    fn bad(text: &str) -> bool {
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            let line_break = c == '\n' || (c == '\r' && chars.peek() == Some(&'\n'));
            if hidden(c) && c != '\t' && !line_break {
                return true;
            }
        }
        false
    }
    fn holds(value: &Value) -> bool {
        match value {
            Value::String(text) => bad(text),
            Value::Array(items) => items.iter().any(holds),
            Value::Object(map) => map.iter().any(|(key, value)| bad(key) || holds(value)),
            _ => false,
        }
    }
    if bad(&record.title) {
        return Some("title");
    }
    record
        .fields
        .iter()
        .find(|(_, value)| holds(value))
        .map(|(field, _)| field.as_str())
}

/// A record's own id, the last segment of its path: a scoped entity is created with it and its scope field
/// (`phase 01` with `milestone v1.0` is `v1.0/01`).
fn local(id: &str) -> &str {
    id.rsplit('/').next().unwrap_or(id)
}
