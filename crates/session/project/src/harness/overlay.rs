//! A rename the editor has applied to its buffers and not yet saved (ADR 0289): the session's checks read the ledger
//! as the rename will have written it, so the editor shows no E083 between the rename and its save.

use specforge_registry::Disk;

use super::{LEDGER_FILE, LedgerChange, Overlay};
use crate::CompiledProject;

/// The ledger change of a rename, over the project it was planned on.
pub struct LedgerOverlay {
    before: CompiledProject,
    change: LedgerChange,
}

impl LedgerOverlay {
    pub fn new(before: CompiledProject, change: LedgerChange) -> Self {
        LedgerOverlay { before, change }
    }

    /// The project the rename was planned on.
    pub fn before(&self) -> &CompiledProject {
        &self.before
    }

    pub fn change(&self) -> &LedgerChange {
        &self.change
    }

    /// The ledger `current` as the rename leaves it, the project being `after`: a line another writer added is
    /// re-keyed too.
    pub fn rewritten(&self, after: &CompiledProject, current: &[u8]) -> Vec<u8> {
        self.before
            .rewritten_ledger(after, current, &self.change, true)
    }
}

impl std::fmt::Debug for LedgerOverlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LedgerOverlay")
            .field("change", &self.change)
            .finish_non_exhaustive()
    }
}

/// `disk` with the ledger of `after` as `overlays`, in order, leave it.
pub(crate) fn through<'a>(
    disk: &'a dyn Disk,
    after: &CompiledProject,
    overlays: &[std::sync::Arc<LedgerOverlay>],
    bytes: &'a mut Vec<u8>,
) -> Overlay<'a> {
    let root = after.environment().root().to_path_buf();
    let mut current = disk.read(&root.join(LEDGER_FILE)).unwrap_or_default();
    for overlay in overlays {
        current = overlay.rewritten(after, &current);
    }
    *bytes = current;
    Overlay::over(disk, &root, bytes)
}
