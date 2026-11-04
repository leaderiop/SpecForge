//! The checker cache (ADR 0336): what a checker answered is a function of what it was asked, so a plan or a judgement it
//! has given is not asked again. It is held in memory on the project and on disk where the journal is written, under
//! `.specforge/effects/checks/<extension>-<origin12>.json`, read in every mode. Only probe records expire, in the
//! journal; an entry goes when its file is full, least recently used first.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use specforge_common::slug;
use specforge_effects::digest;
use specforge_protocol_types::{CheckFinding, CheckItem, CheckPhase, PlannedItem, Probe};

use crate::Environment;

/// The most entries one file keeps.
const CAP: usize = 20_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Entry {
    /// The run that last used it.
    used: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    findings: Vec<CheckFinding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    probes: Vec<Probe>,
}

/// One extension's answers at one origin version.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Book {
    run: u64,
    entries: HashMap<String, Entry>,
    #[serde(skip)]
    changed: bool,
}

impl Book {
    fn read(path: Option<&Path>) -> Self {
        let mut book: Book = path
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        book.run += 1;
        book
    }

    /// Held to [`CAP`], least recently used first.
    fn trim(&mut self) {
        let over = self.entries.len().saturating_sub(CAP);
        if over == 0 {
            return;
        }
        let mut by_use: Vec<(u64, String)> = self
            .entries
            .iter()
            .map(|(key, entry)| (entry.used, key.clone()))
            .collect();
        by_use.sort();
        for (_, key) in by_use.into_iter().take(over) {
            self.entries.remove(&key);
        }
    }
}

/// The cache of one project, shared by its later states.
#[derive(Debug, Default)]
pub(crate) struct CheckCache {
    books: Mutex<BTreeMap<String, Book>>,
}

/// The cache as one run of the checks reads it, for one extension.
pub(super) struct Shelf<'a> {
    cache: &'a CheckCache,
    name: String,
    file: Option<PathBuf>,
    /// The extension's own settings: an answer may read them.
    settings: String,
}

impl<'a> Shelf<'a> {
    pub fn of(
        env: &Environment,
        cache: &'a CheckCache,
        dir: Option<&Path>,
        extension: &str,
    ) -> Self {
        let origin = crate::effects::provenance_of(env, extension);
        let origin = origin.digest();
        let origin = origin.rsplit(':').next().unwrap_or(origin);
        let name = format!("{}-{}", slug(extension), origin.get(..12).unwrap_or(origin));
        let settings = env
            .runtime()
            .and_then(|runtime| runtime.settings_of(extension))
            .map(|settings| settings.members(false))
            .unwrap_or_default();
        Shelf {
            cache,
            file: dir.map(|dir| dir.join(format!("{name}.json"))),
            name,
            settings: digest(&settings),
        }
    }

    /// What `item` is asked, in `phase`: all it was given but its id, and of its answers what they said, not when.
    fn key(&self, phase: CheckPhase, item: &CheckItem) -> String {
        let mut asked = item.clone();
        asked.id = 0;
        for outcome in &mut asked.outcomes {
            outcome.observed_at = 0;
            outcome.from_cache = false;
        }
        digest(&(phase, &self.settings, &asked))
    }

    /// The items of `items` answered before, and those that were not, each with its key.
    pub fn split(
        &self,
        phase: CheckPhase,
        items: Vec<CheckItem>,
    ) -> (Vec<PlannedItem>, Vec<(String, CheckItem)>) {
        let mut books = self.cache.books.lock().unwrap_or_else(|e| e.into_inner());
        let book = books
            .entry(self.name.clone())
            .or_insert_with(|| Book::read(self.file.as_deref()));
        let run = book.run;
        let mut known = Vec::new();
        let mut unknown = Vec::new();
        for item in items {
            let key = self.key(phase, &item);
            match book.entries.get_mut(&key) {
                Some(entry) => {
                    book.changed |= entry.used != run;
                    entry.used = run;
                    known.push(PlannedItem {
                        id: item.id,
                        findings: entry.findings.clone(),
                        probes: entry.probes.clone(),
                    });
                }
                None => unknown.push((key, item)),
            }
        }
        (known, unknown)
    }

    /// Keep `answer` under `key`.
    pub fn keep(&self, key: String, answer: &PlannedItem) {
        let mut books = self.cache.books.lock().unwrap_or_else(|e| e.into_inner());
        let book = books.entry(self.name.clone()).or_default();
        let entry = Entry {
            used: book.run,
            findings: answer.findings.clone(),
            probes: answer.probes.clone(),
        };
        book.entries.insert(key, entry);
        book.changed = true;
    }

    /// Write what this run changed, where the journal is written. A file that cannot be written costs the cache, not
    /// the run.
    pub fn save(&self) {
        let Some(file) = &self.file else {
            return;
        };
        let mut books = self.cache.books.lock().unwrap_or_else(|e| e.into_inner());
        let Some(book) = books.get_mut(&self.name).filter(|book| book.changed) else {
            return;
        };
        book.trim();
        book.changed = false;
        if let Ok(bytes) = serde_json::to_vec(book) {
            let _ = specforge_common::fresh::replace_private(file, &bytes);
        }
    }
}

/// The directory the cache of `journal`'s project is kept in, beside its records; none for a detached journal.
pub(super) fn dir_of(journal: &specforge_effects::EffectJournal) -> Option<PathBuf> {
    Some(journal.dir()?.parent()?.join("checks"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_book_drops_what_was_used_longest_ago() {
        let mut book = Book::default();
        for used in 0..(CAP as u64 + 2) {
            book.entries.insert(
                used.to_string(),
                Entry {
                    used,
                    findings: Vec::new(),
                    probes: Vec::new(),
                },
            );
        }
        book.trim();
        assert_eq!(book.entries.len(), CAP);
        assert!(!book.entries.contains_key("0"));
        assert!(!book.entries.contains_key("1"));
        assert!(book.entries.contains_key("2"));
    }
}
