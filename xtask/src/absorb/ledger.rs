//! The absorb ledger (ADR 0360 D3): every old value the converter did not write as itself, and why, and the id map
//! (ADR 0372 D3, D8): each converted entity's old (kind, id) and its new id or path, so the ledger reads against the
//! old spec and the new one alike. Sorted, with no time and no absolute path, so two runs on one input write the same
//! bytes.

use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;

/// Why a value is in the ledger: it was computed, filled, created, left unconverted or dropped (ADR 0372 D8). No
/// value waits for a kind (ADR 0372 D2), and none is ledgered as an unproven obligation: no kept kind holds a proof
/// (ADR 0370 D8).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Why {
    /// A computed value the new kind gives that differs from the old one (`null`: none).
    Computed { field: String, new: Value },
    /// A required field the old entity had no value for, filled.
    Filled { field: String, value: Value },
    /// A value the converter could not map, with why; it stays where the row's fallback put it.
    Unconverted(String),
    /// An entity the converter created from this value, that the old project did not hold.
    Created { kind: String, id: String },
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

/// One entry of the id map: a converted entity's old kind and id, and its new id or path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Renamed {
    pub kind: String,
    pub old: String,
    pub new: String,
}

/// The ledger of one run: its entries, a set, so an entry recorded twice is written once; and the id map.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ledger {
    entries: BTreeSet<Entry>,
    ids: BTreeSet<Renamed>,
}

impl Ledger {
    pub fn record(&mut self, kind: &str, id: &str, field: &str, value: Value, why: Why) {
        self.entries.insert(Entry {
            kind: kind.to_string(),
            id: id.to_string(),
            field: field.to_string(),
            value,
            why,
        });
    }

    /// The id map's entry of one converted entity: the old (`kind`, `old`) is now `new`.
    pub fn rename(&mut self, kind: &str, old: &str, new: String) {
        self.ids.insert(Renamed {
            kind: kind.to_string(),
            old: old.to_string(),
            new,
        });
    }

    #[cfg(test)]
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter()
    }

    #[cfg(test)]
    pub fn ids(&self) -> impl Iterator<Item = &Renamed> {
        self.ids.iter()
    }

    /// How many entries give each why, by its name (`computed`, `dropped`, …).
    pub fn counts(&self) -> std::collections::BTreeMap<String, usize> {
        let mut counts = std::collections::BTreeMap::new();
        for entry in &self.entries {
            let name = match json!(entry.why) {
                Value::Object(map) => map.keys().next().cloned().unwrap_or_default(),
                Value::String(name) => name,
                _ => String::new(),
            };
            *counts.entry(name).or_insert(0) += 1;
        }
        counts
    }

    /// Format 2: `{"format": 2, "from": <project>, "entries": [...], "ids": [...]}`, pretty, with a final newline.
    /// `ids` is the id map (ADR 0372 D3, D8): one `{"kind", "old", "new"}` per converted entity, sorted by its old
    /// kind and id, `new` its new id or, for a scoped kind, its path.
    pub fn to_json(&self, from: &str) -> String {
        #[derive(Serialize)]
        struct Document<'l> {
            format: u32,
            from: &'l str,
            entries: Vec<&'l Entry>,
            ids: Vec<&'l Renamed>,
        }
        let document = Document {
            format: 2,
            from,
            entries: self.entries.iter().collect(),
            ids: self.ids.iter().collect(),
        };
        let mut text = serde_json::to_string_pretty(&document).unwrap_or_default();
        text.push('\n');
        text
    }
}
