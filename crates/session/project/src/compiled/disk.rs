//! A project read against the disk as it is now (ADR 0289): what a write under the lock needs to know when another
//! writer may have landed since the project was compiled.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use specforge_registry::PlainDisk;

use super::CompiledProject;
use crate::ledger::{Reads, Unrecorded};
use crate::sources::Read;

impl CompiledProject {
    /// Whether a source other than `except` is not what this project read: its file differs, has gone, or has
    /// appeared since. `except` are the files a write is about to change itself.
    pub fn sources_moved(&self, except: &BTreeSet<String>) -> bool {
        let sources_root = self.env.sources();
        // One plain read: the harness ledger a fact store is checked against is read once, by `discover`.
        let mut plain = Unrecorded::plain();
        let now: BTreeSet<String> = plain
            .discover(sources_root, self.env.reads_facts())
            .into_iter()
            .filter(|key| !except.contains(key))
            .collect();
        let had: BTreeSet<String> = self
            .sources
            .keys()
            .into_iter()
            .map(String::from)
            .filter(|key| !except.contains(key))
            .collect();
        if now != had {
            return true;
        }
        had.iter().any(
            |key| match (plain.source(sources_root, key), self.sources.text(key)) {
                (Read::Text(disk), Some(read)) => disk != **read,
                (Read::Gone, Some(_)) | (Read::Text(_), None) => true,
                _ => false,
            },
        )
    }

    /// This project as if `texts` (by source key) were written over the sources on disk now, then every check, the
    /// harness ledger being `ledger` when given: what a write judged against the disk as it is would leave.
    pub fn edited_over_disk(
        &self,
        texts: &BTreeMap<String, String>,
        ledger: Option<&[u8]>,
    ) -> CompiledProject {
        let held: BTreeMap<String, &str> = texts
            .iter()
            .filter(|(key, _)| self.env.sources().holds_key(key))
            .map(|(key, text)| (key.clone(), text.as_str()))
            .collect();
        let mut project = Self::read(Arc::clone(&self.env), &held, &mut Unrecorded::plain());
        project.baseline = self.baseline.clone();
        match ledger {
            Some(bytes) => {
                let overlay = crate::harness::Overlay::new(self.env.root(), bytes);
                project.check(&mut Unrecorded::checking(&overlay));
            }
            None => project.check(&mut Unrecorded::plain()),
        }
        project
    }

    /// This project with each fact store served as the ledger reads with the renames the editor shows and has not
    /// saved (ADR 0289): a fact an earlier rename moved names the entity where it went, so a rename planned over it
    /// rewrites that fact too. Itself when it shows none. The checks are not run again.
    pub fn with_shown_facts(&self) -> Cow<'_, CompiledProject> {
        if self.ledger_overlay.is_empty() {
            return Cow::Borrowed(self);
        }
        self.served_through(&self.ledger_overlay)
    }

    /// [`Self::with_shown_facts`], then the stale lines `stale` (ADR 0346 D7) as one more change: what a plan's later
    /// rename reads, its earlier renames written. Itself when nothing changes what a store serves.
    pub fn with_facts_after(&self, stale: &[crate::harness::Line]) -> Cow<'_, CompiledProject> {
        if stale.is_empty() {
            return self.with_shown_facts();
        }
        let mut overlays = self.ledger_overlay.clone();
        overlays.push(Arc::new(crate::harness::LedgerOverlay::new(
            self.clone(),
            crate::harness::LedgerChange {
                stale: stale.to_vec(),
                ..Default::default()
            },
        )));
        self.served_through(&overlays)
    }

    /// This project with each fact store served as the ledger reads once `overlays` are made, in order. Itself when no
    /// store serves other than it does.
    fn served_through(
        &self,
        overlays: &[Arc<crate::harness::LedgerOverlay>],
    ) -> Cow<'_, CompiledProject> {
        let mut bytes = Vec::new();
        let disk = crate::harness::overlay::through(&PlainDisk, self, overlays, &mut bytes);
        let reads: Vec<(String, Read)> = self
            .sources
            .keys()
            .into_iter()
            .filter(|key| specforge_common::facts::is_fact_store(key))
            .map(|key| {
                (
                    key.to_string(),
                    crate::facts::read(&disk, self.env.sources(), key, None),
                )
            })
            .filter(|(key, read)| !self.is_current(key, read))
            .collect();
        if reads.is_empty() {
            return Cow::Borrowed(self);
        }
        let mut shown = self.clone();
        shown.apply(reads);
        Cow::Owned(shown)
    }
}
