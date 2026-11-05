//! The `fs` port: local reads the grant permits, performed in every mode and never journaled.

use std::sync::Arc;

use specforge_effects::fs::{FileKind, READ_CEILING};
use specforge_effects::{EffectRequest, FsOp, FsRequest, OutcomeClass};
use specforge_protocol_types::{FsContent, FsEntry, FsEntryKind, FsListing, HostError};

use super::{PortSession, asked_cap, failed, held, out_of_scope};

impl PortSession {
    /// An fs request the grant permits, performed here: a local read in every mode, never journaled.
    fn fs(&self, request: FsRequest) -> Result<specforge_effects::FsOutcome, HostError> {
        let (kind, call) = self.lent()?;
        // `<root>/…` names another root than the one the grant judges.
        if request.path.starts_with('<') {
            return Err(out_of_scope(format!(
                "'{}' names a root inside its path: name it as the request's root",
                request.path
            )));
        }
        let asked = EffectRequest::Fs(request.clone());
        Self::granted(kind, call, &asked)?;
        let outcome = call
            .grant
            .fs(kind, Arc::clone(&call.fs))
            .run(&call.roots, &request);
        match outcome.class {
            OutcomeClass::Denied => Err(out_of_scope(format!(
                "`{}` is withheld",
                specforge_effects::grant::one_line(&asked)
            ))),
            _ => Ok(outcome),
        }
    }

    pub fn fs_stat(&mut self, root: &str, path: &str) -> Result<Option<FsEntry>, HostError> {
        let outcome = self.fs(FsRequest {
            root: root.to_string(),
            path: path.to_string(),
            op: FsOp::Stat,
        })?;
        match (outcome.class, outcome.stat) {
            (OutcomeClass::Ok, Some(stat)) => Ok(Some(FsEntry {
                kind: match stat.kind {
                    FileKind::File => FsEntryKind::File,
                    FileKind::Dir => FsEntryKind::Dir,
                    FileKind::Other => FsEntryKind::Other,
                },
                size: stat.size,
                modified_ms: stat.modified_ms,
            })),
            (OutcomeClass::Missing, _) => Ok(None),
            (class, _) => Err(failed(class, format!("{root}/{path}: {class}"))),
        }
    }

    pub fn fs_read(
        &mut self,
        root: &str,
        path: &str,
        max_bytes: u64,
    ) -> Result<FsContent, HostError> {
        let cap = asked_cap(max_bytes, self.answer_cap.min(READ_CEILING));
        let outcome = self.fs(FsRequest {
            root: root.to_string(),
            path: path.to_string(),
            op: FsOp::Read { max_bytes: cap },
        })?;
        if outcome.class != OutcomeClass::Ok {
            return Err(failed(
                outcome.class,
                format!("{root}/{path}: {}", outcome.class),
            ));
        }
        let (bytes, cut) = held(outcome.bytes, cap);
        Ok(FsContent {
            bytes,
            truncated: cut || outcome.truncated,
        })
    }

    pub fn fs_walk(
        &mut self,
        root: &str,
        path: &str,
        glob: Option<&str>,
        max_entries: u32,
    ) -> Result<FsListing, HostError> {
        let outcome = self.fs(FsRequest {
            root: root.to_string(),
            path: path.to_string(),
            op: FsOp::Walk {
                glob: glob.map(str::to_string),
                max_entries,
            },
        })?;
        if outcome.class != OutcomeClass::Ok {
            return Err(failed(
                outcome.class,
                format!("{root}/{path}: {}", outcome.class),
            ));
        }
        let mut room = self.answer_cap;
        let mut entries = outcome.entries;
        let kept = entries
            .iter()
            .take_while(|entry| {
                let fits = entry.len() as u64 <= room;
                room = room.saturating_sub(entry.len() as u64);
                fits
            })
            .count();
        let cut = kept < entries.len();
        entries.truncate(kept);
        Ok(FsListing {
            entries,
            truncated: cut || outcome.truncated,
        })
    }
}
