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
//! A rename of the entity a showing rename gave (`a → b`, then `b → c` before saving, matched by path) composes with
//! it once its own edit is applied: from then on it supersedes the first, one change from the first's project, the
//! moves composed and the second's stale lines after the first's (ADR 0346 D7), shown and landed in its place; undone,
//! the first shows again; the buffer back at the first's old name, both are undone. Until the second's edit arrives
//! (or if it never does) the first shows and lands as itself.

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
    /// Shown as itself.
    Showing,
    /// Shown as one change after the rename it supersedes.
    Composed,
    /// A later rename of the entity this one gave holds its change: neither shown nor landed while that one is.
    /// Whether this one was composed itself, to show again as it was.
    Superseded {
        composed: bool,
    },
    Done,
}

/// What the declaring file's text names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reading {
    Old,
    New,
    Neither,
}

/// A showing rename this one renames the entity of again, and the change and files of both, as one.
#[derive(Debug)]
struct After {
    earlier: Arc<PendingLedger>,
    overlay: Arc<LedgerOverlay>,
    files: Vec<String>,
}

/// One rename's ledger change, until it lands or is undone.
#[derive(Debug)]
pub struct PendingLedger {
    overlay: Arc<LedgerOverlay>,
    /// The source key of the file declaring the renamed entity.
    declaring: String,
    /// The renamed entity's path before and after: what a later rename is matched by.
    old_id: String,
    new_id: String,
    /// The name the declaring file writes before and after, and how many entities it declared with each when the
    /// rename was planned: two scoped entities may share a name.
    old_name: String,
    new_name: String,
    declared: (usize, usize),
    /// The source keys of every file the rename edits.
    files: Vec<String>,
    phase: Mutex<Phase>,
    /// The showing rename of the entity this one renames again, when there was one at the answer.
    after: Option<After>,
}

/// What a rename renames: the declaring file's key, the entity's path and name before and after.
pub struct Renamed {
    pub declaring: String,
    pub old_id: String,
    pub new_id: String,
    pub old_name: String,
    pub new_name: String,
}

impl PendingLedger {
    /// The pending change of `renamed`, planned on `before`, which moved `moves` and marks the fact stores of `stale`
    /// stale (ADR 0346 D7); `None` when the project keeps no ledger (no root, or no harness field and no stale store).
    pub fn of(
        before: &CompiledProject,
        moves: Vec<(String, String)>,
        stale: Vec<Line>,
        renamed: Renamed,
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
        let declared = before
            .source_text(&renamed.declaring)
            .map_or((1, 0), |text| {
                declared(
                    &text,
                    &renamed.declaring,
                    &renamed.old_name,
                    &renamed.new_name,
                )
            });
        Some(Arc::new(PendingLedger {
            overlay: Arc::new(LedgerOverlay::new(before.clone(), change)),
            declaring: renamed.declaring,
            old_id: renamed.old_id,
            new_id: renamed.new_id,
            old_name: renamed.old_name,
            new_name: renamed.new_name,
            declared,
            files,
            phase: Mutex::new(Phase::Waiting),
            after: None,
        }))
    }

    /// This rename, able to compose with `earlier`, the showing rename of the entity it renames again: one change
    /// from the project `earlier` was planned on, so it lands whole whichever of their files is saved last.
    fn after(&self, earlier: &Arc<PendingLedger>) -> Arc<PendingLedger> {
        let change = earlier
            .overlay()
            .change()
            .clone()
            .then(self.overlay.change().clone());
        let mut files = earlier.files().to_vec();
        files.extend(self.files.iter().cloned());
        files.sort();
        files.dedup();
        let after = After {
            earlier: Arc::clone(earlier),
            overlay: Arc::new(LedgerOverlay::new(
                earlier.overlay().before().clone(),
                change,
            )),
            files,
        };
        Arc::new(PendingLedger {
            overlay: Arc::clone(&self.overlay),
            declaring: self.declaring.clone(),
            old_id: self.old_id.clone(),
            new_id: self.new_id.clone(),
            old_name: self.old_name.clone(),
            new_name: self.new_name.clone(),
            declared: self.declared,
            files: self.files.clone(),
            phase: Mutex::new(self.phase()),
            after: Some(after),
        })
    }

    fn phase(&self) -> Phase {
        *self.phase.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn set(&self, phase: Phase) {
        *self.phase.lock().unwrap_or_else(|e| e.into_inner()) = phase;
    }

    /// The change shown and landed: its own, or the one after the rename it supersedes.
    fn overlay(&self) -> &Arc<LedgerOverlay> {
        match (self.phase(), &self.after) {
            (Phase::Composed | Phase::Superseded { composed: true }, Some(after)) => &after.overlay,
            _ => &self.overlay,
        }
    }

    fn files(&self) -> &[String] {
        match (self.phase(), &self.after) {
            (Phase::Composed | Phase::Superseded { composed: true }, Some(after)) => &after.files,
            _ => &self.files,
        }
    }

    /// Whether the rename is over: landed or undone.
    pub fn is_done(&self) -> bool {
        self.phase() == Phase::Done
    }

    /// Whether the rename's ledger change is read over the checks now.
    pub fn is_showing(&self) -> bool {
        matches!(self.phase(), Phase::Showing | Phase::Composed)
    }

    /// Whether the rename edits the file with this source key.
    pub fn touches(&self, key: &str) -> bool {
        self.files().iter().any(|file| file == key)
    }

    /// What `text`, the declaring file, names: one entity fewer declared with the old name than when the rename was
    /// planned, and one at least with the new; or as many with the old as then, and no more with the new. Counted,
    /// not asked whether a name is declared: two scoped entities may share a name, and a later rename of the other
    /// one moves the count of a name this one wrote.
    fn reading(&self, text: &str) -> Reading {
        let (old, new) = declared(text, &self.declaring, &self.old_name, &self.new_name);
        let (planned_old, planned_new) = self.declared;
        if old == planned_old && new <= planned_new {
            Reading::Old
        } else if old + 1 == planned_old && new >= 1 {
            Reading::New
        } else {
            Reading::Neither
        }
    }

    /// The rename's edit is in the buffers: it shows, superseding the rename it renames again while that one shows.
    fn show(&self) {
        match &self.after {
            Some(after) if after.earlier.is_showing() => {
                let composed = after.earlier.phase() == Phase::Composed;
                after.earlier.set(Phase::Superseded { composed });
                self.set(Phase::Composed);
            }
            _ => self.set(Phase::Showing),
        }
    }

    /// The editor's buffer of the declaring file is `text` now: a rename applied shows; one shown and then undone is
    /// dropped, and the rename it superseded shows again; the buffer back at a name a rename it superseded renamed
    /// from, all of them are.
    fn buffer(&self, text: &str) {
        match (self.phase(), self.reading(text)) {
            (Phase::Waiting, Reading::New) => self.show(),
            (phase @ (Phase::Showing | Phase::Composed), Reading::Old) => {
                self.set(Phase::Done);
                if let (Phase::Composed, Some(after)) = (phase, &self.after) {
                    after.earlier.resume();
                }
            }
            (Phase::Showing | Phase::Composed, _) if self.undone_by(text) => self.finish(),
            _ => {}
        }
    }

    /// Shown again as it was before a later rename superseded it.
    fn resume(&self) {
        if let Phase::Superseded { composed } = self.phase() {
            self.set(if composed {
                Phase::Composed
            } else {
                Phase::Showing
            });
        }
    }

    /// The rename this one superseded, while it does.
    fn superseded(&self) -> Option<&Arc<PendingLedger>> {
        match (self.phase(), &self.after) {
            (Phase::Composed | Phase::Superseded { composed: true }, Some(after)) => {
                Some(&after.earlier)
            }
            _ => None,
        }
    }

    /// Whether `text` holds the name this rename, or one it superseded, renamed from: none of them done.
    fn undone_by(&self, text: &str) -> bool {
        self.reading(text) == Reading::Old
            || self
                .superseded()
                .is_some_and(|earlier| earlier.undone_by(text))
    }

    /// This rename and every one it superseded are over.
    fn finish(&self) {
        if let Some(earlier) = self.superseded() {
            earlier.finish();
        }
        self.set(Phase::Done);
    }

    /// The declaring file's buffer was closed and the disk holds `text`: a rename shown and never saved is dropped.
    /// `true` when it was.
    fn closed(&self, text: &str) -> bool {
        let dropped = self.is_showing() && self.undone_by(text);
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
        if matches!(phase, Phase::Done | Phase::Superseded { .. })
            || self.reading(text) != Reading::New
        {
            return false;
        }
        if phase == Phase::Waiting {
            self.show();
        }
        let caller = Caller::new(&UNBOUNDED, Asking::Nobody);
        let origin = Origin::Core {
            surface: Surface::Lsp,
        };
        let overlay = self.overlay();
        let written = land_ledger(
            overlay.before(),
            session.project(),
            overlay.change().clone(),
            &origin,
            caller,
        );
        if written.is_ok() {
            self.finish();
        }
        written.is_ok()
    }
}

/// How many entities `text` (the source `key`) declares named `old` and named `new`.
fn declared(text: &str, key: &str, old: &str, new: &str) -> (usize, usize) {
    let parsed = specforge_parser::parse(text, key);
    let count = |name: &str| {
        parsed
            .entities
            .iter()
            .filter(|e| e.id.raw.as_str() == name)
            .count()
    };
    (count(old), count(new))
}

static UNBOUNDED: Containment = Containment::unbounded();

/// The pending renames of a state, in the order they were answered.
#[derive(Debug, Default)]
pub struct Pending(Mutex<Vec<Arc<PendingLedger>>>);

impl Pending {
    /// Hold `pending`; when it renames again the entity a showing rename gave (by path), it composes with that one
    /// once its edit is applied ([`PendingLedger::after`]).
    pub fn hold(&self, pending: Arc<PendingLedger>) {
        let mut all = self.0.lock().unwrap_or_else(|e| e.into_inner());
        all.retain(|p| !p.is_done());
        let earlier = all
            .iter()
            .find(|p| p.is_showing() && p.new_id == pending.old_id);
        let pending = match earlier {
            Some(earlier) => pending.after(earlier),
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
            .map(|p| Arc::clone(p.overlay()))
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
            .map(|p| Arc::clone(p.overlay()))
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
            .map(|p| Arc::clone(p.overlay()))
            .collect();
        session.overlay_ledger(showing) || landed
    }
}
