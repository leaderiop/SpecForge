//! The renames of one entity made in the editor, as a tree (ADR 0346 D7): each rename hangs under the rename that gave
//! the entity it renames (by path), the first ones under the entity as it was. An undo followed by another rename
//! branches; a redo walks back down. What the texts hold is a node of the tree (the declaring file's, and one for each
//! stale line); what the ledger on disk holds is the node last landed; what is shown and landed next is the path
//! between them: the moves of the renames walked back undone, those walked down done, and each stale line whose node
//! moved, as that node's rename wrote it or as the ledger held it before the tree.

use std::collections::BTreeMap;
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
    /// The declaring file changed without its edit: never applied, it is out of the tree.
    Dropped,
}

#[derive(Debug)]
struct Node {
    rename: Arc<PendingLedger>,
    /// The node of the rename that gave the entity this one renames; `None`: the entity before any.
    parent: Option<usize>,
    applied: Edit,
}

/// A node of the tree, `None` for the entity before any rename.
type At = Option<usize>;

/// What the texts hold: the declaring file's node, and each stale line's node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Reading {
    declared: At,
    lines: Vec<(Key, At)>,
}

/// What the ledger on disk holds of the tree, and the project it was written over.
struct Landed {
    declared: At,
    lines: BTreeMap<Key, At>,
    on: CompiledProject,
}

pub(super) struct Tree {
    nodes: Vec<Node>,
    landed: Landed,
    /// The ledger as the first rename found it: what a stale line goes back to at the entity before any rename.
    before: Ledger,
    /// Each stale line's node in a source file, by the file's text it was read from: a text read again (a keystroke in
    /// another file) is not compared with the renames' texts again. Emptied when the tree changes.
    memo: BTreeMap<(Key, String), (String, Option<At>)>,
}

impl Tree {
    pub(super) fn of(rename: Arc<PendingLedger>, before: &CompiledProject) -> Tree {
        let on = before.clone();
        let before = Ledger::parse(&String::from_utf8_lossy(&on.root_file(LEDGER_FILE))).ledger;
        Tree {
            before,
            memo: BTreeMap::new(),
            nodes: vec![Node {
                rename,
                parent: None,
                applied: Edit::Waiting,
            }],
            landed: Landed {
                declared: None,
                lines: BTreeMap::new(),
                on,
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

    /// The tree is over: nothing of it is left to show or land.
    pub(super) fn end(&mut self) {
        self.nodes.clear();
    }

    pub(super) fn declaring(&self) -> &str {
        &self.nodes[0].rename.declaring
    }

    /// The project the ledger was last written over (the first rename's, until a landing): where files are found.
    pub(super) fn project(&self) -> &CompiledProject {
        &self.landed.on
    }

    /// The node the declaring file's `text` holds: the last rename answered whose new name it declares.
    fn declared(&self, text: Option<&str>, parses: &Parses) -> At {
        let names = parses.names(self.declaring(), text?);
        self.live()
            .rev()
            .find(|&i| self.nodes[i].rename.declares_new(&names))
    }

    /// The path of the entity at `at`.
    fn id(&self, at: At) -> &str {
        match at {
            Some(i) => &self.nodes[i].rename.new_id,
            None => &self.nodes[0].rename.old_id,
        }
    }

    /// `rename`, planned on a project whose declaring file holds the node that gave the entity it renames, or one
    /// whose path moved something `rename` moves (a scope and what it scopes, renamed one after the other), hangs under
    /// that node: one history, landed as one change. `false` when it renames something else.
    pub(super) fn join(&mut self, rename: &Arc<PendingLedger>) -> bool {
        if self.is_over() || self.declaring() != rename.declaring {
            return false;
        }
        let at = self.declared(rename.planned.as_deref(), &Parses::default());
        if self.id(at) != rename.old_id && !self.moved(at, rename) {
            return false;
        }
        self.nodes.push(Node {
            rename: Arc::clone(rename),
            parent: at,
            applied: Edit::Waiting,
        });
        self.memo.clear();
        true
    }

    /// Whether `rename` moves a path the entities at `at` are at: one the renames above it moved to, or at the entity
    /// before any, one the first renames moved from.
    fn moved(&self, at: At, rename: &PendingLedger) -> bool {
        let paths: Vec<&str> = match at {
            Some(_) => self
                .up(at)
                .into_iter()
                .flat_map(|i| &self.nodes[i].rename.moves)
                .map(|(_, new)| new.as_str())
                .collect(),
            None => self
                .live()
                .filter(|&i| self.nodes[i].parent.is_none())
                .flat_map(|i| &self.nodes[i].rename.moves)
                .map(|(old, _)| old.as_str())
                .collect(),
        };
        rename
            .moves
            .iter()
            .any(|(old, _)| paths.contains(&old.as_str()))
    }

    /// `at` and the nodes above it, from `at` up.
    fn up(&self, mut at: At) -> Vec<usize> {
        let mut path = Vec::new();
        while let Some(i) = at {
            path.push(i);
            at = self.nodes[i].parent;
        }
        path
    }

    /// The deepest node above (or at) both `a` and `b`.
    fn meet(&self, a: At, b: At) -> At {
        let above_b = self.up(b);
        self.up(a).into_iter().find(|i| above_b.contains(i))
    }

    /// The node of the source `file`'s `text` for the stale line `key`: the last rename whose name for it reads at the
    /// places it wrote it in the file, else the node above the first whose name read still reads at its places, else
    /// the entity before any (the name changed to one no rename wrote). Whatever else changed in the file (another
    /// entity renamed, typing) does not move it. `None` when no rename marking the line rewrote the file.
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
        Some(read.and_then(|i| self.nodes[i].parent))
    }

    /// What the texts `text` answers hold. A rename waiting for its edit is applied once the declaring file holds it,
    /// and dropped once the declaring file is another text than it was planned on without it. A stale line is at the
    /// deepest node every file it is about holds is under (one line names one name for every file); with no file a
    /// rename marking it rewrote, at the declaring file's node; and its line is that of the nearest rename at or above
    /// that node that marks it.
    pub(super) fn read(
        &mut self,
        text: &dyn Fn(&str) -> Option<String>,
        parses: &Parses,
    ) -> Reading {
        let declaring = text(self.declaring());
        let declared = self.declared(declaring.as_deref(), parses);
        for i in 0..self.nodes.len() {
            let node = &self.nodes[i];
            if node.applied != Edit::Waiting {
                continue;
            }
            if node.rename.planned.as_deref() == declaring.as_deref() {
                continue;
            }
            self.nodes[i].applied = if declared == Some(i) {
                Edit::Applied
            } else {
                Edit::Dropped
            };
            self.memo.clear();
        }
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
            let at = match held.split_first() {
                Some((first, rest)) => rest.iter().fold(*first, |a, b| self.meet(a, *b)),
                None => declared,
            };
            let line = self
                .up(at)
                .into_iter()
                .find(|&i| self.nodes[i].rename.line(&key).is_some());
            lines.push((key, line));
        }
        Reading { declared, lines }
    }

    /// What `reading` changes of the ledger as last landed, and the project to read that change over: the moves of
    /// the renames walked back (each undone) and walked down to the declaring file's node, composed (a name renamed
    /// back moves nothing), and each stale line whose node moved, as that node's rename wrote it or, at the entity
    /// before any rename, as the ledger held it before the tree.
    pub(super) fn change(&self, reading: &Reading) -> (LedgerChange, &CompiledProject) {
        let landed = &self.landed;
        let meet = self.meet(landed.declared, reading.declared);
        let back = self.up(landed.declared);
        let back = back.iter().take_while(|&&i| Some(i) != meet);
        let mut down = self.up(reading.declared);
        down.retain(|&i| !self.up(meet).contains(&i));
        let mut moves = Vec::new();
        for (old, new) in back.flat_map(|&i| &self.nodes[i].rename.moves) {
            compose(&mut moves, new, old);
        }
        for (old, new) in down.iter().rev().flat_map(|&i| &self.nodes[i].rename.moves) {
            compose(&mut moves, old, new);
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

    /// The stale line for `key` at `at`: its rename's, or at the entity before any rename, the ledger's line before the
    /// tree, else one writing the name held back (which drops the line).
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

    /// `reading`, read from the disk, was written over `on`: the ledger holds it now.
    pub(super) fn landed(&mut self, reading: Reading, on: CompiledProject) {
        self.landed = Landed {
            declared: reading.declared,
            lines: reading.lines.into_iter().collect(),
            on,
        };
    }

    /// Mark the rename the texts hold last as showing when `shows`, every other one not.
    pub(super) fn mark(&self, reading: &Reading, shows: bool) {
        let top = reading
            .declared
            .or_else(|| reading.lines.iter().filter_map(|(_, at)| *at).max());
        for (i, node) in self.nodes.iter().enumerate() {
            let showing = shows && Some(i) == top;
            node.rename.showing.store(showing, Ordering::Relaxed);
        }
    }
}
