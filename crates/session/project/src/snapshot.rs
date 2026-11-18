//! A snapshot (ADR 0376 D1, D2, D4): the compiled graph of one state of a project, at a git revision or in the
//! working tree, with the key that names that state and what became of each collector's facts there. A revision's
//! graph is built by [`load_at`], the one way a graph is built from a commit: it reads the commit's `*.spec` files and
//! harness ledger through the [`Git`] port, with no checkout and no file written, and builds them through the one cold
//! build ([`Environment::build_sources`]). `Baseline::at` builds through it too.
//!
//! Facts are never read from git (stores are git-ignored) and never collected for a past revision: a revision is
//! served the store its own ledger records, read from the disk, only when its bytes are the ones that line digests.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use specforge_common::facts::{filed, is_collector_name, is_fact_store, is_store_key};
use specforge_common::{Diagnostic, codes};
use specforge_effects::git::{CommitFile, Git, GitPlace, Located, Sha, TreeEntry};
use specforge_graph::{FieldForms, Graph};
use specforge_protocol_types::DeltaBase;
use specforge_registry::Disk;

use crate::delta::Delta;
use crate::facts::{Store, recorded_by, served_text};
use crate::harness::{Ledger, Line};
use crate::sources::Read;
use crate::{CompiledProject, Environment, SourceBuild};

/// Which state of the project a snapshot is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotKey {
    /// A commit: the revision as it was named, and the commit it resolved to.
    Revision { name: String, sha: Sha },
    /// The working tree, under the key its facts are filed by (`<sha>`, `<sha>-dirty` or `worktree`).
    WorkingTree { key: String },
}

/// The compiled graph of one state of a project, spec entities and fact nodes, the key that names the state, and
/// what became of each collector's facts there, by collector: every loaded collector and every one its ledger names.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub key: SnapshotKey,
    pub graph: Arc<Graph>,
    pub facts: BTreeMap<String, FactsAt>,
}

/// What became of a collector's facts in a snapshot.
#[derive(Debug, Clone, PartialEq)]
pub enum FactsAt {
    /// A store its ledger records, byte for byte, is served: its facts are nodes of the graph.
    Served,
    /// No store its ledger records is there (or it records none): no fact of it is in the graph.
    NotCollected,
    /// A store is there and is not the one its ledger records, or does not read (W191, W192): no fact of it is in
    /// the graph.
    Unrecorded(Diagnostic),
}

/// Why no snapshot is taken at a revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtError {
    /// The repository names no commit by it.
    UnknownRevision(String),
    /// The project is in no git repository.
    NotARepository,
    /// Git could not be asked, or could not give the commit's files, and why.
    Unreadable(String),
}

impl Snapshot {
    /// The project of `env` as the commit `revision` names holds it, compiled with `env`'s extensions (ADR 0376 D3),
    /// its facts the stores its ledger records, read through `disk`. The revision is resolved by the git port in
    /// process, never handed to a program; no collector runs.
    pub fn at_revision(
        env: &Environment,
        disk: &dyn Disk,
        revision: &str,
    ) -> Result<Snapshot, AtError> {
        let history = env
            .git()
            .ok_or_else(|| AtError::Unreadable("no git port is given".to_string()))?;
        let place = match env.place(&*history) {
            Located::Repo(place) => place,
            Located::NotARepository => return Err(AtError::NotARepository),
            Located::Unavailable(why) => return Err(AtError::Unreadable(why)),
        };
        let sha = match history.commit(&place, revision) {
            Ok(Some(sha)) => sha,
            Ok(None) => return Err(AtError::UnknownRevision(revision.to_string())),
            Err(error) => return Err(AtError::Unreadable(error.to_string())),
        };
        let wants = Wants {
            tree: false,
            facts: Some(disk),
        };
        let loaded = load_at(env, &*history, &place, &sha, wants).map_err(AtError::Unreadable)?;
        let (graph, _) = loaded.built.graph.into_parts();
        Ok(Snapshot {
            key: SnapshotKey::Revision {
                name: revision.to_string(),
                sha,
            },
            graph: Arc::new(graph),
            facts: loaded.facts,
        })
    }

    /// The working tree as `project` compiled it, keyed by the key its facts are filed by: a collector is served when
    /// the project serves a store of it, unrecorded when a store of it is there and not served (its W191 or W192).
    pub fn of_project(project: &CompiledProject) -> Snapshot {
        let env = project.environment();
        let mut facts = BTreeMap::new();
        for key in project.source_keys() {
            let Some((collector, _)) = filed(key) else {
                continue;
            };
            let at = match project.source_text(key) {
                Some(_) => FactsAt::Served,
                None => FactsAt::Unrecorded(not_served(project, key)),
            };
            settle(&mut facts, collector, at);
        }
        not_collected(env, &mut facts);
        Snapshot {
            key: SnapshotKey::WorkingTree {
                key: crate::facts::key(env).0,
            },
            graph: Arc::new(project.graph().clone()),
            facts,
        }
    }
}

/// Two snapshots compared: what changed between them, and what either side could not compare.
#[derive(Debug, Clone)]
pub struct Comparison {
    /// The change, without the facts of a collector either side lacks.
    pub delta: Delta,
    pub notes: Vec<SnapshotNote>,
}

/// What one side of a comparison could not compare.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotNote {
    pub side: Side,
    pub about: NoteAbout,
    /// The collector it is about.
    pub subject: String,
    pub message: String,
}

/// A side of a comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    From,
    To,
}

/// Why a side lacks a collector's facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteAbout {
    /// No store its ledger records is there.
    FactsNotCollected,
    /// A store is there and is not the one recorded, or does not read (W191, W192).
    FactsUnrecorded,
}

/// What changed from `from` to `to`, labelled `label` (ADR 0376 D4, D5). A collector served on one side only, or
/// unrecorded on either, is left out of both: its stores' nodes are no change, and each side that lacks it gets a note
/// (`facts not collected at <side>`, or its W191 or W192). A collector neither side collected needs no note.
pub fn compare(from: &Snapshot, to: &Snapshot, forms: &FieldForms, label: DeltaBase) -> Comparison {
    let mut notes = Vec::new();
    let mut lacking = Vec::new();
    let collectors: std::collections::BTreeSet<&String> =
        from.facts.keys().chain(to.facts.keys()).collect();
    for collector in collectors {
        let sides = [(Side::From, from), (Side::To, to)];
        let at = |snapshot: &Snapshot| {
            snapshot
                .facts
                .get(collector)
                .cloned()
                .unwrap_or(FactsAt::NotCollected)
        };
        let (was, is) = (at(from), at(to));
        let neither = was == FactsAt::NotCollected && is == FactsAt::NotCollected;
        if (was == FactsAt::Served && is == FactsAt::Served) || neither {
            continue;
        }
        lacking.push(collector.clone());
        for (side, snapshot) in sides {
            let (about, message) = match at(snapshot) {
                FactsAt::Served => continue,
                FactsAt::NotCollected => (
                    NoteAbout::FactsNotCollected,
                    format!("facts not collected at {}", snapshot.name()),
                ),
                FactsAt::Unrecorded(diagnostic) => (NoteAbout::FactsUnrecorded, diagnostic.message),
            };
            notes.push(SnapshotNote {
                side,
                about,
                subject: collector.clone(),
                message,
            });
        }
    }
    let delta = Delta::between(&from.graph, &to.graph, forms, label);
    let delta = if lacking.is_empty() {
        delta
    } else {
        delta.without(|file| {
            filed(file).is_some_and(|(collector, _)| lacking.iter().any(|c| c == collector))
        })
    };
    Comparison { delta, notes }
}

impl Snapshot {
    /// How the snapshot is named to a reader: the revision as written, or `the working tree`.
    pub fn name(&self) -> &str {
        match &self.key {
            SnapshotKey::Revision { name, .. } => name,
            SnapshotKey::WorkingTree { .. } => "the working tree",
        }
    }
}

/// The W191 or W192 `project` reported for the fact store `key`, which it holds and does not serve.
fn not_served(project: &CompiledProject, key: &str) -> Diagnostic {
    let named = format!("fact store '{key}'");
    project
        .diagnostics()
        .into_iter()
        .find(|d| {
            (codes::W191.matches(&d.code) || codes::W192.matches(&d.code))
                && d.message.starts_with(&named)
        })
        .unwrap_or_else(|| crate::facts::unreadable(key, "it is not served"))
}

/// `at` for `collector` in `facts`: a collector one store serves stays served, whatever another store of it says.
fn settle(facts: &mut BTreeMap<String, FactsAt>, collector: &str, at: FactsAt) {
    match facts.get(collector) {
        Some(FactsAt::Served) => {}
        Some(FactsAt::Unrecorded(_)) if at == FactsAt::NotCollected => {}
        _ => {
            facts.insert(collector.to_string(), at);
        }
    }
}

/// Every collector `env` loads that `facts` says nothing of: not collected.
fn not_collected(env: &Environment, facts: &mut BTreeMap<String, FactsAt>) {
    for declaration in env.registries().declarations() {
        for collector in &declaration.collectors {
            facts
                .entry(collector.name.clone())
                .or_insert(FactsAt::NotCollected);
        }
    }
}

/// What [`load_at`] reads besides the specs and the ledger.
#[derive(Clone, Copy, Default)]
pub(crate) struct Wants<'d> {
    /// The commit's whole tree: for the checks and a compute pass that reads files (ADR 0300).
    pub tree: bool,
    /// The fact stores the commit's ledger records, read through this disk: none, no fact is served.
    pub facts: Option<&'d dyn Disk>,
}

/// A commit's project as [`load_at`] read and built it.
pub(crate) struct LoadedAt {
    pub revision: Sha,
    /// The project's directory, from the repository's top.
    pub within: PathBuf,
    /// Each file read from the commit, by source key.
    pub held: BTreeMap<String, Vec<u8>>,
    /// The text of each source the graph was built from, by source key: the specs the environment holds, and each
    /// fact store served.
    pub texts: BTreeMap<String, String>,
    /// The text of `specforge-harness` at the commit (empty when it has none).
    pub ledger: String,
    /// The commit's tree under the project, when asked for.
    pub tree: Vec<TreeEntry>,
    /// What became of each collector's facts, when they were asked for.
    pub facts: BTreeMap<String, FactsAt>,
    pub built: SourceBuild,
}

/// The project of `env` at `commit` of the repository `place`, read through `history` (no checkout) and built with
/// `env`'s extensions: its specs and its ledger, its tree and the fact stores its ledger records when `wants` asks.
pub(crate) fn load_at(
    env: &Environment,
    history: &dyn Git,
    place: &GitPlace,
    commit: &Sha,
    wants: Wants<'_>,
) -> Result<LoadedAt, String> {
    let root = env.root().canonicalize().map_err(|e| e.to_string())?;
    let within = root
        .strip_prefix(&place.toplevel)
        .map_err(|_| "the project is outside the repository's working tree".to_string())?
        .to_path_buf();
    // Only what a graph is built from: the specs and the ledger.
    let wanted = |path: &Path| {
        path.extension().is_some_and(|e| e == "spec")
            || path
                .file_name()
                .is_some_and(|n| n == crate::harness::LEDGER_FILE)
    };
    let files = history
        .files(place, commit, &within, &wanted)
        .map_err(|e| e.to_string())?;
    let tree = if wants.tree {
        history
            .tree(place, commit, &within)
            .map_err(|e| e.to_string())?
    } else {
        Vec::new()
    };
    Ok(LoadedAt::of_files(
        env,
        commit.clone(),
        within,
        files,
        tree,
        wants.facts,
    ))
}

impl LoadedAt {
    /// The project of `files` (from the repository's top; `within` is the project's directory there) at `revision`,
    /// built in `env`: each file held by its source key, the ledger's text, and the graph of the sources `env` holds
    /// and, when `facts` is a disk, of the fact stores the ledger records as it records them ([`served_stores`]).
    pub(crate) fn of_files(
        env: &Environment,
        revision: Sha,
        within: PathBuf,
        files: Vec<CommitFile>,
        tree: Vec<TreeEntry>,
        facts: Option<&dyn Disk>,
    ) -> LoadedAt {
        let mut texts: BTreeMap<String, String> = BTreeMap::new();
        let mut held: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        let mut ledger = String::new();
        for (path, bytes) in files {
            let Some(key) = key_of(&within, &path) else {
                continue;
            };
            held.insert(key.clone(), bytes.clone());
            if key == crate::harness::LEDGER_FILE {
                ledger = String::from_utf8(bytes).unwrap_or_default();
            } else if key.ends_with(".spec")
                && env.sources().holds_key(&key)
                && let Ok(text) = String::from_utf8(bytes)
            {
                texts.insert(key, text);
            }
        }
        let mut served = BTreeMap::new();
        if let Some(disk) = facts {
            let (stores, at) = served_stores(env, disk, &ledger);
            texts.extend(stores);
            served = at;
        }
        let written: BTreeMap<String, &str> =
            texts.iter().map(|(k, v)| (k.clone(), v.as_str())).collect();
        let keys: Vec<String> = texts.keys().cloned().collect();
        let built = env.build_sources(&keys, &written, |_| Read::Gone);
        // a store the ledger records that is no store this specforge reads is the build's W192
        for key in keys.iter().filter(|key| is_fact_store(key)) {
            if let (Read::Unreadable(w192), Some((collector, _))) =
                (built.sources.as_read(key), filed(key))
            {
                served.insert(collector.to_string(), FactsAt::Unrecorded(w192));
            }
        }
        if facts.is_some() {
            not_collected(env, &mut served);
        }
        LoadedAt {
            revision,
            within,
            held,
            texts,
            ledger,
            tree,
            facts: served,
            built,
        }
    }
}

/// The fact stores the ledger `text` records, read through `disk` under `env`'s root: the text served of each one
/// whose bytes its line digests, by source key, and what became of each collector a line names. A line whose collector
/// or key is no store name is never made a path, and a store that is not there was not collected.
fn served_stores(
    env: &Environment,
    disk: &dyn Disk,
    text: &str,
) -> (BTreeMap<String, String>, BTreeMap<String, FactsAt>) {
    let recorded = recorded_by(disk, env.root(), Ledger::parse(text).ledger);
    let mut stores = BTreeMap::new();
    let mut facts = BTreeMap::new();
    for (collector, key) in recorded.ledger.lines().filter_map(Line::facts_of) {
        let at = if is_collector_name(collector) && is_store_key(key) {
            let path = Store::path(collector, key);
            match disk.read(&env.root().join(&path)) {
                Ok(bytes) => match served_text(&path, bytes, &recorded) {
                    Ok(served) => {
                        stores.insert(path, served);
                        FactsAt::Served
                    }
                    Err(diagnostic) => FactsAt::Unrecorded(diagnostic),
                },
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => FactsAt::NotCollected,
                Err(e) => FactsAt::Unrecorded(crate::facts::unreadable(&path, e)),
            }
        } else {
            FactsAt::NotCollected
        };
        settle(&mut facts, collector, at);
    }
    (stores, facts)
}

/// The source key of `path` (from the repository's top) under the project's directory `within`.
pub(crate) fn key_of(within: &Path, path: &Path) -> Option<String> {
    let rest = path.strip_prefix(within).ok()?;
    Some(
        rest.components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
    )
}
