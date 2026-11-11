//! The ledger change of an LSP rename (ADR 0289). The server writes no source: a rename answers edits the editor
//! applies to its buffers. A project that declares a harness field keeps a ledger line per harness value, keyed by the
//! entity's path, and a rename marks the fact stores whose names it rewrites in source files stale (ADR 0346 D7), so
//! the rename's ledger change is held here until it lands on disk.
//!
//! The renames of an entity and of whatever their moves touch are one **history** (`tree`), whichever files declare
//! them: a rename moving a path a history's renames moved (or one under or above it) joins it, and one touching two
//! histories merges them. Each rename hangs under the rename that gave the entity it renames, so an undo then another
//! rename branches and a redo walks back down. What it shows and lands is never kept as a phase: it is read from the
//! texts each time, so whatever the editor did to its buffers (undo in one buffer and not another, part way, by hand,
//! redo, before a save or after) the ledger read is what the texts say:
//!
//! - the **moves** between the renames in effect and those the ledger holds: a rename is in effect while its
//!   declaring file declares its new name (counted: two scoped entities may share a name), with the renames it came
//!   from; texts in which a rename is in effect without one it was planned after (an earlier rename undone by hand
//!   before a later one it led to) are no state of the history, and show and land nothing;
//! - for each **stale line**, by store, field and name held: the line of the last rename every source file it is
//!   about holds, a file holding a rename while the name it wrote reads at its places there, whatever else changed in
//!   the file (`reads`); none once the name went back or was typed over.
//!
//! The checks read, over the session, what the buffers say (a file no buffer holds, as on disk), as a change from what
//! the ledger on disk holds of the history. Once no file of it holds a buffer the disk does not (each one saved, or
//! closed), what the disk says lands under the write lock; a write that fails leaves it to the next event. A history is
//! kept while any of its files is open, so a redo or an undo saved later lands what it says; with every file closed it
//! is over. It is kept only while the ledger on disk reads as it left it: another writer (the CLI, MCP, another
//! client, a pull) that changes what it says of its entities or stale lines ends it. A rename answered and never
//! applied waits while its declaring file is the text it was planned on, and is dropped at the first other text
//! without its edit: a name typed by hand later moves nothing.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use specforge_ops::asking::Asking;
use specforge_ops::authoring::apply::{Origin, Surface, land_ledger};
use specforge_ops::authoring::rename::Marked;
use specforge_ops::{Caller, Containment};
use specforge_project::harness::{LEDGER_FILE, Ledger, LedgerChange, LedgerOverlay, Line};
use specforge_project::{CompiledProject, ProjectSession};

mod reads;
mod tree;

use tree::{Reading, Tree};

/// One rename's ledger change, and the texts it read and wrote: what reading the texts and landing need, no project.
pub struct PendingLedger {
    /// The declaring file's text the rename was planned on.
    planned: Option<Arc<str>>,
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
    /// The rename is the last of its tree the texts hold, and the tree shows a change.
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
        let planned = before.source_text(&renamed.declaring);
        let declared = planned.as_deref().map_or((1, 0), |text| {
            declared(
                text,
                &renamed.declaring,
                &renamed.old_name,
                &renamed.new_name,
            )
        });
        Some(Arc::new(PendingLedger {
            planned,
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
            showing: AtomicBool::new(false),
        }))
    }

    /// Whether the rename's tree shows a change, and this rename is the last of it the texts hold.
    pub fn is_showing(&self) -> bool {
        self.showing.load(Ordering::Relaxed)
    }

    /// Whether `text`, the declaring file, declares the new name: one at least with it, and one entity fewer with the
    /// old name than when the rename was planned, or one more with the new (an entity declared by hand under the old
    /// name beside it takes nothing of the rename). Counted, not asked whether a name is declared: two scoped
    /// entities may share a name, and a later rename of the other one moves the count of a name this one wrote.
    fn declares_new(&self, names: &Names) -> bool {
        let count = |name: &str| names.get(name).copied().unwrap_or(0);
        let (old, new) = (count(&self.old_name), count(&self.new_name));
        new >= 1 && (old + 1 == self.declared.0 || new == self.declared.1 + 1)
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

/// How many entities a declaring file declares with each name.
type Names = BTreeMap<String, usize>;

/// The names `text` (the source `key`) declares, each with how many entities it declares with it: parsed once.
fn names(text: &str, key: &str) -> Names {
    let mut names = Names::new();
    for entity in specforge_parser::parse(text, key).entities {
        *names.entry(entity.id.raw.to_string()).or_default() += 1;
    }
    names
}

/// The names each declaring file declares, parsed once per text for every tree one reading reads.
#[derive(Default)]
struct Parses(RefCell<BTreeMap<String, (String, Rc<Names>)>>);

impl Parses {
    /// The names `text`, the declaring file `key`, declares.
    fn names(&self, key: &str, text: &str) -> Rc<Names> {
        let mut parsed = self.0.borrow_mut();
        if let Some((then, names)) = parsed.get(key)
            && then == text
        {
            return Rc::clone(names);
        }
        let read = Rc::new(names(text, key));
        parsed.insert(key.to_string(), (text.to_string(), Rc::clone(&read)));
        read
    }
}

/// How many entities `text` (the source `key`) declares named `old` and named `new`.
fn declared(text: &str, key: &str, old: &str, new: &str) -> (usize, usize) {
    let names = names(text, key);
    let count = |name: &str| names.get(name).copied().unwrap_or(0);
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

/// The ledger on disk, at `project`'s root.
fn ledger_on_disk(project: &CompiledProject) -> Ledger {
    Ledger::parse(&String::from_utf8_lossy(&project.root_file(LEDGER_FILE))).ledger
}

/// The text of the file `key` on disk, found through `project`'s sources.
fn on_disk(project: &CompiledProject, key: &str) -> Option<String> {
    std::fs::read_to_string(project.environment().sources().path(key)).ok()
}

/// A tree of renames, and the overlay last shown with the reading it was made of: the same reading shows the same
/// overlay, so the session's checks do not run again for nothing.
struct Links {
    tree: Tree,
    shown: Option<(Reading, Arc<LedgerOverlay>)>,
    /// The disk text of each file of the tree no buffer holds, read once and again only after an event names it.
    disk: BTreeMap<String, Option<String>>,
    /// The ledger on disk was read since an event last named it, and reads as the tree left it.
    ledger: bool,
}

/// The renames of one entity.
struct Chain(Mutex<Links>);

impl std::fmt::Debug for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.renames()).finish()
    }
}

impl Chain {
    fn of(
        rename: Arc<PendingLedger>,
        before: &CompiledProject,
        seq: u64,
        context: BTreeSet<u64>,
    ) -> Arc<Chain> {
        Arc::new(Chain(Mutex::new(Links {
            tree: Tree::of(rename, before, seq, context),
            shown: None,
            disk: BTreeMap::new(),
            ledger: true,
        })))
    }

    fn links(&self) -> MutexGuard<'_, Links> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn is_over(&self) -> bool {
        self.links().tree.is_over()
    }

    fn renames(&self) -> Vec<Arc<PendingLedger>> {
        self.links().tree.renames().cloned().collect()
    }

    /// The key of every file a rename of the tree edits.
    fn files(&self) -> Vec<String> {
        let links = self.links();
        let mut files: Vec<String> = links
            .tree
            .renames()
            .flat_map(|r| r.files.iter().chain([&r.declaring]))
            .cloned()
            .collect();
        files.sort();
        files.dedup();
        files
    }

    /// Whether a rename of the tree edits the file `key`.
    fn touches(&self, key: &str) -> bool {
        self.links()
            .tree
            .renames()
            .any(|r| r.files.iter().any(|file| file == key))
    }

    /// The overlay last shown.
    fn shown(&self) -> Option<Arc<LedgerOverlay>> {
        let links = self.links();
        links.shown.as_ref().map(|(_, overlay)| Arc::clone(overlay))
    }

    /// Whether `rename` moves a path a rename of this history moved, or one under or above it.
    fn touches_rename(&self, rename: &PendingLedger) -> bool {
        self.links().tree.touches(rename)
    }

    /// The renames (by answer order) in effect in the texts `text` answers.
    fn in_effect(&self, text: &dyn Fn(&str) -> Option<String>, parses: &Parses) -> Vec<u64> {
        self.links().tree.in_effect(text, parses)
    }

    /// `rename` joins this history ([`Tree::push`]).
    fn push(
        &self,
        rename: Arc<PendingLedger>,
        seq: u64,
        context: BTreeSet<u64>,
        text: &dyn Fn(&str) -> Option<String>,
        parses: &Parses,
    ) {
        let mut links = self.links();
        links.tree.push(rename, seq, context, text, parses);
        links.shown = None;
    }

    /// `other`'s history joins this one ([`Tree::merge`]), the ledger on disk as `project`'s root holds it; `other` is
    /// over.
    fn merge(&self, other: &Chain, project: &CompiledProject) {
        let taken = other.links().tree.take();
        let mut links = self.links();
        links.tree.merge(taken, ledger_on_disk(project));
        links.shown = None;
        links.disk.clear();
    }

    /// The files `keys` names changed on disk: read them again when next needed.
    fn forget(&self, keys: &[String]) {
        let mut links = self.links();
        for key in keys {
            links.disk.remove(key);
            links.ledger &= key != LEDGER_FILE;
        }
    }

    /// Whether the ledger on disk reads as the tree last left it ([`Tree::touched`]); when another writer touched
    /// what it holds, the tree is over.
    fn holds(links: &mut Links, project: &CompiledProject) -> bool {
        if links.tree.touched(&ledger_on_disk(project)) {
            links.tree.end();
            links.shown = None;
            return false;
        }
        links.ledger = true;
        true
    }

    /// The ledger on disk reads as the tree last left it, else the tree is over: before any landing of an event, so a
    /// landing of another tree does not hide another writer's.
    fn check(&self, project: &CompiledProject) {
        let mut links = self.links();
        if !links.tree.is_over() {
            Chain::holds(&mut links, project);
        }
    }

    /// The editor landed a tree: the ledger on disk is what it wrote, as this tree left it too.
    fn rebase(&self, project: &CompiledProject) {
        let mut links = self.links();
        links.tree.left(ledger_on_disk(project), project);
        links.ledger = true;
    }

    /// What the tree shows over the session when its files read as `text` answers (a file it does not answer, as on
    /// disk, read once): the change from what the ledger holds of it; `None` when that is no change.
    fn show(
        &self,
        text: &dyn Fn(&str) -> Option<String>,
        parses: &Parses,
    ) -> Option<Arc<LedgerOverlay>> {
        let files = self.files();
        let mut links = self.links();
        if links.tree.is_over() {
            return None;
        }
        let project = links.tree.project().clone();
        if !links.ledger && !Chain::holds(&mut links, &project) {
            return None;
        }
        let mut texts: BTreeMap<String, Option<String>> = BTreeMap::new();
        for file in files {
            let text = match text(&file) {
                Some(buffer) => Some(buffer),
                None => links
                    .disk
                    .entry(file.clone())
                    .or_insert_with(|| on_disk(&project, &file))
                    .clone(),
            };
            texts.insert(file, text);
        }
        let reading = links
            .tree
            .read(&|key| texts.get(key).cloned().flatten(), parses);
        let (change, on) = links.tree.change(&reading);
        links.tree.mark(&reading, !is_empty(&change));
        if is_empty(&change) {
            links.shown = None;
            return None;
        }
        if let Some((shown, overlay)) = &links.shown
            && *shown == reading
        {
            return Some(Arc::clone(overlay));
        }
        let overlay = Arc::new(LedgerOverlay::new(on.clone(), change));
        links.shown = Some((reading, Arc::clone(&overlay)));
        Some(overlay)
    }

    /// Land what the disk says of the tree, over the project the session shows: the change from what the ledger holds
    /// of it. `Ok(true)` when the ledger was written; `Err` when the write failed, the tree left as it was. The tree is
    /// kept while `open` says a file of it is open, and is over when none is.
    fn land(
        &self,
        session: &ProjectSession,
        open: &dyn Fn(&str) -> bool,
        parses: &Parses,
    ) -> Result<bool, ()> {
        let files = self.files();
        let mut links = self.links();
        if links.tree.is_over() {
            return Ok(false);
        }
        let project = session.project();
        if !Chain::holds(&mut links, project) {
            return Ok(false);
        }
        let reading = links.tree.read(&|key| on_disk(project, key), parses);
        let (change, on) = links.tree.change(&reading);
        let written = !is_empty(&change);
        if written {
            let caller = Caller::new(&UNBOUNDED, Asking::Nobody);
            let origin = Origin::Core {
                surface: Surface::Lsp,
            };
            land_ledger(on, project, change, &origin, caller).map_err(drop)?;
            links.tree.left(ledger_on_disk(project), project);
        }
        links.tree.landed(reading.clone());
        links.tree.mark(&reading, false);
        links.shown = None;
        if !files.iter().any(|file| open(file)) {
            links.tree.end();
        }
        Ok(written)
    }
}

fn is_empty(change: &LedgerChange) -> bool {
    change.moves.is_empty() && change.stale.is_empty()
}

static UNBOUNDED: Containment = Containment::unbounded();

/// The pending renames of a state: their trees, in the order they were begun.
#[derive(Debug, Default)]
pub struct Pending {
    chains: Mutex<Vec<Arc<Chain>>>,
    /// How many renames were held: each one's place in the order answered.
    answered: AtomicU64,
}

impl Pending {
    /// The trees not over.
    fn chains(&self) -> MutexGuard<'_, Vec<Arc<Chain>>> {
        let mut chains = self.chains.lock().unwrap_or_else(|e| e.into_inner());
        chains.retain(|chain| !chain.is_over());
        chains
    }

    /// Hold `rename`, planned on `before`: it joins every history whose renames moved a path it moves (or one under or
    /// above it), merged into one, else begins one over `before`. It knows the renames in effect as it was planned,
    /// in every history ([`Tree::push`]). A history the ledger on disk no longer reads as it left is over first.
    pub fn hold(&self, rename: Arc<PendingLedger>, before: &CompiledProject) {
        let mut chains = self.chains();
        let seq = self.answered.fetch_add(1, Ordering::Relaxed);
        let text = |key: &str| before.source_text(key).map(|text| text.to_string());
        let parses = Parses::default();
        let context: BTreeSet<u64> = chains
            .iter()
            .flat_map(|chain| chain.in_effect(&text, &parses))
            .collect();
        let touching: Vec<Arc<Chain>> = chains
            .iter()
            .filter(|chain| chain.touches_rename(&rename))
            .filter(|chain| {
                chain.check(before);
                !chain.is_over()
            })
            .cloned()
            .collect();
        match touching.split_first() {
            None => chains.push(Chain::of(rename, before, seq, context)),
            Some((first, rest)) => {
                for other in rest {
                    first.merge(other, before);
                }
                first.push(rename, seq, context, &text, &parses);
                chains.retain(|chain| !chain.is_over());
            }
        }
    }

    /// The renames not over and not dropped, tree by tree, each in the order answered.
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

    /// The editor's buffers are as `buffer` answers (the text of a source key, when it is open): what each tree shows
    /// the checks read over.
    pub fn showing(&self, buffer: impl Fn(&str) -> Option<String>) -> Vec<Arc<LedgerOverlay>> {
        let parses = Parses::default();
        self.chains()
            .iter()
            .filter_map(|chain| chain.show(&buffer, &parses))
            .collect()
    }

    /// What the trees showed when last read: the ledger changes the checks read over.
    pub fn shown(&self) -> Vec<Arc<LedgerOverlay>> {
        self.chains()
            .iter()
            .filter_map(|chain| chain.shown())
            .collect()
    }

    /// The server is shutting down: every buffer is gone, so what the disk says of each tree lands now.
    pub fn flush(&self, session: &ProjectSession) {
        let chains = self.chains().clone();
        let parses = Parses::default();
        for chain in chains {
            let _ = chain.land(session, &|_| false, &parses);
        }
    }
}

/// The buffer each file of a tree had, by source key (`None`: no buffer).
type Buffers = BTreeMap<String, Option<String>>;

/// What an event asks of the pending renames: every tree not over, with the buffer each of its files had, and
/// whether it edits a file the event names, to land once none of its files holds a text the disk does not.
pub struct Landing {
    chains: Vec<(Arc<Chain>, Buffers, bool)>,
    /// The files the event names: read from disk again.
    keys: Vec<String>,
}

impl Landing {
    /// No rename waits on anything.
    pub fn none() -> Landing {
        Landing {
            chains: Vec::new(),
            keys: Vec::new(),
        }
    }

    /// The trees of `pending`, each with the buffer `buffer` answers for each of its files, and whether it edits a
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
                    .files()
                    .into_iter()
                    .map(|file| {
                        let text = buffer(&file);
                        (file, text)
                    })
                    .collect();
                let named = keys.iter().any(|key| chain.touches(key));
                (Arc::clone(chain), files, named)
            })
            .collect();
        Landing {
            chains,
            keys: keys.to_vec(),
        }
    }

    /// Whether no tree edits a file the event names.
    pub fn is_empty(&self) -> bool {
        !self.chains.iter().any(|(_, _, named)| *named)
    }

    /// Land each tree the event names none of whose files holds a buffer the disk does not, then show what every
    /// tree shows as the buffers say (a file closed is read from disk now). `true` when the session needs its checks
    /// again.
    pub fn land(&self, session: &mut ProjectSession) -> bool {
        let mut landed = false;
        let parses = Parses::default();
        for (chain, _, _) in &self.chains {
            chain.forget(&self.keys);
            chain.check(session.project());
        }
        for (chain, buffers, _) in self.chains.iter().filter(|(_, _, named)| *named) {
            let project = session.project();
            let saved = buffers.iter().all(|(file, buffer)| {
                buffer
                    .as_ref()
                    .is_none_or(|text| on_disk(project, file).as_ref() == Some(text))
            });
            if saved {
                let open = |key: &str| buffers.get(key).is_some_and(Option::is_some);
                if chain.land(session, &open, &parses).unwrap_or(false) {
                    landed = true;
                    for (other, _, _) in &self.chains {
                        other.rebase(session.project());
                    }
                }
            }
        }
        let shown = self
            .chains
            .iter()
            .filter_map(|(chain, buffers, _)| {
                chain.show(&|key| buffers.get(key).cloned().flatten(), &parses)
            })
            .collect();
        session.overlay_ledger(shown) || landed
    }
}
