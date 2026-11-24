//! The records to one edit plan (ADR 0391 D9). The guest refuses what the host would, and reports it, before it
//! answers: a host refusal of the plan (E080) would drop the JSON report. A record is dropped, never stubbed: an id the
//! grammar refuses, an id given twice, a reference to nothing, a file outside `spec/gsd/`; a record naming a dropped
//! one is reported "parent not imported". Past a cap the plan is refused "truncated at N".

use std::collections::{BTreeMap, BTreeSet};

use specforge_extension_sdk::prelude::*;

use crate::ids;
use crate::kinds::rank;
use crate::limits::{MAX_ENTRIES, MAX_OPS};
use crate::records::{Record, Records};
use crate::report::{Entry, Outcome, Report};

/// The plan built on `plan` from what of `records` passes, and how many entities of each kind it creates.
pub fn emit(
    records: &Records,
    graph: &CommandGraph,
    mut plan: PlanBuilder,
    report: &mut Report,
) -> Result<(PlanBuilder, BTreeMap<&'static str, usize>), String> {
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
            format!(
                "{} {}: {why}: not imported",
                record.kind,
                record.id.escape_debug()
            ),
        ));
    };
    // The ids, files and collisions: what a record is alone.
    let mut kept: Vec<&Record> = Vec::new();
    let mut taken: BTreeMap<&str, &Record> = BTreeMap::new();
    for record in ordered {
        if !ids::valid(record.kind, &record.id) {
            refuse(record, format!("not an id a {} takes", record.kind), report);
        } else if !ids::file(&record.file) {
            refuse(
                record,
                format!(
                    "its file {} is not under spec/gsd/",
                    record.file.escape_debug()
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
                    refuse(
                        record,
                        format!("parent not imported ({})", id.escape_debug()),
                        report,
                    );
                    false
                }
                Some(id) => {
                    refuse(
                        record,
                        format!(
                            "it names {}, which neither the import nor the graph holds",
                            id.escape_debug()
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
            &record.id,
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
    Ok((plan, counts))
}
