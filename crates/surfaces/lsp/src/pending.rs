//! The ledger change of an LSP rename (ADR 0289). The server writes no source: a rename answers edits the editor
//! applies to its buffers. A project that declares a harness field keeps a ledger line per harness value, keyed by the
//! entity's path, so the rename's ledger change is held here until the rename lands on disk:
//!
//! - **waiting**: the answer was given; the declaring file's buffer still holds the old name;
//! - **showing**: the buffer holds the new name; the session's checks read the ledger as the rename will have written
//!   it, so the editor shows no E083 between the rename and its save;
//! - **undone**: the buffer holds the old name again after showing the new one, or the declaring file was closed
//!   with the disk still holding the old name; the change is dropped, nothing written;
//! - **landed**: the file on disk holds the new name; the ledger is written under the write lock.
//!
//! A rename of the name a showing rename gave (`a → b`, then `b → c` before saving) supersedes it: one change from the
//! first's project, the moves composed and the second's stale lines after the first's (ADR 0346 D7), shown and landed
//! in its place; undone, the first shows again.

use std::sync::{Arc, Mutex};

use specforge_ops::asking::Asking;
use specforge_ops::authoring::apply::{Origin, Surface, land_ledger};
use specforge_ops::{Caller, Containment};
use specforge_project::harness::{LedgerChange, LedgerOverlay, Line};
use specforge_project::{CompiledProject, ProjectSession};

/// Where a pending rename is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Waiting,
    Showing,
    /// A later rename of the name this one gave holds its change: neither shown nor landed while that one is.
    Superseded,
    Done,
}

/// What the declaring file's text names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reading {
    Old,
    New,
    Neither,
}

/// One rename's ledger change, until it lands or is undone.
#[derive(Debug)]
pub struct PendingLedger {
    overlay: Arc<LedgerOverlay>,
    /// The source key of the file declaring the renamed entity, and the names it holds before and after.
    declaring: String,
    old_name: String,
    new_name: String,
    /// The source keys of every file the rename edits.
    files: Vec<String>,
    phase: Mutex<Phase>,
    /// The showing rename of the name this one renames again, whose change this one's holds.
    supersedes: Option<Arc<PendingLedger>>,
}

impl PendingLedger {
    /// The pending change of renaming `old_name` (declared in `declaring`) to `new_name`, planned on `before`, which
    /// moved `moves` and marks the fact stores of `stale` stale (ADR 0346 D7); `None` when the project keeps no ledger
    /// (no root, or no harness field and no stale store).
    pub fn of(
        before: &CompiledProject,
        moves: Vec<(String, String)>,
        stale: Vec<Line>,
        declaring: String,
        old_name: String,
        new_name: String,
        files: Vec<String>,
    ) -> Option<Arc<PendingLedger>> {
        if before.root().is_none() || (!before.declares_harness() && stale.is_empty()) {
            return None;
        }
        let change = LedgerChange {
            moves,
            stale,
            ..LedgerChange::default()
        };
        Some(Arc::new(PendingLedger {
            overlay: Arc::new(LedgerOverlay::new(before.clone(), change)),
            declaring,
            old_name,
            new_name,
            files,
            phase: Mutex::new(Phase::Waiting),
            supersedes: None,
        }))
    }

    /// This rename after `earlier`, the showing rename of the name it renames again: one change from the project
    /// `earlier` was planned on, so it lands whole whichever of their files is saved last.
    fn after(&self, earlier: &Arc<PendingLedger>) -> PendingLedger {
        let change = earlier
            .overlay
            .change()
            .clone()
            .then(self.overlay.change().clone());
        let mut files = earlier.files.clone();
        files.extend(self.files.iter().cloned());
        files.sort();
        files.dedup();
        PendingLedger {
            overlay: Arc::new(LedgerOverlay::new(earlier.overlay.before().clone(), change)),
            declaring: self.declaring.clone(),
            old_name: self.old_name.clone(),
            new_name: self.new_name.clone(),
            files,
            phase: Mutex::new(self.phase()),
            supersedes: Some(Arc::clone(earlier)),
        }
    }

    fn phase(&self) -> Phase {
        *self.phase.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn set(&self, phase: Phase) {
        *self.phase.lock().unwrap_or_else(|e| e.into_inner()) = phase;
    }

    /// Whether the rename is over: landed or undone.
    pub fn is_done(&self) -> bool {
        self.phase() == Phase::Done
    }

    /// Whether the rename's ledger change is read over the checks now.
    pub fn is_showing(&self) -> bool {
        self.phase() == Phase::Showing
    }

    /// Whether the rename edits the file with this source key.
    pub fn touches(&self, key: &str) -> bool {
        self.files.iter().any(|file| file == key)
    }

    /// What `text`, the declaring file, names: the new name declared and the old one not, or the reverse.
    fn reading(&self, text: &str) -> Reading {
        let parsed = specforge_parser::parse(text, &self.declaring);
        let declares = |name: &str| parsed.entities.iter().any(|e| e.id.raw.as_str() == name);
        match (declares(&self.old_name), declares(&self.new_name)) {
            (false, true) => Reading::New,
            (true, false) => Reading::Old,
            _ => Reading::Neither,
        }
    }

    /// The editor's buffer of the declaring file is `text` now: a rename shown and then undone is dropped, and the
    /// rename it superseded shows again.
    fn buffer(&self, text: &str) {
        match (self.phase(), self.reading(text)) {
            (Phase::Waiting, Reading::New) => self.set(Phase::Showing),
            (Phase::Showing, Reading::Old) => {
                self.set(Phase::Done);
                if let Some(earlier) = &self.supersedes {
                    earlier.set(Phase::Showing);
                }
            }
            _ => {}
        }
    }

    /// Whether `text` holds the name this rename, or one it superseded, renamed from: none of them done.
    fn undone_by(&self, text: &str) -> bool {
        self.reading(text) == Reading::Old
            || self
                .supersedes
                .as_ref()
                .is_some_and(|earlier| earlier.undone_by(text))
    }

    /// This rename and every one it superseded are over.
    fn finish(&self) {
        self.set(Phase::Done);
        if let Some(earlier) = &self.supersedes {
            earlier.finish();
        }
    }

    /// The declaring file's buffer was closed and the disk holds `text`: a rename shown and never saved is dropped.
    /// `true` when it was.
    fn closed(&self, text: &str) -> bool {
        let dropped = self.phase() == Phase::Showing && self.undone_by(text);
        if dropped {
            self.finish();
        }
        dropped
    }

    /// The file on disk is `text`: when it holds the new name, the ledger is written once, over the project the
    /// session shows. `true` when it was. A write that fails leaves the rename pending, so the next event or the
    /// shutdown tries again; the overlay keeps showing the ledger as renamed meanwhile.
    fn lands(&self, text: &str, session: &ProjectSession) -> bool {
        let phase = self.phase();
        if phase == Phase::Done || phase == Phase::Superseded || self.reading(text) != Reading::New
        {
            return false;
        }
        let caller = Caller::new(&UNBOUNDED, Asking::Nobody);
        let origin = Origin::Core {
            surface: Surface::Lsp,
        };
        let written = land_ledger(
            self.overlay.before(),
            session.project(),
            self.overlay.change().clone(),
            &origin,
            caller,
        );
        if written.is_ok() {
            self.finish();
        }
        written.is_ok()
    }
}

static UNBOUNDED: Containment = Containment::unbounded();

/// The pending renames of a state, in the order they were answered.
#[derive(Debug, Default)]
pub struct Pending(Mutex<Vec<Arc<PendingLedger>>>);

impl Pending {
    /// Hold `pending`; when it renames again the name a showing rename gave, it supersedes that one
    /// ([`PendingLedger::after`]).
    pub fn hold(&self, pending: Arc<PendingLedger>) {
        let mut all = self.0.lock().unwrap_or_else(|e| e.into_inner());
        all.retain(|p| !p.is_done());
        let earlier = all.iter().find(|p| {
            p.is_showing() && p.declaring == pending.declaring && p.new_name == pending.old_name
        });
        let pending = match earlier {
            Some(earlier) => {
                earlier.set(Phase::Superseded);
                Arc::new(pending.after(earlier))
            }
            None => pending,
        };
        all.push(pending);
    }

    /// The renames not over.
    pub fn open(&self) -> Vec<Arc<PendingLedger>> {
        let mut all = self.0.lock().unwrap_or_else(|e| e.into_inner());
        all.retain(|p| !p.is_done());
        all.clone()
    }

    /// The editor's buffers are as `buffer` answers (the text of a source key, when it is open): each rename moves
    /// along, and the ones showing are what the checks read over.
    pub fn showing(&self, buffer: impl Fn(&str) -> Option<String>) -> Vec<Arc<LedgerOverlay>> {
        let open = self.open();
        // Every rename moves along first: one undone may show the rename it superseded again.
        for p in &open {
            if let Some(text) = buffer(&p.declaring) {
                p.buffer(&text);
            }
        }
        open.iter()
            .filter(|p| p.is_showing())
            .map(|p| Arc::clone(&p.overlay))
            .collect()
    }
}

impl Pending {
    /// The server is shutting down: a rename whose declaring file was saved and whose event never came lands now.
    pub fn flush(&self, session: &ProjectSession) {
        for p in self.open() {
            let path = session.project().environment().sources().path(&p.declaring);
            if let Ok(text) = std::fs::read_to_string(path) {
                p.lands(&text, session);
            }
        }
    }
}

/// What a watched-file event asks of the pending renames: the ones that edit a file it names, to land if the disk
/// says so.
pub struct Landing {
    pending: Vec<Arc<PendingLedger>>,
    keys: Vec<String>,
}

impl Landing {
    /// No rename waits on anything.
    pub fn none() -> Landing {
        Landing {
            pending: Vec::new(),
            keys: Vec::new(),
        }
    }

    pub fn of(pending: &Pending, keys: Vec<String>) -> Landing {
        Landing {
            pending: pending.open(),
            keys,
        }
    }

    /// Whether any rename waits on a file the event names.
    pub fn is_empty(&self) -> bool {
        !self
            .pending
            .iter()
            .any(|p| self.keys.iter().any(|k| p.touches(k)))
    }

    /// The buffer of the source `keys` names was closed: a rename shown in its declaring file and never saved is
    /// dropped, and the checks stop reading its ledger. `true` when the session needs its checks again.
    pub fn discard(&self, session: &mut ProjectSession) -> bool {
        let mut dropped = false;
        for p in self
            .pending
            .iter()
            .filter(|p| self.keys.iter().any(|k| *k == p.declaring))
        {
            let path = session.project().environment().sources().path(&p.declaring);
            if let Ok(text) = std::fs::read_to_string(path) {
                dropped |= p.closed(&text);
            }
        }
        let showing = self
            .pending
            .iter()
            .filter(|p| p.is_showing())
            .map(|p| Arc::clone(&p.overlay))
            .collect();
        session.overlay_ledger(showing) || dropped
    }

    /// Land the renames whose declaring file on disk holds the new name, then show only those still showing.
    /// `true` when the session needs its checks again.
    pub fn land(&self, session: &mut ProjectSession) -> bool {
        let mut landed = false;
        for p in self
            .pending
            .iter()
            .filter(|p| self.keys.iter().any(|k| p.touches(k)))
        {
            let path = session.project().environment().sources().path(&p.declaring);
            if let Ok(text) = std::fs::read_to_string(path) {
                landed |= p.lands(&text, session);
            }
        }
        let showing = self
            .pending
            .iter()
            .filter(|p| p.is_showing())
            .map(|p| Arc::clone(&p.overlay))
            .collect();
        session.overlay_ledger(showing) || landed
    }
}
