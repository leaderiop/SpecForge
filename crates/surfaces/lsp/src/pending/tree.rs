//! The renames made in the editor of an entity and of whatever its moves touch, as one history (ADR 0346 D7): a scope
//! and what it scopes, renamed one after the other, are one history whichever files declare them, and a rename that
//! touches two histories joins them. Each rename hangs under the rename that gave the entity it renames (its path
//! traced through the moves since), the first ones under the entity as it was, so an undo followed by another rename
//! branches and a redo walks back down; each also knows the renames in effect when it was planned, whose paths its
//! moves are written after.
//!
//! What the texts hold is a set of renames in effect (each declaring file holds the renames whose new names it
//! declares, with the renames they came from) and one rename for each stale line; what the ledger on disk holds is the
//! set last landed; what is shown and landed next is the change between them: the renames left undone in the reverse
//! order they were made, those taken up done in the order made, and each stale line whose rename moved. A set in which
//! a rename is in effect without one it was planned after that touches the same paths (an earlier rename undone by
//! hand before a later one it led to) is no state of the history: it shows and lands nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::sync::atomic::Ordering;

use specforge_project::CompiledProject;
use specforge_project::facts::Span;
use specforge_project::harness::{LEDGER_FILE, Ledger, LedgerChange, Line, compose};

use super::reads::reads;
use super::{Key, Parses, PendingLedger, key_of};

/// Whether a rename's edit is in the texts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edit {
    /// Answered, and the declaring file is still the text it was planned on.
    Waiting,
    /// The declaring file was read holding it.
    Applied,
    /// The declaring file changed without its edit: never applied, it is out of the history.
    Dropped,
}

#[derive(Debug)]
struct Node {
    rename: Arc<PendingLedger>,
    /// When it was answered, among every rename the editor holds: the nodes are in this order.
    seq: u64,
    /// The node of the rename that gave the entity this one renames; `None`: the entity before any.
    parent: Option<usize>,
    /// The renames (by `seq`) in effect when this one was planned: its moves are paths after theirs.
    context: BTreeSet<u64>,
    applied: Edit,
}

/// A node of the history, `None` for the entities before any rename.
type At = Option<usize>;

/// What the texts hold: the renames in effect (`None`: no state of the history), and each stale line's node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Reading {
    effect: Option<Vec<usize>>,
    lines: Vec<(Key, At)>,
}

/// What the ledger on disk holds of the history, and the project it was written over.
struct Landed {
    effect: Vec<usize>,
    lines: BTreeMap<Key, At>,
    on: CompiledProject,
}

pub(super) struct Tree {
    nodes: Vec<Node>,
    landed: Landed,
    /// The ledger as the first rename found it: what a stale line goes back to at no rename.
    before: Ledger,
    /// The ledger on disk as the history last left it (as the first rename found it, until a landing): any other
    /// writer that changes what it says of the history's entities or lines ends it.
    left: Ledger,
    /// Each stale line's node in a source file, by the file's text it was read from: a text read again (a keystroke in
    /// another file) is not compared with the renames' texts again. Emptied when the history changes.
    memo: BTreeMap<(Key, String), (String, Option<At>)>,
}

/// Whether one path is the other or under it.
fn related(a: &str, b: &str) -> bool {
    let under = |a: &str, b: &str| a.strip_prefix(b).is_some_and(|rest| rest.starts_with('/'));
    a == b || under(a, b) || under(b, a)
}

/// Every path `rename` moves from or to.
fn paths(rename: &PendingLedger) -> impl Iterator<Item = &str> {
    rename
        .moves
        .iter()
        .flat_map(|(old, new)| [old.as_str(), new.as_str()])
}

/// Whether two renames move related paths: the order they are done in matters.
fn touch(a: &PendingLedger, b: &PendingLedger) -> bool {
    paths(a).any(|x| paths(b).any(|y| related(x, y)))
}

/// Where `path` was before `moves`, by the nearest moved path at or above it.
fn unmoved(moves: &[(String, String)], path: &str) -> String {
    let nearest = moves
        .iter()
        .filter(|(_, new)| related(path, new) && path.len() >= new.len())
        .max_by_key(|(_, new)| new.len());
    match nearest {
        Some((old, new)) => format!("{old}{}", &path[new.len()..]),
        None => path.to_string(),
    }
}

impl Tree {
    /// The history of `rename`, the `seq`-th answered, planned on `before` with the renames `context` in effect.
    pub(super) fn of(
        rename: Arc<PendingLedger>,
        before: &CompiledProject,
        seq: u64,
        context: BTreeSet<u64>,
    ) -> Tree {
        let ledger = Ledger::parse(&String::from_utf8_lossy(&before.root_file(LEDGER_FILE))).ledger;
        Tree {
            left: ledger.clone(),
            before: ledger,
            memo: BTreeMap::new(),
            nodes: vec![Node {
                rename,
                seq,
                parent: None,
                context,
                applied: Edit::Waiting,
            }],
            landed: Landed {
                effect: Vec::new(),
                lines: BTreeMap::new(),
                on: before.clone(),
            },
        }
    }

    /// The renames not dropped, in the order answered.
    pub(super) fn renames(&self) -> impl Iterator<Item = &Arc<PendingLedger>> {
        self.live().map(|i| &self.nodes[i].rename)
    }

    fn live(&self) -> impl DoubleEndedIterator<Item = usize> + '_ {
        (0..self.nodes.len()).filter(|&i| self.nodes[i].applied != Edit::Dropped)
    }

    pub(super) fn is_over(&self) -> bool {
        self.live().next().is_none()
    }

    /// The history is over: nothing of it is left to show or land.
    pub(super) fn end(&mut self) {
        self.nodes.clear();
    }

    /// Whether `rename` moves a path a rename of the history moved (from or to), or one under or above it: it belongs
    /// to this history.
    pub(super) fn touches(&self, rename: &PendingLedger) -> bool {
        self.renames().any(|known| touch(known, rename))
    }

    /// Whether `now`, the ledger on disk, says anything of the history's entities (any path a rename of it moved) or
    /// stale lines other than the history left it saying: another writer (the CLI, MCP, another client, a pull)
    /// renamed or recorded them, and an undo read from the texts would land over that.
    pub(super) fn touched(&self, now: &Ledger) -> bool {
        let ours = |line: &&Line| {
            self.nodes.iter().any(|node| {
                let rename = &node.rename;
                let path = |id: &String| *id == line.subject;
                path(&rename.old_id)
                    || path(&rename.new_id)
                    || rename.moves.iter().any(|(old, new)| path(old) || path(new))
                    || key_of(line).is_some_and(|key| rename.marked(&key).is_some())
            })
        };
        !self.left.lines().filter(ours).eq(now.lines().filter(ours))
    }

    /// The ledger on disk is `now`, as a landing of the editor's renames left it, over `on`.
    pub(super) fn left(&mut self, now: Ledger, on: &CompiledProject) {
        self.left = now;
        self.landed.on = on.clone();
    }

    /// The project the ledger was last written over (the first rename's, until a landing): where files are found.
    pub(super) fn project(&self) -> &CompiledProject {
        &self.landed.on
    }

    /// Whether the texts `text` answers hold node `i`'s rename: its declaring file declares its new name.
    fn held(&self, i: usize, text: &dyn Fn(&str) -> Option<String>, parses: &Parses) -> bool {
        let rename = &self.nodes[i].rename;
        text(&rename.declaring)
            .is_some_and(|text| rename.declares_new(&parses.names(&rename.declaring, &text)))
    }

    /// The renames in effect in the texts `text` answers, by node, ascending: each the declaring files hold, but of
    /// two renames of one entity from one node (a branch) the later only, and the renames each came from.
    fn effect(&self, text: &dyn Fn(&str) -> Option<String>, parses: &Parses) -> Vec<usize> {
        let mut effect: BTreeSet<usize> = BTreeSet::new();
        for i in self.live().rev().filter(|&i| self.held(i, text, parses)) {
            let node = &self.nodes[i];
            let branch = effect.iter().any(|&a| {
                let other = &self.nodes[a];
                a != i && other.parent == node.parent && other.rename.old_id == node.rename.old_id
            });
            if branch {
                continue;
            }
            let mut at = Some(i);
            while let Some(n) = at {
                effect.insert(n);
                at = self.nodes[n].parent;
            }
        }
        effect.into_iter().collect()
    }

    /// The renames (by `seq`) in effect in the texts `text` answers.
    pub(super) fn in_effect(
        &self,
        text: &dyn Fn(&str) -> Option<String>,
        parses: &Parses,
    ) -> Vec<u64> {
        let effect = self.effect(text, parses);
        effect.into_iter().map(|i| self.nodes[i].seq).collect()
    }

    /// Whether `effect` is a state of the history: each rename in it planned after exactly the renames in it before
    /// it that touch the same paths.
    fn holds(&self, effect: &[usize]) -> bool {
        effect.iter().all(|&n| {
            let node = &self.nodes[n];
            self.live().take_while(|&m| m < n).all(|m| {
                let other = &self.nodes[m];
                !touch(&other.rename, &node.rename)
                    || effect.contains(&m) == node.context.contains(&other.seq)
            })
        })
    }

    /// `rename`, the `seq`-th answered, planned on the texts `text` answers with the renames `context` in effect,
    /// hangs under the rename in effect there that gave the entity it renames (its path traced back through the moves
    /// in effect since), else under the entities as they were.
    pub(super) fn push(
        &mut self,
        rename: Arc<PendingLedger>,
        seq: u64,
        context: BTreeSet<u64>,
        text: &dyn Fn(&str) -> Option<String>,
        parses: &Parses,
    ) {
        let mut id = rename.old_id.clone();
        let mut parent = None;
        for i in self.effect(text, parses).into_iter().rev() {
            let known = &self.nodes[i].rename;
            if known.new_id == id {
                parent = Some(i);
                break;
            }
            id = unmoved(&known.moves, &id);
        }
        self.nodes.push(Node {
            rename,
            seq,
            parent,
            context,
            applied: Edit::Waiting,
        });
        self.memo.clear();
    }

    /// What is left of this history once taken: nothing.
    pub(super) fn take(&mut self) -> Tree {
        Tree {
            nodes: std::mem::take(&mut self.nodes),
            landed: Landed {
                effect: std::mem::take(&mut self.landed.effect),
                lines: std::mem::take(&mut self.landed.lines),
                on: self.landed.on.clone(),
            },
            before: std::mem::take(&mut self.before),
            left: std::mem::take(&mut self.left),
            memo: BTreeMap::new(),
        }
    }

    /// `other` joins this history, its renames in the order answered among these, what it landed beside what this
    /// one did; the ledger on disk is `left`. The project and the ledger before are the elder history's.
    pub(super) fn merge(&mut self, other: Tree, left: Ledger) {
        let elder = |tree: &Tree| tree.nodes.first().map_or(u64::MAX, |n| n.seq);
        let (old, young) = if elder(&other) < elder(self) {
            (other, self.take())
        } else {
            (self.take(), other)
        };
        let offset = old.nodes.len();
        let mut nodes: Vec<(usize, Node)> = old.nodes.into_iter().enumerate().collect();
        nodes.extend(
            young
                .nodes
                .into_iter()
                .enumerate()
                .map(|(i, n)| (i + offset, n)),
        );
        nodes.sort_by_key(|(_, node)| node.seq);
        let mut index = vec![0; nodes.len()];
        for (at, (was, _)) in nodes.iter().enumerate() {
            index[*was] = at;
        }
        let young_at = |i: usize| index[i + offset];
        let mut effect: Vec<usize> = old.landed.effect.iter().map(|&i| index[i]).collect();
        effect.extend(young.landed.effect.iter().map(|&i| young_at(i)));
        effect.sort_unstable();
        let mut lines: BTreeMap<Key, At> = old
            .landed
            .lines
            .into_iter()
            .map(|(key, at)| (key, at.map(|i| index[i])))
            .collect();
        for (key, at) in young.landed.lines {
            lines.entry(key).or_insert(at.map(young_at));
        }
        let mut before = old.before;
        let known: BTreeSet<Key> = before.lines().filter_map(key_of).collect();
        for line in young.before.lines() {
            if key_of(line).is_some_and(|key| !known.contains(&key)) {
                before.insert(line.clone());
            }
        }
        self.nodes = nodes
            .into_iter()
            .map(|(was, mut node)| {
                let shift = |i: usize| if was < offset { index[i] } else { young_at(i) };
                node.parent = node.parent.map(shift);
                node
            })
            .collect();
        self.landed = Landed {
            effect,
            lines,
            on: old.landed.on,
        };
        self.before = before;
        self.left = left;
        self.memo.clear();
    }

    /// The node of the source `file`'s `text` for the stale line `key`: the last rename whose name for it reads at the
    /// places it wrote it in the file, else the last marking it in effect when the first whose name read still reads
    /// at its places was planned, else none (the name changed to one no rename wrote). Whatever else changed in the
    /// file (another entity renamed, typing) does not move it. `None` when no rename marking the line rewrote the file.
    fn source_node(&self, key: &Key, file: &str, text: Option<&str>) -> Option<At> {
        let marked = |i: usize| {
            let rename = &self.nodes[i].rename;
            Some((rename.marked(key)?, rename.sources.get(file)?))
        };
        let marking: Vec<usize> = self.live().filter(|&i| marked(i).is_some()).collect();
        if marking.is_empty() {
            return None;
        }
        let Some(text) = text else { return Some(None) };
        let spans = |i: usize, read: bool| -> Vec<Span> {
            let (m, _) = marked(i).expect("marking");
            let at = m.places.iter().filter(|p| p.file == file);
            at.map(|p| if read { p.read } else { p.wrote }).collect()
        };
        let wrote = marking.iter().rev().copied().find(|&i| {
            let (m, (_, wrote)) = marked(i).expect("marking");
            reads(wrote, text, &spans(i, false), &m.wrote)
        });
        if let Some(i) = wrote {
            return Some(Some(i));
        }
        let read = marking.iter().copied().find(|&i| {
            let (m, (read, _)) = marked(i).expect("marking");
            reads(read, text, &spans(i, true), &m.read)
        });
        Some(read.and_then(|i| self.history(i, key).into_iter().rev().nth(1)))
    }

    /// Node `i` and the renames marking `key` that were in effect when it was planned, ascending.
    fn history(&self, i: usize, key: &Key) -> Vec<usize> {
        let context = &self.nodes[i].context;
        self.live()
            .take_while(|&m| m < i)
            .filter(|&m| context.contains(&self.nodes[m].seq))
            .filter(|&m| self.nodes[m].rename.line(key).is_some())
            .chain([i])
            .collect()
    }

    /// What the texts `text` answers hold. A rename waiting for its edit is applied once its declaring file holds it,
    /// and dropped once that file is another text than it was planned on without it. A stale line is at the latest
    /// rename every file it is about holds or came after (one line names one name for every file); with no file a
    /// rename marking it rewrote, at the last rename in effect that marks it.
    pub(super) fn read(
        &mut self,
        text: &dyn Fn(&str) -> Option<String>,
        parses: &Parses,
    ) -> Reading {
        for i in 0..self.nodes.len() {
            let node = &self.nodes[i];
            if node.applied != Edit::Waiting {
                continue;
            }
            if node.rename.planned.as_deref() == text(&node.rename.declaring).as_deref() {
                continue;
            }
            self.nodes[i].applied = if self.held(i, text, parses) {
                Edit::Applied
            } else {
                Edit::Dropped
            };
            self.memo.clear();
        }
        let effect = self.effect(text, parses);
        let mut keys: Vec<(Key, Vec<String>)> = Vec::new();
        for marked in self.renames().flat_map(|r| &r.stale) {
            let Some(key) = key_of(&marked.line) else {
                continue;
            };
            let files = marked.files().into_iter().map(str::to_string);
            match keys.iter_mut().find(|(k, _)| *k == key) {
                Some((_, known)) => known.extend(files),
                None => keys.push((key, files.collect())),
            }
        }
        let mut lines = Vec::new();
        for (key, files) in keys {
            let mut held: Vec<At> = Vec::new();
            for file in files {
                let now = text(&file);
                let memo = (key.clone(), file);
                let node = match (self.memo.get(&memo), &now) {
                    (Some((then, node)), Some(now)) if then == now => *node,
                    _ => {
                        let node = self.source_node(&key, &memo.1, now.as_deref());
                        if let Some(now) = now {
                            self.memo.insert(memo, (now, node));
                        }
                        node
                    }
                };
                held.extend(node);
            }
            let at = if held.is_empty() {
                let marks = |i: &&usize| self.nodes[**i].rename.line(&key).is_some();
                effect.iter().rfind(marks).copied()
            } else {
                self.earliest(&held, &key)
            };
            lines.push((key, at));
        }
        let effect = self.holds(&effect).then_some(effect);
        Reading { effect, lines }
    }

    /// The latest rename marking `key` that each of `held` is or came after; none when one is none.
    fn earliest(&self, held: &[At], key: &Key) -> At {
        let history = |at: At| -> BTreeSet<usize> {
            at.map(|i| self.history(i, key).into_iter().collect())
                .unwrap_or_default()
        };
        let mut common = history(held[0]);
        for at in &held[1..] {
            let other = history(*at);
            common.retain(|i| other.contains(i));
        }
        common.last().copied()
    }

    /// What `reading` changes of the ledger as last landed, and the project to read that change over: the moves of the
    /// renames left undone (each undone, the last made first) and taken up (in the order made), composed (a name
    /// renamed back moves nothing), and each stale line whose node moved, as that node's rename wrote it or, at no
    /// rename, as the ledger held it before the history. Nothing when the reading is no state of the history.
    pub(super) fn change(&self, reading: &Reading) -> (LedgerChange, &CompiledProject) {
        let landed = &self.landed;
        let Some(effect) = &reading.effect else {
            return (LedgerChange::default(), &landed.on);
        };
        let mut moves = Vec::new();
        for i in landed.effect.iter().rev().filter(|i| !effect.contains(i)) {
            for (old, new) in &self.nodes[*i].rename.moves {
                compose(&mut moves, new, old);
            }
        }
        for i in effect.iter().filter(|i| !landed.effect.contains(i)) {
            for (old, new) in &self.nodes[*i].rename.moves {
                compose(&mut moves, old, new);
            }
        }
        moves.retain(|(old, new)| old != new);
        let mut stale = Vec::new();
        for (key, at) in &reading.lines {
            if landed.lines.get(key).copied().flatten() != *at {
                stale.extend(self.line(key, *at));
            }
        }
        for (key, at) in &landed.lines {
            if at.is_some() && !reading.lines.iter().any(|(k, _)| k == key) {
                stale.extend(self.line(key, None));
            }
        }
        let change = LedgerChange {
            moves,
            stale,
            ..LedgerChange::default()
        };
        (change, &landed.on)
    }

    /// The stale line for `key` at `at`: its rename's, or at no rename, the ledger's line before the history, else one
    /// writing the name held back (which drops the line).
    fn line(&self, key: &Key, at: At) -> Option<Line> {
        if let Some(i) = at {
            return self.nodes[i].rename.line(key).cloned();
        }
        if let Some(line) = self
            .before
            .lines()
            .find(|l| key_of(l).as_ref() == Some(key))
        {
            return Some(line.clone());
        }
        let line = self.nodes.iter().find_map(|n| n.rename.line(key))?;
        let stale = line.stale_of()?;
        let held = format!(
            "{}{}",
            &line.key[..line.key.len() - stale.now.len()],
            stale.held
        );
        Some(Line {
            subject: line.subject.clone(),
            key: held,
            digest: String::new(),
        })
    }

    /// `reading`, read from the disk, landed: the ledger holds it now, written over the project a write left
    /// ([`Tree::left`]), or over the one it was when the change wrote nothing (the ledger's paths are that project's
    /// still). A reading that is no state of the history landed nothing, and the ledger holds what it did.
    pub(super) fn landed(&mut self, reading: Reading) {
        let Some(effect) = reading.effect else {
            return;
        };
        self.landed.effect = effect;
        self.landed.lines = reading.lines.into_iter().collect();
    }

    /// Mark the rename the texts hold last as showing when `shows`, every other one not.
    pub(super) fn mark(&self, reading: &Reading, shows: bool) {
        let top = reading
            .effect
            .as_ref()
            .and_then(|effect| effect.last().copied())
            .or_else(|| reading.lines.iter().filter_map(|(_, at)| *at).max());
        for (i, node) in self.nodes.iter().enumerate() {
            let showing = shows && Some(i) == top;
            node.rename.showing.store(showing, Ordering::Relaxed);
        }
    }
}
