//! One change to project files under the write lock and the write journal (ADR 0128, ADR 0289): the lock is taken,
//! an interrupted write is rolled back, the caller's check runs over the disk as it is, the files are staged, the
//! journal records what they replace, the change is committed all or nothing, and the journal goes. A plan of
//! sources and the install of a host's files (ADR 0292) are both written this way.

use specforge_common::SourceRoot;

use super::ProjectChange;
use super::journal;
use crate::{Caller, Held, OpError, OpErrorKind, Writes};

/// The files of a change, staged with the journal's record of them.
pub(crate) struct Stager<'a> {
    caller: Caller<'a>,
    sources: &'a SourceRoot,
    change: ProjectChange,
    record: journal::Record,
}

impl Stager<'_> {
    /// `key` (from the project root) as the caller's containment holds it.
    pub(crate) fn held(&self, key: &str) -> Result<Held, OpError> {
        // A host file is never reached through a link (it can lead outside the project).
        if (crate::host::owned_path(key) || key.starts_with(".specforge/"))
            && let Some(link) = crate::host::link_in(self.sources.base(), key)
        {
            return Err(OpError::new(
                OpErrorKind::PermissionDenied,
                crate::FILE_UNWRITABLE,
                format!(
                    "{key} passes through the link {}: not written",
                    link.display()
                ),
            ));
        }
        Ok(self.caller.containment().resolve(&self.sources.path(key))?)
    }

    /// `key` held `before` (`None`: not there) and will hold `after`.
    pub(crate) fn write(
        &mut self,
        key: &str,
        before: Option<Vec<u8>>,
        after: Vec<u8>,
    ) -> Result<(), OpError> {
        let held = self.held(key)?;
        self.record.add(key, before.as_deref(), &after);
        self.change.stage(held, before, after);
        Ok(())
    }

    /// `key` held `before` and is removed.
    pub(crate) fn remove(&mut self, key: &str, before: Vec<u8>) -> Result<(), OpError> {
        let held = self.held(key)?;
        self.record.add_removal(key, &before);
        self.change.stage_removal(held, before);
        Ok(())
    }
}

/// Commit what `stage` stages. `locked`: take the write lock and journal the write (a project with nothing to
/// serialize gets no `.specforge/` from a write); `under_lock` runs once the lock is held and before anything is
/// staged, reading the disk as it is, and may refuse the change. The result is every file written, the files an
/// interrupted write left put back first.
pub(crate) fn commit(
    sources: &SourceRoot,
    doing: &str,
    caller: Caller<'_>,
    locked: bool,
    under_lock: impl FnOnce() -> Result<(), OpError>,
    stage: impl FnOnce(&mut Stager<'_>) -> Result<(), OpError>,
) -> Result<Writes, OpError> {
    let _lock = locked
        .then(|| specforge_project::harness::lock(sources.base()))
        .transpose()
        .map_err(|e| {
            OpError::new(
                OpErrorKind::Internal,
                crate::FILE_UNWRITABLE,
                format!("{doing}: cannot take the write lock: {e}"),
            )
        })?;
    // A write that was interrupted is put back first (ADR 0289); what the checks below read is the result.
    let mut rolled = if locked {
        journal::rollback(sources, caller)?
    } else {
        Writes::none()
    };
    under_lock()?;
    let mut stager = Stager {
        caller,
        sources,
        change: ProjectChange::new(sources.base(), doing),
        record: journal::Record::new(sources),
    };
    stage(&mut stager)?;
    let Stager { change, record, .. } = stager;
    if locked {
        record.begin(doing)?;
    }
    let result = change.commit();
    if locked {
        match &result {
            // Written: the record says so before it goes, and a failure to do either is the caller's.
            Ok(writes) => record
                .finish(doing)
                .map_err(|e| e.with_writes(writes.clone()))?,
            // Put back, or nothing written: the record is not needed. A file that could not be put back keeps it.
            Err(e) if e.writes.is_empty() => record.discard(),
            Err(_) => {}
        }
    }
    result.map(|writes| {
        rolled.merge(writes);
        rolled
    })
}
