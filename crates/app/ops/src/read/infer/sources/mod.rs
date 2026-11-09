//! The inference sources (ADR 0113, ADR 0199): the files the item facts of the compiled graph locate under the
//! manifest's source roots, keyed by one source path rule, and read inside the project root.

use std::path::Path;

use sha2::{Digest, Sha256};

use super::items::item_facts;
use super::manifest::{InferenceManifest, SourceFileEntry};
use crate::source_key::SourceKey;
use crate::view::ProjectView;
use crate::{Containment, Outside, ReadFailure};

/// What reading a source file found.
#[derive(Debug)]
pub(crate) enum SourceRead {
    Bytes(Vec<u8>),
    Missing,
    /// The name lands outside the project root: where, and what bounds it.
    Outside(crate::Outside),
    Unreadable(std::io::Error),
}

/// The project's files as inference reads them: held inside the project root on every surface
/// (ADR 0113 D4).
#[derive(Debug)]
pub(crate) struct ProjectFiles {
    held: Containment,
}

impl ProjectFiles {
    pub(crate) fn at(root: &Path) -> Self {
        ProjectFiles {
            held: Containment::within(root),
        }
    }

    /// Whether `key` names a directory held inside the project root; `Outside` when it lands elsewhere.
    pub(crate) fn directory(&self, key: &str) -> Result<bool, Outside> {
        Ok(self.held.resolve(Path::new(key))?.real.is_dir())
    }

    /// The file at `key`, read once through its held name.
    pub(crate) fn read(&self, key: &SourceKey) -> SourceRead {
        match self.held.read(Path::new(key.as_str())) {
            Ok((_, bytes)) => SourceRead::Bytes(bytes),
            Err(ReadFailure::Outside(outside)) => SourceRead::Outside(outside),
            Err(ReadFailure::Io { error, .. }) if error.kind() == std::io::ErrorKind::NotFound => {
                SourceRead::Missing
            }
            Err(ReadFailure::Io { error, .. }) => SourceRead::Unreadable(error),
        }
    }
}

/// Where an index entry's file stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IndexState {
    /// Its hash is the recorded one.
    Current { lines: Option<usize> },
    /// Its hash differs from the recorded one.
    Changed { lines: Option<usize> },
    /// The path is no key, lands outside the project, or names no file.
    Deleted,
    /// The file exists and cannot be read.
    Unreadable,
}

/// An index entry and where its file stands.
#[derive(Debug)]
pub(crate) struct Indexed<'m> {
    pub(crate) entry: &'m SourceFileEntry,
    /// The key the entry's path is read into from the project root; `None` when it is no key.
    pub(crate) key: Option<SourceKey>,
    pub(crate) state: IndexState,
}

impl ProjectFiles {
    /// Each index entry, in index order, its file read once: `Deleted` when the path is no key,
    /// lands outside or is missing; `Unreadable` when it exists and cannot be read; else
    /// `Current` or `Changed` by its hash, with its line count when it is UTF-8.
    pub(crate) fn indexed<'m>(&self, manifest: &'m InferenceManifest) -> Vec<Indexed<'m>> {
        let root = self.held.root();
        manifest
            .source_index
            .iter()
            .map(|entry| {
                let key = root.and_then(|root| SourceKey::read(root, &entry.path).ok());
                let state = match key.as_ref().map(|key| self.read(key)) {
                    None | Some(SourceRead::Missing | SourceRead::Outside(_)) => {
                        IndexState::Deleted
                    }
                    Some(SourceRead::Unreadable(_)) => IndexState::Unreadable,
                    Some(SourceRead::Bytes(bytes)) => {
                        let lines = std::str::from_utf8(&bytes)
                            .ok()
                            .map(|text| text.lines().count().max(1));
                        if content_hash(&bytes) == entry.content_hash {
                            IndexState::Current { lines }
                        } else {
                            IndexState::Changed { lines }
                        }
                    }
                };
                Indexed { entry, key, state }
            })
            .collect()
    }
}

/// The inference sources of one project: the files its item facts locate.
#[derive(Debug)]
pub(crate) struct InferenceSources {
    files: ProjectFiles,
    keys: Vec<SourceKey>,
}

impl InferenceSources {
    /// The files the item facts of `view`'s graph (public or not) locate under `manifest`'s source roots (the
    /// project root when it has none), read from the project `root`: which files exist is the collectors' to say.
    pub(crate) fn of(view: &ProjectView, root: &Path, manifest: &InferenceManifest) -> Self {
        let mut keys: Vec<SourceKey> = item_facts(view, root, &manifest.source_roots)
            .into_iter()
            .map(|item| item.key)
            .collect();
        keys.sort();
        keys.dedup();
        InferenceSources {
            files: ProjectFiles::at(root),
            keys,
        }
    }

    /// Sorted, each once.
    pub(crate) fn keys(&self) -> &[SourceKey] {
        &self.keys
    }

    pub(crate) fn files(&self) -> &ProjectFiles {
        &self.files
    }
}

/// The SHA-256 of `bytes`, lowercase hex.
pub(crate) fn content_hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests;
