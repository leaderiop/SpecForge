//! What the readers find: one [`Record`] per entity to create, each with the `.planning` line it comes from.

use std::collections::BTreeMap;
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
    pub source: Source,
}

/// Every record, keyed by kind then id, so the plan's order does not depend on the order files are read in.
#[derive(Debug, Default)]
pub struct Records(BTreeMap<(&'static str, String), Record>);

impl Records {
    /// Adds `record`; a second record of the same kind and id is not added, and `report` names both lines.
    pub fn add(&mut self, record: Record, report: &mut Report) {
        let key = (record.kind, record.id.clone());
        match self.0.get(&key) {
            Some(first) => report.push(Entry::at(
                &record.source,
                record.kind,
                Outcome::Unmapped,
                format!(
                    "{} {} is given twice (first at {}): not imported",
                    record.kind, record.id, first.source
                ),
            )),
            None => {
                self.0.insert(key, record);
            }
        }
    }

    /// The records in kind then id order.
    pub fn iter(&self) -> impl Iterator<Item = &Record> {
        self.0.values()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// How many records of each kind.
    pub fn counts(&self) -> BTreeMap<&'static str, usize> {
        let mut counts = BTreeMap::new();
        for (kind, _) in self.0.keys() {
            *counts.entry(*kind).or_default() += 1;
        }
        counts
    }
}
