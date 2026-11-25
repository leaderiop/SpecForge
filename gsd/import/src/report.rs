//! The import's report (ADR 0391 D8): one entry per file, section or row, with its outcome and the reason, then the
//! settings lines, the owed verdicts, what is authored after the import and the minted ids. Sorted, with no clock and
//! no absolute path, so one input gives the same bytes whatever the order it was read in. Every echoed string is
//! escaped once, here, as it is rendered; the plan's field values keep the source text verbatim.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::owed::{self, Owed};
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

/// The most characters of one echoed value the report shows.
pub const SHOWN_MAX: usize = 200;

/// Whether `c` is a character no `.spec` file holds: a C0 control (tab and line feed too), DEL, a C1 control, the line
/// and paragraph separators, or a Unicode bidi control, which a terminal acts on or reorders. Exactly the parser's
/// `is_unprintable`, so a record holding one is dropped (`emit::unwritable`) as the host would refuse its file.
pub fn hidden(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{061C}'
                | '\u{200E}'
                | '\u{200F}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202A}'..='\u{202E}'
                | '\u{2066}'..='\u{2069}'
        )
}

/// Whether `c` is written `\u{..}` when echoed (ADR 0391 D8): a [`hidden`] character or any other Unicode format
/// control (General Category Cf): the soft hyphen, the zero-width characters, the word joiner and invisible
/// operators, U+FEFF, the interlinear annotation marks, the tag characters (U+E0001, U+E0020-U+E007F) and the rest.
/// A `.spec` file may hold these, so they are escaped, never dropped: text can carry no invisible content into the
/// report, which an agent may read.
pub fn invisible(c: char) -> bool {
    hidden(c)
        || matches!(
            c,
            '\u{00AD}'
                | '\u{0600}'..='\u{0605}'
                | '\u{06DD}'
                | '\u{070F}'
                | '\u{0890}'..='\u{0891}'
                | '\u{08E2}'
                | '\u{180E}'
                | '\u{200B}'..='\u{200D}'
                | '\u{2060}'..='\u{2064}'
                | '\u{206A}'..='\u{206F}'
                | '\u{FEFF}'
                | '\u{FFF9}'..='\u{FFFB}'
                | '\u{110BD}'
                | '\u{110CD}'
                | '\u{13430}'..='\u{1343F}'
                | '\u{1BCA0}'..='\u{1BCA3}'
                | '\u{1D173}'..='\u{1D17A}'
                | '\u{E0001}'
                | '\u{E0020}'..='\u{E007F}'
        )
}

/// `text` with every [`invisible`] character written `\u{<hex>}`; any other text unchanged.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if invisible(c) {
            out.push_str(&format!("\\u{{{:x}}}", c as u32));
        } else {
            out.push(c);
        }
    }
    out
}

/// `text` escaped and capped at [`SHOWN_MAX`] characters, a trailing `…` marking the cut.
pub fn shown(text: &str) -> String {
    match text.char_indices().nth(SHOWN_MAX) {
        Some((at, _)) => format!("{}…", escape(&text[..at])),
        None => escape(text),
    }
}

/// One line of the report. Fields in sort order: file, line (none, the whole file, first), section, outcome, reason.
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

    /// `file:line` or `file`, shown.
    pub fn location(&self) -> String {
        match self.line {
            Some(line) => format!("{}:{line}", shown(&self.file)),
            None => shown(&self.file),
        }
    }
}

/// An id the import gave an entity whose source names it otherwise (or not at all).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Minted {
    pub kind: String,
    /// What the source calls it (a row, a file name).
    pub old: String,
    pub new: String,
}

/// Everything one import reports.
#[derive(Debug, Default)]
pub struct Report {
    entries: Vec<Entry>,
    /// `specforge settings set` lines (ADR 0391 D6), each already quoted for a shell.
    settings: Vec<String>,
    /// The verdicts the imported entities owe, each listed with the commands that record it (D5).
    owed: Vec<Owed>,
    /// The fields no `.planning` source gives, which the user writes after the import.
    authored_after_import: Vec<String>,
    ids: Vec<Minted>,
}

impl Report {
    pub fn push(&mut self, entry: Entry) {
        self.entries.push(entry);
    }

    pub fn setting(&mut self, line: String) {
        self.settings.push(line);
    }

    pub fn owed(&mut self, owed: Owed) {
        self.owed.push(owed);
    }

    pub fn authored_after_import(&mut self, note: String) {
        self.authored_after_import.push(note);
    }

    pub fn minted(&mut self, minted: Minted) {
        self.ids.push(minted);
    }

    /// The number of entries: the cap `MAX_ENTRIES` counts them.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The entries, sorted by file, line, section, outcome and reason.
    pub fn entries(&self) -> Vec<&Entry> {
        sorted(&self.entries)
    }

    /// `{"format": 1, "entries", "settings", "owed", "authored_after_import", "ids", "counts"}`, every echoed string
    /// shown or escaped.
    pub fn payload(&self, counts: &BTreeMap<&'static str, usize>) -> Value {
        let entries: Vec<Value> = self
            .entries()
            .into_iter()
            .map(|e| {
                json!({
                    "file": shown(&e.file),
                    "line": e.line,
                    "section": shown(&e.section),
                    "outcome": e.outcome.as_str(),
                    "reason": shown(&e.reason),
                })
            })
            .collect();
        let ids: Vec<Value> = sorted(&self.ids)
            .into_iter()
            .map(|m| json!({"kind": m.kind, "old": shown(&m.old), "new": escape(&m.new)}))
            .collect();
        json!({
            "format": 1,
            "entries": entries,
            "settings": rendered(&self.settings),
            "owed": sorted(&self.owed).into_iter().map(Owed::json).collect::<Vec<Value>>(),
            "authored_after_import": rendered(&self.authored_after_import),
            "ids": ids,
            "counts": counts,
        })
    }

    /// One line per entry (`<file>:<line>  <section>  <outcome>  <reason>`, `-` for no section), then the settings
    /// lines, the owed verdicts and what is authored after the import, each under its heading, then the count line.
    pub fn text(&self, counts: &BTreeMap<&'static str, usize>) -> String {
        let mut out = String::new();
        for e in self.entries() {
            let section = match e.section.as_str() {
                "" => "-".to_string(),
                section => shown(section),
            };
            out.push_str(&format!(
                "{}  {section}  {}  {}\n",
                e.location(),
                e.outcome.as_str(),
                shown(&e.reason)
            ));
        }
        let mut owed: Vec<Owed> = self.owed.clone();
        owed.sort();
        // The owed lines keep their order: each phase's command, then its queries, then the count.
        let owed: Vec<String> = owed::lines(&owed).iter().map(|l| escape(l)).collect();
        for (heading, lines) in [
            (
                "settings: run each line to keep the setting",
                rendered(&self.settings),
            ),
            ("owed: record each verdict anew", owed),
            (
                "authored after import: no .planning source gives these",
                rendered(&self.authored_after_import),
            ),
        ] {
            if !lines.is_empty() {
                out.push_str(&format!("\n{heading}\n"));
                for line in lines {
                    out.push_str(&format!("  {line}\n"));
                }
            }
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

fn sorted<T: Ord>(items: &[T]) -> Vec<&T> {
    let mut out: Vec<&T> = items.iter().collect();
    out.sort();
    out
}

/// `lines` sorted, each escaped (a settings or owed line is never capped: it is meant to be run).
fn rendered(lines: &[String]) -> Vec<String> {
    sorted(lines).into_iter().map(|l| escape(l)).collect()
}

#[cfg(test)]
mod tests;
