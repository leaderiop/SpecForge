//! A project read against the disk as it is now (ADR 0289): what a write under the lock needs to know when another
//! writer may have landed since the project was compiled.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use specforge_registry::PlainDisk;

use super::CompiledProject;
use crate::ledger::{Reads, Unrecorded};
use crate::sources::{self, Read};

impl CompiledProject {
    /// Whether a source other than `except` is not what this project read: its file differs, has gone, or has
    /// appeared since. `except` are the files a write is about to change itself.
    pub fn sources_moved(&self, except: &BTreeSet<String>) -> bool {
        let sources_root = self.env.sources();
        let now: BTreeSet<String> = Unrecorded::plain()
            .discover(sources_root)
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
        had.iter().any(|key| {
            match (
                sources::read(&PlainDisk, sources_root, key),
                self.sources.text(key),
            ) {
                (Read::Text(disk), Some(read)) => disk != **read,
                (Read::Gone, Some(_)) | (Read::Text(_), None) => true,
                _ => false,
            }
        })
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
}
