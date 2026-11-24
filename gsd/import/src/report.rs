//! The import's report (ADR 0391 D8): one entry per file, section or row, with its outcome and the reason, sorted, with
//! no clock and no absolute path.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::records::Source;

/// What became of a file, a section or a row: the ten classes of ADR 0391 D3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Outcome {
    /// Written to an entity's field.
    Mapped,
    /// A field `@gsd/core` computes: never written.
    Computed,
    /// Another file already gives it.
    Redundant,
    /// gsd-tools' runtime state, which the graph computes.
    Runtime,
    /// Nothing to import.
    Ignored,
    /// No kind holds it; the reason says why.
    Unmapped,
    /// Its kinds are an extension's the project does not load.
    NeedsExtension,
    /// A setting: the report prints the line that sets it.
    Setting,
    /// No `.planning` source: the user writes it after the import.
    AuthoredAfterImport,
    /// No `.planning` source, so the import filled it (a minted id).
    Filled,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Mapped => "mapped",
            Outcome::Computed => "computed",
            Outcome::Redundant => "redundant",
            Outcome::Runtime => "runtime",
            Outcome::Ignored => "ignored",
            Outcome::Unmapped => "unmapped",
            Outcome::NeedsExtension => "needs-extension",
            Outcome::Setting => "setting",
            Outcome::AuthoredAfterImport => "authored-after-import",
            Outcome::Filled => "filled",
        }
    }
}

/// One line of the report. Fields in sort order: file, line (none, the whole file, first), section.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entry {
    pub file: String,
    /// 1-based; none when the entry is about the whole file.
    pub line: Option<usize>,
    pub section: String,
    pub outcome: Outcome,
    pub reason: String,
}

impl Entry {
    /// An entry about the line `source`, in `section`.
    pub fn at(source: &Source, section: &str, outcome: Outcome, reason: String) -> Self {
        Entry {
            file: source.file.clone(),
            line: Some(source.line),
            section: section.to_string(),
            outcome,
            reason,
        }
    }

    /// An entry about the whole file `file`.
    pub fn file(file: &str, outcome: Outcome, reason: String) -> Self {
        Entry {
            file: file.to_string(),
            line: None,
            section: String::new(),
            outcome,
            reason,
        }
    }

    /// `file:line` or `file`.
    pub fn location(&self) -> String {
        match self.line {
            Some(line) => format!("{}:{line}", self.file),
            None => self.file.clone(),
        }
    }
}

/// Every entry of one import.
#[derive(Debug, Default)]
pub struct Report {
    entries: Vec<Entry>,
}

impl Report {
    pub fn push(&mut self, entry: Entry) {
        self.entries.push(entry);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The entries, sorted by file, line and section.
    pub fn entries(&self) -> Vec<&Entry> {
        let mut entries: Vec<&Entry> = self.entries.iter().collect();
        entries.sort();
        entries
    }

    /// `{"format": 1, "entries": [..], "counts": {kind: n}}`.
    pub fn payload(&self, counts: &BTreeMap<&'static str, usize>) -> Value {
        let entries: Vec<Value> = self
            .entries()
            .into_iter()
            .map(|e| {
                json!({
                    "file": e.file,
                    "line": e.line,
                    "section": e.section,
                    "outcome": e.outcome.as_str(),
                    "reason": e.reason,
                })
            })
            .collect();
        json!({ "format": 1, "entries": entries, "counts": counts })
    }

    /// One line per entry, then the count line.
    pub fn text(&self, counts: &BTreeMap<&'static str, usize>) -> String {
        let mut out = String::new();
        for e in self.entries() {
            let section = match e.section.as_str() {
                "" => String::new(),
                section => format!(" [{section}]"),
            };
            out.push_str(&format!(
                "{} {}{section}: {}\n",
                e.location(),
                e.outcome.as_str(),
                e.reason
            ));
        }
        let total: usize = counts.values().sum();
        let kinds: Vec<String> = counts.iter().map(|(k, n)| format!("{k} {n}")).collect();
        out.push_str(&format!(
            "import: {total} entities ({}), {} report entries\n",
            kinds.join(", "),
            self.entries.len()
        ));
        out
    }
}
