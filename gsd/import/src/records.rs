//! What the readers find: one [`Record`] per entity to create, each with the `.planning` line it comes from.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde_json::Value;

use crate::report::{Entry, Outcome, Report};

/// Where a record or an entry comes from: a served `.planning/` path and its 1-based line.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Source {
    pub file: String,
    pub line: usize,
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.file, self.line)
    }
}

/// One entity the plan creates.
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    pub kind: &'static str,
    /// The id path the host's create takes (`TASK-01`).
    pub id: String,
    pub title: String,
    /// The `spec/gsd/` file it is written to.
    pub file: String,
    pub fields: BTreeMap<String, Value>,
    /// The ids its reference fields name (its phase, its plan): each must be created by the plan or be in the graph.
    pub refs: Vec<String>,
    pub source: Source,
}

/// Every record, keyed by kind then id, so the plan's order does not depend on the order files are read in.
#[derive(Debug, Default)]
pub struct Records {
    kept: BTreeMap<(&'static str, String), Record>,
    /// `(served file, id)` of each record not added because another file gave its id first, and of each record of
    /// that file dropped under it: a record of the same file naming one of them is its child, never the first's.
    dropped: BTreeSet<(String, String)>,
}

impl Records {
    /// Adds `record`. A second record of the same kind and id is not added, and `report` names both lines; when the
    /// first came from another file, the second's children in its file (read before or after it) are not added either,
    /// each reported "parent not imported", so no child of one file is grafted onto another file's record.
    pub fn add(&mut self, record: Record, report: &mut Report) {
        let file = &record.source.file;
        if let Some(parent) = record
            .refs
            .iter()
            .find(|r| self.dropped.contains(&(file.clone(), (*r).clone())))
        {
            report.push(orphan(&record, parent));
            self.dropped.insert((file.clone(), record.id));
            return;
        }
        let key = (record.kind, record.id.clone());
        let Some(first) = self.kept.get(&key) else {
            self.kept.insert(key, record);
            return;
        };
        report.push(Entry::at(
            &record.source,
            record.kind,
            Outcome::Unmapped,
            format!(
                "{} {} is given twice (first at {}): not imported",
                record.kind, record.id, first.source
            ),
        ));
        if first.source.file != *file {
            self.drop_children(file.clone(), record.id, report);
        }
    }

    /// Drops, reported, every record of `file` naming `id` and, in turn, every record of `file` naming one dropped.
    fn drop_children(&mut self, file: String, id: String, report: &mut Report) {
        let mut parents = vec![id];
        while let Some(parent) = parents.pop() {
            let children: Vec<_> = self
                .kept
                .iter()
                .filter(|(_, r)| r.source.file == file && r.refs.contains(&parent))
                .map(|(key, _)| key.clone())
                .collect();
            for key in children {
                if let Some(child) = self.kept.remove(&key) {
                    report.push(orphan(&child, &parent));
                    parents.push(child.id);
                }
            }
            self.dropped.insert((file.clone(), parent));
        }
    }

    /// Whether a record `kind id` was added.
    pub fn contains(&self, kind: &'static str, id: &str) -> bool {
        self.kept.contains_key(&(kind, id.to_string()))
    }

    /// The record `kind id`, if one was added.
    pub fn get(&self, kind: &'static str, id: &str) -> Option<&Record> {
        self.kept.get(&(kind, id.to_string()))
    }

    /// The record `kind id`, to enrich (a SECURITY.md row sets its plan threat's status).
    pub fn get_mut(&mut self, kind: &'static str, id: &str) -> Option<&mut Record> {
        self.kept.get_mut(&(kind, id.to_string()))
    }

    /// The records in kind then id order.
    pub fn iter(&self) -> impl Iterator<Item = &Record> {
        self.kept.values()
    }

    pub fn len(&self) -> usize {
        self.kept.len()
    }

    pub fn is_empty(&self) -> bool {
        self.kept.is_empty()
    }
}

/// The entry of `record`, not added because its file's `parent` was not.
fn orphan(record: &Record, parent: &str) -> Entry {
    Entry::at(
        &record.source,
        record.kind,
        Outcome::Unmapped,
        format!(
            "{} {}: parent not imported ({parent}, given twice): not imported",
            record.kind, record.id
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(kind: &'static str, id: &str, refs: &[&str], (file, line): (&str, usize)) -> Record {
        Record {
            kind,
            id: id.to_string(),
            title: id.to_string(),
            file: "spec/gsd/x.spec".to_string(),
            fields: BTreeMap::new(),
            refs: refs.iter().map(|r| r.to_string()).collect(),
            source: Source {
                file: file.to_string(),
                line,
            },
        }
    }

    fn ids(records: &Records) -> Vec<(&str, &str)> {
        records
            .iter()
            .map(|r| (r.id.as_str(), r.source.file.as_str()))
            .collect()
    }

    fn says(report: &Report, file: &str, line: usize) -> String {
        report
            .entries()
            .into_iter()
            .filter(|e| e.file == file && e.line == Some(line))
            .map(|e| e.reason.clone())
            .collect()
    }

    #[test]
    fn a_duplicate_plans_children_are_its_own_never_the_firsts() {
        let (a, b) = ("a/01-01-PLAN.md", "b/01-01-PLAN.md");
        let mut records = Records::default();
        let mut report = Report::default();
        records.add(record("plan", "P", &[], (a, 1)), &mut report);
        records.add(record("task", "P/T1", &["P"], (a, 2)), &mut report);
        // The second file's plan, then its tasks (one id the first has, one it lacks) and a truth under a task.
        records.add(record("plan", "P", &[], (b, 1)), &mut report);
        records.add(record("task", "P/T1", &["P"], (b, 2)), &mut report);
        records.add(record("task", "P/T5", &["P"], (b, 3)), &mut report);
        records.add(record("truth", "P/T5/X", &["P/T5"], (b, 4)), &mut report);
        assert_eq!(ids(&records), [("P", a), ("P/T1", a)]);
        assert!(says(&report, b, 1).contains("plan P is given twice (first at a/01-01-PLAN.md:1)"));
        for line in [2, 3] {
            let said = says(&report, b, line);
            assert!(
                said.contains("parent not imported (P, given twice)"),
                "{line}: {said}"
            );
        }
        assert!(says(&report, b, 4).contains("parent not imported (P/T5, given twice)"));
    }

    #[test]
    fn children_read_before_a_duplicate_are_dropped_with_it() {
        let (a, b) = ("a/01-RESEARCH.md", "b/01-RESEARCH.md");
        let mut records = Records::default();
        let mut report = Report::default();
        records.add(record("research", "R", &[], (a, 1)), &mut report);
        // A research's items are read before the research itself.
        records.add(record("finding", "R/F1", &["R"], (b, 2)), &mut report);
        records.add(record("source", "R/S1/X", &["R/F1"], (b, 3)), &mut report);
        records.add(record("research", "R", &[], (b, 1)), &mut report);
        assert_eq!(ids(&records), [("R", a)]);
        assert!(says(&report, b, 2).contains("parent not imported (R, given twice)"));
        assert!(says(&report, b, 3).contains("parent not imported (R/F1, given twice)"));
    }

    #[test]
    fn a_duplicate_in_the_first_records_own_file_drops_only_itself() {
        let f = "ROADMAP.md";
        let mut records = Records::default();
        let mut report = Report::default();
        records.add(record("milestone", "v1", &[], (f, 1)), &mut report);
        records.add(record("milestone", "v1", &[], (f, 2)), &mut report);
        records.add(record("phase", "v1/01", &["v1"], (f, 3)), &mut report);
        assert_eq!(ids(&records), [("v1", f), ("v1/01", f)]);
    }
}
