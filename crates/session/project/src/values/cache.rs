//! The checker cache (ADR 0336): what a checker answered is a function of what it was asked, so a plan or a judgement it
//! has given is not asked again. It is held in memory on the project and on disk where the journal is written, under
//! `.specforge/effects/checks/<extension>-<origin12>.json`, read in every mode. Probe records expire, in the journal; a
//! judgement over an answer that carries an expiry holds for its day; an entry goes when its file is full, least
//! recently used first.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use specforge_common::remedy::hold_printable;
use specforge_common::slug;
use specforge_effects::digest;
use specforge_protocol_types::{
    CheckFinding, CheckItem, CheckPhase, OutcomeData, PlannedItem, Probe,
};

use crate::Environment;

/// The most entries one file keeps.
const CAP: usize = 20_000;

const DAY_MS: u64 = 24 * 60 * 60 * 1000;

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
            // Written private: one that is not was not written here (a committed memo forges a pass).
            .and_then(|path| specforge_common::fresh::read_private(path).ok())
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .and_then(|mut read| {
                // Answers it gives again go where a guest's go: read printable (ADR 0244).
                hold_printable(&mut read, &[]);
                serde_json::from_value(read).ok()
            })
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

    /// What `item` is asked, in `phase`: all it was given but its id, and of its answers what they said, not when. The
    /// day an answer carrying an expiry was observed is what it said: `observed_at` is a pure guest's only clock.
    fn key(&self, phase: CheckPhase, item: &CheckItem) -> String {
        let mut asked = item.clone();
        asked.id = 0;
        for outcome in &mut asked.outcomes {
            let expires = matches!(
                outcome.data,
                OutcomeData::Tls {
                    not_after: Some(_),
                    ..
                }
            );
            outcome.observed_at = if expires {
                outcome.observed_at / DAY_MS
            } else {
                0
            };
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
    use specforge_protocol_types::{OutcomeClass, ProbeOutcome};

    use super::*;

    /// Windows sets no mode, so it cannot tell a committed book this way.
    #[cfg(unix)]
    #[test]
    fn a_book_not_private_to_this_user_is_empty() {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("book.json");
        let mut book = Book::default();
        book.entries.insert(
            "key".to_string(),
            Entry {
                used: 0,
                findings: Vec::new(),
                probes: Vec::new(),
            },
        );
        specforge_common::fresh::replace_private(&file, &serde_json::to_vec(&book).unwrap())
            .unwrap();
        assert_eq!(Book::read(Some(&file)).entries.len(), 1);
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(Book::read(Some(&file)).entries.is_empty());
    }

    /// A value judged over one answer, observed at `at`.
    fn judged_over(data: OutcomeData, at: u64) -> CheckItem {
        CheckItem {
            checker: "tls".to_string(),
            text: "a.example".to_string(),
            outcomes: vec![ProbeOutcome {
                probe: Probe::DnsResolve {
                    host: "a.example".to_string(),
                },
                class: OutcomeClass::Ok,
                data,
                observed_at: at,
                from_cache: false,
            }],
            ..CheckItem::default()
        }
    }

    #[test]
    fn a_judgement_over_an_expiry_holds_for_its_day_and_no_other_for_any_time() {
        let cache = CheckCache::default();
        let shelf = Shelf {
            cache: &cache,
            file: None,
            name: "shelf".to_string(),
            settings: String::new(),
        };
        let known = |item: CheckItem| {
            let (known, unknown) = shelf.split(CheckPhase::Judge, vec![item]);
            for (key, item) in unknown {
                let answer = PlannedItem::findings(Vec::new());
                shelf.keep(
                    key,
                    &PlannedItem {
                        id: item.id,
                        ..answer
                    },
                );
            }
            known.len() == 1
        };
        let certificate = || OutcomeData::Tls {
            not_after: Some("2030-01-01T00:00:00Z".to_string()),
            issuer: None,
            chain_valid: true,
        };
        assert!(!known(judged_over(certificate(), DAY_MS)));
        assert!(known(judged_over(certificate(), DAY_MS + 1000)), "its day");
        assert!(
            !known(judged_over(certificate(), 2 * DAY_MS)),
            "a later day"
        );

        let no_expiry = || OutcomeData::Tls {
            not_after: None,
            issuer: None,
            chain_valid: false,
        };
        assert!(!known(judged_over(no_expiry(), DAY_MS)));
        assert!(known(judged_over(no_expiry(), 2 * DAY_MS)));
        let other = || OutcomeData::Resolve {
            addresses: vec!["192.0.2.1".to_string()],
        };
        assert!(!known(judged_over(other(), DAY_MS)));
        assert!(known(judged_over(other(), 2 * DAY_MS)));
    }

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
