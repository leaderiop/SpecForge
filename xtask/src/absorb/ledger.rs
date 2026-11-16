//! The absorb ledger (ADR 0360 D3): every old value the converter did not write as itself, and why. Sorted, with no
//! time and no absolute path, so two runs on one input write the same bytes.

use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;

use super::table::Pending;

/// Why a value is in the ledger.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Why {
    /// A reference, or an inverse write, onto an old entity this run does not convert.
    OutOfSelection(String),
    /// A value whose home is a kind `@gsd/core` does not hold yet.
    Later(Pending),
    /// A reference to an entity of a kind with no home yet.
    TargetLater { target: String, pending: Pending },
    /// A word written as a coarser one: the new field and what it holds.
    Collapsed { field: String, into: Value },
    /// A computed value the new kind gives that differs from the old one (`null`: none).
    Computed { field: String, new: Value },
    /// A required field the old entity had no value for, filled.
    Filled { field: String, value: Value },
    /// A value the converter could not map, with why; it stays where the row's fallback put it.
    Unconverted(String),
    /// An entity the converter created from this value, that the old project did not hold.
    Created { kind: String, id: String },
    /// An obligation no test item states: the attributes that state it (`file:line`), none when none does.
    Unproven { tests: Vec<String> },
    /// A value no `@gsd/core` field takes, with why: nothing is written for it.
    Dropped(String),
}

/// One ledger row: the old entity, the old field (`title` for its header, `verify` for an obligation), its value as
/// every output writes it, and why.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Entry {
    pub kind: String,
    pub id: String,
    pub field: String,
    pub value: Value,
    pub why: Why,
}

impl Entry {
    /// The order the ledger is written in: kind, id, field, the value's canonical JSON, then why's.
    fn key(&self) -> (String, String, String, String, String) {
        (
            self.kind.clone(),
            self.id.clone(),
            self.field.clone(),
            self.value.to_string(),
            json!(self.why).to_string(),
        )
    }
}

impl Eq for Entry {}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key().cmp(&other.key())
    }
}

/// The ledger of one run: a set, so an entry recorded twice is written once.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ledger(BTreeSet<Entry>);

impl Ledger {
    pub fn record(&mut self, kind: &str, id: &str, field: &str, value: Value, why: Why) {
        self.0.insert(Entry {
            kind: kind.to_string(),
            id: id.to_string(),
            field: field.to_string(),
            value,
            why,
        });
    }

    #[cfg(test)]
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.0.iter()
    }

    /// How many entries give each why, by its name (`later`, `computed`, …).
    pub fn counts(&self) -> std::collections::BTreeMap<String, usize> {
        let mut counts = std::collections::BTreeMap::new();
        for entry in &self.0 {
            let name = match json!(entry.why) {
                Value::Object(map) => map.keys().next().cloned().unwrap_or_default(),
                Value::String(name) => name,
                _ => String::new(),
            };
            *counts.entry(name).or_insert(0) += 1;
        }
        counts
    }

    /// Format 1: `{"format": 1, "from": <project>, "entries": [...]}`, pretty, with a final newline.
    pub fn to_json(&self, from: &str) -> String {
        #[derive(Serialize)]
        struct Document<'l> {
            format: u32,
            from: &'l str,
            entries: Vec<&'l Entry>,
        }
        let document = Document {
            format: 1,
            from,
            entries: self.0.iter().collect(),
        };
        let mut text = serde_json::to_string_pretty(&document).unwrap_or_default();
        text.push('\n');
        text
    }
}
