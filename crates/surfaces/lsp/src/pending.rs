//! The ledger change of an LSP rename (ADR 0289). The server writes no source: a rename answers edits the editor
//! applies to its buffers. A project that declares a harness field keeps a ledger line per harness value, keyed by the
//! entity's path, and a rename marks the fact stores whose names it rewrites in source files stale (ADR 0346 D7), so
//! the rename's ledger change is held here until it lands on disk.
//!
//! Renames of one entity made before any lands are a **chain** (`a → b`, then `b → c`, matched by path), one change
//! from the project the first was planned on (ADR 0346 D7). What a chain shows and lands is never kept as a phase: it
//! is read from the texts each time, so whatever the editor did to its buffers (undo in one buffer and not another,
//! part way, by hand, redo) the ledger read is what the texts say:
//!
//! - the **moves** of the renames the declaring file holds: up to the last whose new name it declares (counted: two
//!   scoped entities may share a name); none when it declares the first's old name, or a name no rename wrote;
//! - for each **stale line**, by store, field and name held: the line of the last rename the source files it is about
//!   hold, each file read as the text a rename wrote or read (an editor's undo restores it byte for byte); none when
//!   a file holds no text of the chain's (typed in since: no line makes it the text collected either way).
//!
//! The checks read, over the session, what the buffers say (a file no buffer holds, as on disk). Once no file of a
//! chain holds a buffer the disk does not (each one saved, or closed), what the disk says lands, once, under the write
//! lock, and the chain is over; a write that fails leaves it to the next event. A rename answered whose edit no text
//! was ever read holding stays, while its declaring file is open, to land as itself when its edit comes. A rename of
//! the entity a chain's declaring file holds continues the chain there; the renames after that point, whose texts a
//! source file may still hold, go on as a chain of their own.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use specforge_ops::asking::Asking;
use specforge_ops::authoring::apply::{Origin, Surface, land_ledger};
use specforge_ops::authoring::rename::Marked;
use specforge_ops::{Caller, Containment};
use specforge_project::facts::Span;
use specforge_project::harness::{LedgerChange, LedgerOverlay, Line, compose};
use specforge_project::{CompiledProject, ProjectSession};

mod reads;
use reads::reads;

/// One rename's ledger change, and the texts it read and wrote.
pub struct PendingLedger {
    /// The project the rename was planned on.
    before: CompiledProject,
    moves: Vec<(String, String)>,
    stale: Vec<Marked>,
    /// Each source file the rename rewrote: the text it read and the text it wrote.
    sources: BTreeMap<String, (Arc<str>, String)>,
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
    /// A text was read holding the rename's edit: from then on, a reading without it ends it.
    seen: AtomicBool,
    /// The rename is the last of its chain the texts hold, and the chain shows a change.
    showing: AtomicBool,
}

impl std::fmt::Debug for PendingLedger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingLedger")
            .field("old_id", &self.old_id)
            .field("new_id", &self.new_id)
            .field("moves", &self.moves)
            .field("stale", &self.stale)
            .field("showing", &self.is_showing())
            .finish_non_exhaustive()
    }
}

/// What a rename renames: the declaring file's key, the entity's path and name before and after.
pub struct Renamed {
    pub declaring: String,
    pub old_id: String,
    pub new_id: String,
    pub old_name: String,
    pub new_name: String,
}

/// What a rename changes besides the declaring file's name: the entities it moves (old path, new path), the stale
/// lines it marks with the source files each is about, each source file it rewrote (key, text read, text written),
/// and the key of every file it edits.
pub struct Rewrites {
    pub moves: Vec<(String, String)>,
    pub stale: Vec<Marked>,
    pub sources: Vec<(String, Arc<str>, String)>,
    pub files: Vec<String>,
}

impl PendingLedger {
    /// The pending change of `renamed`, planned on `before` (ADR 0346 D7); `None` when the project keeps no ledger (no
    /// root, or no harness field and no stale store).
    pub fn of(
        before: &CompiledProject,
        renamed: Renamed,
        rewrites: Rewrites,
    ) -> Option<Arc<PendingLedger>> {
        if before.root().is_none() || (!before.declares_harness() && rewrites.stale.is_empty()) {
            return None;
        }
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
            before: before.clone(),
            moves: rewrites.moves,
            stale: rewrites.stale,
            sources: rewrites
                .sources
                .into_iter()
                .map(|(key, read, wrote)| (key, (read, wrote)))
                .collect(),
            declaring: renamed.declaring,
            old_id: renamed.old_id,
            new_id: renamed.new_id,
            old_name: renamed.old_name,
            new_name: renamed.new_name,
            declared,
            files: rewrites.files,
            seen: AtomicBool::new(false),
            showing: AtomicBool::new(false),
        }))
    }

    /// Whether the rename's chain shows a change, and this rename is the last of it the texts hold.
    pub fn is_showing(&self) -> bool {
        self.showing.load(Ordering::Relaxed)
    }

    /// Whether `text`, the declaring file, declares the new name: one entity fewer with the old name than when the
    /// rename was planned, and one at least with the new. Counted, not asked whether a name is declared: two scoped
    /// entities may share a name, and a later rename of the other one moves the count of a name this one wrote.
    fn declares_new(&self, text: &str) -> bool {
        let (old, new) = declared(text, &self.declaring, &self.old_name, &self.new_name);
        old + 1 == self.declared.0 && new >= 1
    }

    /// The stale line this rename marks for `key`, if any.
    fn line(&self, key: &Key) -> Option<&Line> {
        self.marked(key).map(|m| &m.line)
    }

    /// What this rename marks for `key`, if anything.
    fn marked(&self, key: &Key) -> Option<&Marked> {
        self.stale
            .iter()
            .find(|m| key_of(&m.line).as_ref() == Some(key))
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

/// A stale line's collector, store, field and name held: what a later line of the same key replaces.
type Key = (String, String, String, String);

fn key_of(line: &Line) -> Option<Key> {
    let stale = line.stale_of()?;
    Some((
        stale.collector.to_string(),
        stale.key.to_string(),
        stale.field.to_string(),
        stale.held.to_string(),
    ))
}

/// What the texts say of a chain.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Reading {
    /// How many of the renames the declaring file holds: their moves are read.
    declared: usize,
    /// For each stale line the renames mark ([`keys`], in order): the rename whose line is read, 0 for none.
    lines: Vec<usize>,
}

impl Reading {
    /// How many renames the texts hold, in the declaring file or in a source file.
    fn top(&self) -> usize {
        self.lines
            .iter()
            .copied()
            .chain([self.declared])
            .max()
            .unwrap_or(0)
    }
}

/// The step of the declaring file's `text` in `renames`: the last rename whose new name it declares; 0 when none
/// does (the first's old name, a name no rename wrote, or no text).
fn declaring_step(renames: &[Arc<PendingLedger>], text: Option<&str>) -> usize {
    let Some(text) = text else { return 0 };
    (1..=renames.len())
        .rev()
        .find(|&i| renames[i - 1].declares_new(text))
        .unwrap_or(0)
}

/// The step of the source `file`'s `text` in `renames` for the stale line `key`: the last rename whose name for it
/// still reads at the places it wrote it in the file, else one before the first whose name it read still reads at
/// its places, else 0 (the name changed to one no rename wrote). Whatever else changed in the file (another
/// entity renamed, typing) does not move the step. `None` when no rename marking the line rewrote the file.
fn source_step(
    renames: &[Arc<PendingLedger>],
    key: &Key,
    file: &str,
    text: Option<&str>,
) -> Option<usize> {
    let marked = |i: usize| {
        let rename = &renames[i - 1];
        Some((rename.marked(key)?, rename.sources.get(file)?))
    };
    let n = renames.len();
    if !(1..=n).any(|i| marked(i).is_some()) {
        return None;
    }
    let Some(text) = text else { return Some(0) };
    let spans = |marked: &Marked, read: bool| -> Vec<Span> {
        let at = marked.places.iter().filter(|p| p.file == file);
        at.map(|p| if read { p.read } else { p.wrote }).collect()
    };
    let wrote = (1..=n).rev().find(|&i| {
        marked(i).is_some_and(|(m, (_, wrote))| reads(wrote, text, &spans(m, false), &m.wrote))
    });
    let read = || {
        (1..=n)
            .find(|&i| {
                marked(i).is_some_and(|(m, (read, _))| reads(read, text, &spans(m, true), &m.read))
            })
            .map(|i| i - 1)
    };
    Some(wrote.or_else(read).unwrap_or(0))
}

/// Every stale line `renames` mark, by key, in the order first marked, with the source files any of them is about.
fn keys(renames: &[Arc<PendingLedger>]) -> Vec<(Key, BTreeSet<String>)> {
    let mut keys: Vec<(Key, BTreeSet<String>)> = Vec::new();
    for marked in renames.iter().flat_map(|r| &r.stale) {
        let Some(key) = key_of(&marked.line) else {
            continue;
        };
        let files = marked.files().into_iter().map(str::to_string);
        match keys.iter_mut().find(|(k, _)| *k == key) {
            Some((_, known)) => known.extend(files),
            None => keys.push((key, files.collect())),
        }
    }
    keys
}

/// What the texts `text` answers say of `renames`. A stale line is at the earliest step any file it is about holds
/// (one line names one name for every file); with no file a rename marking it rewrote, at the declaring file's step.
fn read(renames: &[Arc<PendingLedger>], text: &dyn Fn(&str) -> Option<String>) -> Reading {
    let declared = declaring_step(renames, text(&renames[0].declaring).as_deref());
    let lines = keys(renames)
        .iter()
        .map(|(key, files)| {
            let step = files
                .iter()
                .filter_map(|file| source_step(renames, key, file, text(file).as_deref()))
                .min()
                .unwrap_or(declared);
            (1..=step)
                .rev()
                .find(|&i| renames[i - 1].line(key).is_some())
                .unwrap_or(0)
        })
        .collect();
    Reading { declared, lines }
}

/// The change `reading` says of `renames`, from the project the first was planned on: the moves of the renames the
/// declaring file holds, composed (a name renamed back moves nothing), and the stale line read for each key.
fn change(renames: &[Arc<PendingLedger>], reading: &Reading) -> LedgerChange {
    let mut moves = Vec::new();
    for (old, new) in renames[..reading.declared].iter().flat_map(|r| &r.moves) {
        compose(&mut moves, old, new);
    }
    moves.retain(|(old, new)| old != new);
    let stale = keys(renames)
        .iter()
        .zip(&reading.lines)
        .filter(|(_, at)| **at > 0)
        .filter_map(|((key, _), at)| renames[at - 1].line(key).cloned())
        .collect();
    LedgerChange {
        moves,
        stale,
        ..LedgerChange::default()
    }
}

fn is_empty(change: &LedgerChange) -> bool {
    change.moves.is_empty() && change.stale.is_empty()
}

/// The text of the file `key` on disk, found through `project`'s sources.
fn on_disk(project: &CompiledProject, key: &str) -> Option<String> {
    std::fs::read_to_string(project.environment().sources().path(key)).ok()
}

/// A chain's renames, and the overlay last shown with the reading it was made of: the same reading shows the same
/// overlay, so the session's checks do not run again for nothing.
#[derive(Debug, Default)]
struct Links {
    renames: Vec<Arc<PendingLedger>>,
    shown: Option<(Reading, Arc<LedgerOverlay>)>,
}

/// Renames of one entity, each of the entity the one before gave, none landed.
#[derive(Debug, Default)]
struct Chain(Mutex<Links>);

impl Chain {
    fn of(renames: Vec<Arc<PendingLedger>>) -> Arc<Chain> {
        Arc::new(Chain(Mutex::new(Links {
            renames,
            shown: None,
        })))
    }

    fn links(&self) -> MutexGuard<'_, Links> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn is_over(&self) -> bool {
        self.links().renames.is_empty()
    }

    fn renames(&self) -> Vec<Arc<PendingLedger>> {
        self.links().renames.clone()
    }

    /// Whether a rename of the chain edits the file `key`.
    fn touches(&self, key: &str) -> bool {
        self.links()
            .renames
            .iter()
            .any(|r| r.files.iter().any(|file| file == key))
    }

    /// The overlay last shown.
    fn shown(&self) -> Option<Arc<LedgerOverlay>> {
        let links = self.links();
        links.shown.as_ref().map(|(_, overlay)| Arc::clone(overlay))
    }

    /// `pending`, planned on a project whose declaring file this chain reads at the rename that gave the entity
    /// `pending` renames, continues the chain there; the renames after that point (undone in that file, their texts
    /// maybe still in a source file) are returned, to go on as a chain of their own. `None` when `pending` renames
    /// something else.
    fn join(&self, pending: &Arc<PendingLedger>) -> Option<Vec<Arc<PendingLedger>>> {
        let mut links = self.links();
        let first = links.renames.first()?;
        if first.declaring != pending.declaring {
            return None;
        }
        let text = pending.before.source_text(&pending.declaring);
        let at = declaring_step(&links.renames, text.as_deref());
        let id = match at {
            0 => &first.old_id,
            at => &links.renames[at - 1].new_id,
        };
        if *id != pending.old_id {
            return None;
        }
        let after = links.renames.split_off(at);
        links.renames.push(Arc::clone(pending));
        links.shown = None;
        Some(after)
    }

    /// What the chain shows over the session when its files read as `text` answers (a file it does not answer, as on
    /// disk); `None` when that is no change.
    fn show(&self, text: &dyn Fn(&str) -> Option<String>) -> Option<Arc<LedgerOverlay>> {
        let mut links = self.links();
        let before = links.renames.first()?.before.clone();
        let reading = read(&links.renames, &|key| {
            text(key).or_else(|| on_disk(&before, key))
        });
        let change = change(&links.renames, &reading);
        let top = reading.top();
        for (at, rename) in links.renames.iter().enumerate() {
            if at < top {
                rename.seen.store(true, Ordering::Relaxed);
            }
            let showing = at + 1 == top && !is_empty(&change);
            rename.showing.store(showing, Ordering::Relaxed);
        }
        if is_empty(&change) {
            links.shown = None;
            return None;
        }
        if let Some((shown, overlay)) = &links.shown
            && *shown == reading
        {
            return Some(Arc::clone(overlay));
        }
        let overlay = Arc::new(LedgerOverlay::new(before, change));
        links.shown = Some((reading, Arc::clone(&overlay)));
        Some(overlay)
    }

    /// Land what the disk says of the chain, over the project the session shows. `Ok(true)` when the ledger was
    /// written; `Err` when the write failed, the chain left as it was. Landed (or with nothing to land), the chain is
    /// over but for the renames after the last a text was read holding, which stay while `open` says their declaring
    /// file is open: their edits may yet come.
    fn land(&self, session: &ProjectSession, open: &dyn Fn(&str) -> bool) -> Result<bool, ()> {
        let mut links = self.links();
        if links.renames.is_empty() {
            return Ok(false);
        }
        let project = session.project();
        let reading = read(&links.renames, &|key| on_disk(project, key));
        let change = change(&links.renames, &reading);
        let written = !is_empty(&change);
        if written {
            let caller = Caller::new(&UNBOUNDED, Asking::Nobody);
            let origin = Origin::Core {
                surface: Surface::Lsp,
            };
            land_ledger(&links.renames[0].before, project, change, &origin, caller)
                .map_err(drop)?;
        }
        let top = reading.top();
        let seen = links
            .renames
            .iter()
            .enumerate()
            .filter(|(at, rename)| *at < top || rename.seen.load(Ordering::Relaxed))
            .map(|(at, _)| at + 1)
            .max()
            .unwrap_or(0);
        let mut after = links.renames.split_off(seen);
        after.retain(|rename| open(&rename.declaring));
        for rename in links.renames.iter().chain(&after) {
            rename.showing.store(false, Ordering::Relaxed);
        }
        links.renames = after;
        links.shown = None;
        Ok(written)
    }
}

static UNBOUNDED: Containment = Containment::unbounded();

/// The pending renames of a state: their chains, in the order they were begun.
#[derive(Debug, Default)]
pub struct Pending(Mutex<Vec<Arc<Chain>>>);

impl Pending {
    /// The chains not over.
    fn chains(&self) -> MutexGuard<'_, Vec<Arc<Chain>>> {
        let mut chains = self.0.lock().unwrap_or_else(|e| e.into_inner());
        chains.retain(|chain| !chain.is_over());
        chains
    }

    /// Hold `pending`: it continues the chain whose declaring file, as `pending` was planned on it, holds the rename
    /// that gave the entity it renames (by path), else begins one ([`Chain::join`]).
    pub fn hold(&self, pending: Arc<PendingLedger>) {
        let mut chains = self.chains();
        for chain in chains.iter() {
            if let Some(after) = chain.join(&pending) {
                if !after.is_empty() {
                    chains.push(Chain::of(after));
                }
                return;
            }
        }
        chains.push(Chain::of(vec![pending]));
    }

    /// The renames not over, chain by chain, each chain in the order its renames were answered.
    pub fn open(&self) -> Vec<Arc<PendingLedger>> {
        self.chains()
            .iter()
            .flat_map(|chain| chain.renames())
            .collect()
    }

    /// Whether a rename not over edits the file with this source key.
    pub fn touches(&self, key: &str) -> bool {
        self.chains().iter().any(|chain| chain.touches(key))
    }

    /// The editor's buffers are as `buffer` answers (the text of a source key, when it is open): what each chain
    /// shows the checks read over.
    pub fn showing(&self, buffer: impl Fn(&str) -> Option<String>) -> Vec<Arc<LedgerOverlay>> {
        self.chains()
            .iter()
            .filter_map(|chain| chain.show(&buffer))
            .collect()
    }

    /// What the chains showed when last read: the ledger changes the checks read over.
    pub fn shown(&self) -> Vec<Arc<LedgerOverlay>> {
        self.chains()
            .iter()
            .filter_map(|chain| chain.shown())
            .collect()
    }

    /// The server is shutting down: every buffer is gone, so what the disk says of each chain lands now.
    pub fn flush(&self, session: &ProjectSession) {
        let chains = self.chains().clone();
        for chain in chains {
            let _ = chain.land(session, &|_| false);
        }
    }
}

/// The buffer each file of a chain had, by source key (`None`: no buffer).
type Buffers = BTreeMap<String, Option<String>>;

/// What an event asks of the pending renames: every chain not over, with the buffer each of its files had, and
/// whether it edits a file the event names, to land once none of its files holds a text the disk does not.
pub struct Landing {
    chains: Vec<(Arc<Chain>, Buffers, bool)>,
}

impl Landing {
    /// No rename waits on anything.
    pub fn none() -> Landing {
        Landing { chains: Vec::new() }
    }

    /// The chains of `pending`, each with the buffer `buffer` answers for each of its files, and whether it edits a
    /// file `keys` names.
    pub fn of(
        pending: &Pending,
        keys: &[String],
        buffer: impl Fn(&str) -> Option<String>,
    ) -> Landing {
        let chains = pending
            .chains()
            .iter()
            .map(|chain| {
                let files = chain
                    .renames()
                    .iter()
                    .flat_map(|r| r.files.iter().chain([&r.declaring]))
                    .map(|file| (file.clone(), buffer(file)))
                    .collect();
                let named = keys.iter().any(|key| chain.touches(key));
                (Arc::clone(chain), files, named)
            })
            .collect();
        Landing { chains }
    }

    /// Whether no chain edits a file the event names.
    pub fn is_empty(&self) -> bool {
        !self.chains.iter().any(|(_, _, named)| *named)
    }

    /// Land each chain the event names none of whose files holds a buffer the disk does not, then show what every
    /// chain shows as the buffers say (a file closed is read from disk now). `true` when the session needs its checks
    /// again.
    pub fn land(&self, session: &mut ProjectSession) -> bool {
        let mut landed = false;
        for (chain, buffers, _) in self.chains.iter().filter(|(_, _, named)| *named) {
            let project = session.project();
            let saved = buffers.iter().all(|(file, buffer)| {
                buffer
                    .as_ref()
                    .is_none_or(|text| on_disk(project, file).as_ref() == Some(text))
            });
            if saved {
                let open = |key: &str| buffers.get(key).is_some_and(Option::is_some);
                landed |= chain.land(session, &open).unwrap_or(false);
            }
        }
        let shown = self
            .chains
            .iter()
            .filter_map(|(chain, buffers, _)| {
                chain.show(&|key| buffers.get(key).cloned().flatten())
            })
            .collect();
        session.overlay_ledger(shown) || landed
    }
}
