//! The records to one edit plan (ADR 0391 D9). The guest refuses what the host would, and reports it, before it
//! answers: a host refusal of the plan (E080) would drop the JSON report. A record is dropped, never stubbed: an id the
//! grammar refuses, an id given twice, a reference to nothing, a file outside `spec/gsd/`, a required field with no
//! source (`kinds::REQUIRED`: a reader puts each `filled` value in the fields first), a title or field holding a
//! character no `.spec` file holds; a record whose scope or required reference names a dropped one is reported "parent
//! not imported", while a link (an item of a reference list, an optional reference) naming one is left out of its field
//! and reported, the record kept. Past a cap the plan is refused "truncated at N".
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
    // The links: an item naming nothing is left out of its field, the record kept (the readers' per-item rule).
    let alive: BTreeSet<&str> = kept.iter().map(|&r| r.id.as_str()).collect();
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for record in &kept {
        let mut fields = record.fields.clone();
        for (field, id) in &record.links {
            if alive.contains(id.as_str()) || graph.node(id).is_some() {
                continue;
            }
            let why = if every.contains(id.as_str()) {
                "which is not imported"
            } else {
                "which neither the import nor the graph holds"
            };
            report.push(Entry::at(
                &record.source,
                record.kind,
                Outcome::Unmapped,
                format!(
                    "{} {}: its {field} names {id}, {why}: left out",
                    record.kind, record.id
                ),
            ));
            unlink(
                &mut fields,
                field,
                id,
                required(record.kind).contains(field),
            );
        }
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
            fields,
            Vec::new(),
        );
        *counts.entry(record.kind).or_default() += 1;
    }
    if report.len() > MAX_ENTRIES {
        return Err(format!(
            "truncated at {MAX_ENTRIES}: the report would hold {} lines; nothing is imported",
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

/// `fields[field]` without what names `id`: the item `id` or an item block holding it (`requires: {phase, provides}`)
/// of a list, or the field when it is `id`. An optional list left empty goes; a required one stays written, empty, as
/// a reader writes it when it keeps no item (`requirements_completed: []`).
fn unlink(fields: &mut BTreeMap<String, Value>, field: &str, id: &str, required: bool) {
    let names = |v: &Value| match v {
        Value::String(s) => s == id,
        Value::Object(map) => map.values().any(|v| v.as_str() == Some(id)),
        _ => false,
    };
    match fields.get_mut(field) {
        Some(Value::Array(items)) => {
            items.retain(|item| !names(item));
            if items.is_empty() && !required {
                fields.remove(field);
            }
        }
        Some(value) if names(value) => {
            fields.remove(field);
        }
        _ => {}
    }
}

/// A record's own id, the last segment of its path: a scoped entity is created with it and its scope field
/// (`phase 01` with `milestone v1.0` is `v1.0/01`).
fn local(id: &str) -> &str {
    id.rsplit('/').next().unwrap_or(id)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use specforge_extension_sdk::PlanOp;

    use super::*;
    use crate::records::Source;

    /// A record of `kind` holding each of its required fields, naming `refs` and linking `links`.
    fn record(
        kind: &'static str,
        id: &str,
        refs: &[&str],
        links: &[(&'static str, &str)],
    ) -> Record {
        Record {
            kind,
            id: id.to_string(),
            title: id.to_string(),
            file: "spec/gsd/x.spec".to_string(),
            fields: required(kind)
                .iter()
                .map(|f| (f.to_string(), json!("x")))
                .collect(),
            refs: refs.iter().map(|r| r.to_string()).collect(),
            links: links.iter().map(|(f, id)| (*f, id.to_string())).collect(),
            source: Source {
                file: ".planning/x.md".to_string(),
                line: 1,
            },
        }
    }

    /// The creates of `records` on an empty graph, each id with its fields.
    fn created(
        records: &Records,
        report: &mut Report,
    ) -> BTreeMap<String, BTreeMap<String, Value>> {
        let plan = PlanBuilder::new(Default::default(), Default::default());
        let (plan, _, _) =
            emit(records, &CommandGraph::default(), plan, report).expect("not refused");
        plan.finish()
            .ops
            .into_iter()
            .filter_map(|op| match op {
                PlanOp::Create { id, fields, .. } => Some((id, fields)),
                _ => None,
            })
            .collect()
    }

    fn reasons(report: &Report) -> Vec<String> {
        report
            .entries()
            .into_iter()
            .map(|e| e.reason.clone())
            .collect()
    }

    #[test]
    fn a_dropped_list_item_is_left_out_and_its_holder_kept() {
        let mut records = Records::default();
        let mut report = Report::default();
        // A-01's text holds U+200F, which the parser refuses: it is dropped, and the phase naming it is kept.
        let mut rlm = record("requirement", "A-01", &[], &[]);
        rlm.fields.insert("text".to_string(), json!("x\u{200F}"));
        records.add(rlm, &mut report);
        records.add(record("requirement", "A-02", &[], &[]), &mut report);
        records.add(record("milestone", "v1.0", &[], &[]), &mut report);
        let mut phase = record(
            "phase",
            "v1.0/01",
            &["v1.0"],
            &[("requirements", "A-01"), ("requirements", "A-02")],
        );
        phase
            .fields
            .insert("requirements".to_string(), json!(["A-01", "A-02"]));
        records.add(phase, &mut report);
        let mut plan = record(
            "plan",
            "v1.0/01/01-01",
            &["v1.0/01"],
            &[("requirements", "A-01")],
        );
        plan.fields
            .insert("requirements".to_string(), json!(["A-01"]));
        records.add(plan, &mut report);
        let created = created(&records, &mut report);
        assert_eq!(
            created.keys().map(String::as_str).collect::<Vec<_>>(),
            ["01", "01-01", "A-02", "v1.0"]
        );
        assert_eq!(created["01"]["requirements"], json!(["A-02"]));
        // An optional list left empty goes.
        assert!(!created["01-01"].contains_key("requirements"));
        let said = reasons(&report);
        assert!(
            said.iter().any(|r| r
                == "phase v1.0/01: its requirements names A-01, which is not imported: left out"),
            "{said:#?}"
        );
        assert!(
            !said.iter().any(|r| r.contains("parent not imported")),
            "{said:#?}"
        );
    }

    #[test]
    fn a_dropped_parent_still_drops_its_children() {
        let mut records = Records::default();
        let mut report = Report::default();
        let mut milestone = record("milestone", "v1.0", &[], &[]);
        milestone.title = "x\u{202E}".to_string();
        records.add(milestone, &mut report);
        records.add(record("phase", "v1.0/01", &["v1.0"], &[]), &mut report);
        records.add(
            record("plan", "v1.0/01/01-01", &["v1.0/01"], &[]),
            &mut report,
        );
        assert!(created(&records, &mut report).is_empty());
        let said = reasons(&report);
        assert!(
            said.iter()
                .any(|r| r.contains("v1.0/01: parent not imported (v1.0)")),
            "{said:#?}"
        );
    }

    #[test]
    fn settings_lines_and_minted_ids_count_toward_the_report_cap() {
        let mut report = Report::default();
        for n in 0..MAX_ENTRIES / 2 {
            report.setting(format!("specforge settings set @gsd/core k{n} 'x'"));
        }
        for n in 0..=MAX_ENTRIES / 2 {
            report.minted(crate::report::Minted {
                kind: "requirement".to_string(),
                old: format!("row {n}"),
                new: format!("OOS-{n}"),
            });
        }
        let plan = PlanBuilder::new(Default::default(), Default::default());
        let Err(refused) = emit(
            &Records::default(),
            &CommandGraph::default(),
            plan,
            &mut report,
        ) else {
            panic!("a report past the cap is refused");
        };
        assert!(
            refused.starts_with(&format!("truncated at {MAX_ENTRIES}")),
            "{refused}"
        );
    }

    #[test]
    fn unlink_leaves_out_what_names_the_id() {
        let mut fields: BTreeMap<String, Value> = BTreeMap::from([
            ("completed".to_string(), json!(["A"])),
            ("optional".to_string(), json!(["A"])),
            ("one".to_string(), json!("A")),
            (
                "requires".to_string(),
                json!([{"phase": "A", "provides": "p"}, {"phase": "B", "provides": "q"}]),
            ),
        ]);
        unlink(&mut fields, "completed", "A", true);
        unlink(&mut fields, "optional", "A", false);
        unlink(&mut fields, "one", "A", false);
        unlink(&mut fields, "requires", "A", false);
        assert_eq!(
            Value::Object(fields.into_iter().collect()),
            json!({"completed": [], "requires": [{"phase": "B", "provides": "q"}]})
        );
    }
}
