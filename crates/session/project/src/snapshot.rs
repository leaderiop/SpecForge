//! A snapshot (ADR 0376 D1, D2, D4): the compiled graph of one state of a project, at a git revision or in the
//! working tree, with the key that names that state and what became of each collector's facts there. A revision's
//! graph is built by [`load_at`], the one way a graph is built from a commit: it reads the commit's `*.spec` files and
//! harness ledger through the [`Git`] port, with no checkout and no file written, and builds them through the one cold
//! build ([`Environment::build_sources`]). `Baseline::at` builds through it too.
//!
//! Facts are never read from git (stores are git-ignored) and never collected for a past revision: a revision is
//! served the store its own ledger records, live or archived by a later collect ([`retain`]), read from the disk,
//! only when its bytes are the ones that line digests.
//!
//! Both sides of a comparison are compiled with the current extensions, so a diff shows changes of content, not of
//! toolchain (ADR 0376 D3); a revision whose `specforge.json` loads another extension set says so ([`ConfigAt`]).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use specforge_common::facts::{filed, is_collector_name, is_fact_store, is_store_key};
use specforge_common::{Diagnostic, ExtensionEntry, PROJECT_FILE, codes, parse_project_config};
use specforge_effects::git::{CommitFile, Git, GitPlace, Located, Sha, TreeEntry};
use specforge_graph::{FieldForms, Graph, GraphBuild};
use specforge_protocol_types::DeltaBase;
use specforge_registry::Disk;

use crate::baseline::UnknownRevision;
use crate::delta::Delta;
use crate::facts::{Reach, Recorded, Store, recorded_by, retain, served_text};
use crate::harness::Ledger;
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
    /// How its `specforge.json` compares with the current one: the working tree's is the current one.
    pub config: ConfigAt,
}

/// How a revision's `specforge.json` compares with the current one, whose extensions both sides are compiled with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigAt {
    /// It loads the extensions the current one loads, whatever their order or version.
    Same,
    /// It loads another set: `added` the extensions only the current one loads, `removed` those only it loads, each
    /// sorted by name.
    Differs {
        added: Vec<String>,
        removed: Vec<String>,
    },
    /// It cannot be used (not UTF-8, not JSON, not an object, an `extensions` that is no list), and why.
    Unreadable(String),
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
    /// No git port answers here: none is given, or it cannot read the repository, and why.
    GitUnavailable(String),
    /// Git could not resolve the revision, or could not give the commit's files, and why.
    Unreadable(String),
}

/// What a check run since a revision is judged since ([`since`]): the commit the revision names and, when it was
/// asked for, the merge-base of that commit and `HEAD`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Since {
    /// The revision's commit and its merge-base with `HEAD`, in the repository `place`.
    Base {
        place: GitPlace,
        named: Sha,
        base: Sha,
    },
    /// The revision's commit; its merge-base was not asked for.
    Named { place: GitPlace, named: Sha },
    /// No merge-base, and why. `named` is the revision's commit when git resolved it before it stopped.
    NoBase { named: Option<Sha>, why: NoBase },
}

/// Why a check run since a revision has no merge-base to be judged against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoBase {
    /// The environment was given no git port: history is never asked.
    NoGitPort,
    /// The project is in no git repository.
    NotARepository,
    /// Git could not be asked, or could not answer, and why.
    Unreadable(String),
    /// `HEAD` names no commit yet.
    UnbornHead,
    /// The merge-base is not in this shallow clone: the message that says so.
    Shallow(String),
    /// The revision and `HEAD` share no commit: the message that says so.
    Unrelated(String),
}

/// The one resolver of what a check is judged since (`check --since`, a delta's revision base, ADR 0376 D7): the
/// commit `revision` names in `env`'s repository, then, when `needs_base`, the commit `HEAD` names and the merge-base
/// of the two, each asked of the git port in process, in that order, and nothing more. `UnknownRevision` when the
/// repository names no commit by `revision`.
pub fn since(
    env: &Environment,
    revision: &str,
    needs_base: bool,
) -> Result<Since, UnknownRevision> {
    let no_base = |named: Option<&Sha>, why: NoBase| Since::NoBase {
        named: named.cloned(),
        why,
    };
    let Some(history) = env.git() else {
        return Ok(no_base(None, NoBase::NoGitPort));
    };
    let (place, named) = match named(env, &*history, revision) {
        Ok(found) => found,
        Err(Unnamed::Unknown) => return Err(UnknownRevision(revision.to_string())),
        Err(Unnamed::NotARepository) => return Ok(no_base(None, NoBase::NotARepository)),
        Err(Unnamed::Unavailable(why) | Unnamed::Unreadable(why)) => {
            return Ok(no_base(None, NoBase::Unreadable(why)));
        }
    };
    if !needs_base {
        return Ok(Since::Named { place, named });
    }
    let head = match history.commit(&place, "HEAD") {
        Ok(Some(sha)) => sha,
        Ok(None) => return Ok(no_base(Some(&named), NoBase::UnbornHead)),
        Err(error) => return Ok(no_base(Some(&named), NoBase::Unreadable(error.to_string()))),
    };
    let base = match history.merge_base(&place, &named, &head) {
        Ok(Some(sha)) => sha,
        Ok(None) if place.shallow => {
            let why = format!(
                "the merge-base of {revision} and HEAD is not in this shallow clone: fetch more history (`fetch-depth: 0`)"
            );
            return Ok(no_base(Some(&named), NoBase::Shallow(why)));
        }
        Ok(None) => {
            let why =
                format!("{revision} and HEAD share no commit: nothing is judged against history");
            return Ok(no_base(Some(&named), NoBase::Unrelated(why)));
        }
        Err(error) => return Ok(no_base(Some(&named), NoBase::Unreadable(error.to_string()))),
    };
    Ok(Since::Base { place, named, base })
}

/// Why the git port gave no commit for a revision.
enum Unnamed {
    /// The project is in no git repository.
    NotARepository,
    /// Git cannot read the repository it located, and why.
    Unavailable(String),
    /// Git could not resolve the revision, and why.
    Unreadable(String),
    /// The repository names no commit by it.
    Unknown,
}

/// The repository of `env`, located through `history`, and the commit `revision` names there, asked in process.
fn named(env: &Environment, history: &dyn Git, revision: &str) -> Result<(GitPlace, Sha), Unnamed> {
    let place = match env.place(history) {
        Located::Repo(place) => place,
        Located::NotARepository => return Err(Unnamed::NotARepository),
        Located::Unavailable(why) => return Err(Unnamed::Unavailable(why)),
    };
    match history.commit(&place, revision) {
        Ok(Some(sha)) => Ok((place, sha)),
        Ok(None) => Err(Unnamed::Unknown),
        Err(error) => Err(Unnamed::Unreadable(error.to_string())),
    }
}

impl Snapshot {
    /// The project of `env` as the commit `revision` names holds it, compiled with `env`'s extensions (ADR 0376 D3),
    /// its facts the stores its ledger records, read through `disk` when `env` loads a collected kind (none else, as
    /// the working tree serves none). The revision is resolved by the git port in
    /// process, as [`since`] resolves it, never handed to a program; no collector runs.
    pub fn at_revision(
        env: &Environment,
        disk: &dyn Disk,
        revision: &str,
    ) -> Result<Snapshot, AtError> {
        let history = env
            .git()
            .ok_or_else(|| AtError::GitUnavailable("no git port is given".to_string()))?;
        let (place, sha) = named(env, &*history, revision).map_err(|unnamed| match unnamed {
            Unnamed::NotARepository => AtError::NotARepository,
            Unnamed::Unavailable(why) => AtError::GitUnavailable(why),
            Unnamed::Unreadable(why) => AtError::Unreadable(why),
            Unnamed::Unknown => AtError::UnknownRevision(revision.to_string()),
        })?;
        let wants = Wants {
            tree: false,
            // the stores are sources only while a collected kind is loaded, as in the working tree
            facts: env.reads_facts().then_some(disk),
            config: true,
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
            config: loaded.config.unwrap_or(ConfigAt::Same),
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
            config: ConfigAt::Same,
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
    /// What it is about: the collector, the extensions that differ (comma-separated), or `specforge.json`.
    pub subject: String,
    pub message: String,
}

/// A side of a comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    From,
    To,
}

/// What a side could not compare as written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteAbout {
    /// No store its ledger records is there.
    FactsNotCollected,
    /// A store is there and is not the one recorded, or does not read (W191, W192).
    FactsUnrecorded,
    /// Its `specforge.json` loads another extension set: it was compiled with the current one.
    ExtensionsDiffer,
    /// Its `specforge.json` cannot be used: it was compiled with the current extensions.
    ConfigUnreadable,
}

/// What changed from `from` to `to`, labelled `label` (ADR 0376 D3, D4, D5). A side whose `specforge.json` loads
/// another extension set, or cannot be used, gets a note first: it was compiled with the current extensions. A
/// collector served on one side only, or unrecorded on either, is left out of both: its stores' nodes are no change,
/// and each side that lacks it gets a note (`facts not collected at <side>`, or its W191 or W192). A collector neither
/// side collected needs no note.
pub fn compare(from: &Snapshot, to: &Snapshot, forms: &FieldForms, label: DeltaBase) -> Comparison {
    let mut notes: Vec<SnapshotNote> = [(Side::From, from), (Side::To, to)]
        .into_iter()
        .filter_map(|(side, snapshot)| config_note(side, snapshot))
        .collect();
    let collectors: BTreeSet<&String> = from.facts.keys().chain(to.facts.keys()).collect();
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
    let served = |snapshot: &Snapshot| -> BTreeSet<String> {
        snapshot
            .facts
            .iter()
            .filter(|(_, at)| **at == FactsAt::Served)
            .map(|(collector, _)| collector.clone())
            .collect()
    };
    Comparison {
        delta: facts_served_on_both(delta, &served(from), &served(to)),
        notes,
    }
}

/// `delta` from a side whose served collectors are `from` to one whose are `to`, without the facts of a collector
/// served on one side only (ADR 0376 D4): a store is never in git, so one side's missing store is never read as
/// every fact of it removed or added. A collector neither side serves has no fact in either graph. The one rule of a
/// diff ([`compare`]) and of a check's delta since a revision.
pub(crate) fn facts_served_on_both(
    delta: Delta,
    from: &BTreeSet<String>,
    to: &BTreeSet<String>,
) -> Delta {
    let lacking: BTreeSet<&String> = from.symmetric_difference(to).collect();
    if lacking.is_empty() {
        return delta;
    }
    delta.without(|file| {
        filed(file).is_some_and(|(collector, _)| lacking.iter().any(|c| *c == collector))
    })
}

/// The collectors `build` serves a fact store of: each store it was built from is one its compile served, whatever
/// facts it holds (one that was not served is no source of the build).
pub(crate) fn served_by(build: &GraphBuild) -> BTreeSet<String> {
    build
        .files()
        .filter_map(|(key, _)| filed(key))
        .map(|(collector, _)| collector.to_string())
        .collect()
}

/// The note on `side` when `snapshot`'s `specforge.json` is not the current one's extension set.
fn config_note(side: Side, snapshot: &Snapshot) -> Option<SnapshotNote> {
    const CURRENT: &str = "both sides were compiled with the current extensions";
    let name = snapshot.name();
    let (about, subject, message) = match &snapshot.config {
        ConfigAt::Same => return None,
        ConfigAt::Differs { added, removed } => {
            let names: Vec<&str> = added.iter().chain(removed).map(String::as_str).collect();
            let signed: Vec<String> = added
                .iter()
                .map(|n| format!("+{n}"))
                .chain(removed.iter().map(|n| format!("-{n}")))
                .collect();
            (
                NoteAbout::ExtensionsDiffer,
                names.join(", "),
                format!(
                    "the extensions at {name} differ from the current ones ({}); {CURRENT}",
                    signed.join(", ")
                ),
            )
        }
        ConfigAt::Unreadable(why) => (
            NoteAbout::ConfigUnreadable,
            PROJECT_FILE.to_string(),
            format!("{PROJECT_FILE} at {name} cannot be used: {why}; {CURRENT}"),
        ),
    };
    Some(SnapshotNote {
        side,
        about,
        subject,
        message,
    })
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
    /// How the commit's `specforge.json` compares with the current one ([`ConfigAt`]).
    pub config: bool,
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
    /// How the commit's `specforge.json` compares with the current one, when asked for.
    pub config: Option<ConfigAt>,
    pub built: SourceBuild,
}

/// The project of `env` at `commit` of the repository `place`, read through `history` (no checkout) and built with
/// `env`'s extensions: its specs and its ledger, its tree, the fact stores its ledger records and how its
/// `specforge.json` compares with the current one when `wants` asks. The config is read in the one question that
/// reads the specs.
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
    // Only what a graph is built from, the specs and the ledger, and the project's own config when it is asked for.
    let config = within.join(PROJECT_FILE);
    let wanted = |path: &Path| {
        path.extension().is_some_and(|e| e == "spec")
            || path
                .file_name()
                .is_some_and(|n| n == crate::harness::LEDGER_FILE)
            || (wants.config && path == config)
    };
    let mut files = history
        .files(place, commit, &within, &wanted)
        .map_err(|e| e.to_string())?;
    let config = wants.config.then(|| {
        let written = files
            .iter()
            .position(|(path, _)| *path == config)
            .map(|at| files.remove(at).1);
        config_at(env, &config, written)
    });
    let tree = if wants.tree {
        history
            .tree(place, commit, &within)
            .map_err(|e| e.to_string())?
    } else {
        Vec::new()
    };
    let mut loaded = LoadedAt::of_files(env, commit.clone(), within, files, tree, wants.facts);
    loaded.config = config;
    Ok(loaded)
}

/// How the `specforge.json` a commit holds at `path` (`None`: it holds none) compares with `env`'s, read by the one
/// reader of the file: none is the default config, which loads no extension.
fn config_at(env: &Environment, path: &Path, written: Option<Vec<u8>>) -> ConfigAt {
    let read = match written {
        Some(bytes) => match String::from_utf8(bytes) {
            Ok(text) => parse_project_config(path, &text),
            Err(_) => return ConfigAt::Unreadable("it is not UTF-8".to_string()),
        },
        None => Default::default(),
    };
    if let Some(problem) = read.problems.iter().find(|p| p.loads_nothing()) {
        return ConfigAt::Unreadable(problem.to_string());
    }
    let names = |entries: &[String]| -> std::collections::BTreeSet<String> {
        entries
            .iter()
            .map(|entry| {
                let parsed = ExtensionEntry::parse(entry);
                parsed.name().unwrap_or(entry.trim()).to_string()
            })
            .collect()
    };
    let (then, now) = (
        names(&read.config.extensions),
        names(&env.config().extensions),
    );
    if then == now {
        return ConfigAt::Same;
    }
    ConfigAt::Differs {
        added: now.difference(&then).cloned().collect(),
        removed: then.difference(&now).cloned().collect(),
    }
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
            config: None,
            built,
        }
    }
}

/// The fact stores the ledger `text` records, read through `disk` under `env`'s root: the text served of each one
/// whose bytes its line digests, live or archived ([`retain::find`]) and filed under its live source key as a compile
/// files it, and what became of each collector a line names. Each store a stale line names is found the same way, so
/// the renames it recorded rewrite the text served as they did at the revision. A line whose collector or key is no
/// store name is never made a path, and a store that is nowhere was not collected.
fn served_stores(
    env: &Environment,
    disk: &dyn Disk,
    text: &str,
) -> (BTreeMap<String, String>, BTreeMap<String, FactsAt>) {
    let recorded = recorded_by(
        disk,
        env.root(),
        Ledger::parse(text).ledger,
        Reach::Retained,
    );
    let mut stores = BTreeMap::new();
    let mut facts = BTreeMap::new();
    for line in recorded.ledger.lines() {
        let Some((collector, key)) = line.facts_of() else {
            continue;
        };
        let at = if is_collector_name(collector) && is_store_key(key) {
            let path = Store::path(collector, key);
            match retain::find(disk, env.root(), collector, key, &line.digest) {
                Some(bytes) => match served_text(&path, bytes, &recorded) {
                    Ok(served) => {
                        stores.insert(path, served);
                        FactsAt::Served
                    }
                    Err(diagnostic) => FactsAt::Unrecorded(diagnostic),
                },
                None => not_recorded(disk, env.root(), (collector, key), &recorded),
            }
        } else {
            FactsAt::NotCollected
        };
        settle(&mut facts, collector, at);
    }
    (stores, facts)
}

/// What became of `collector`'s store filed under `key` when none of its stores under the project root `root` holds
/// the bytes its line digests: the newest one there, live then archived, is unrecorded (its W191, or why it does not
/// read); none there was not collected.
fn not_recorded(
    disk: &dyn Disk,
    root: &Path,
    (collector, key): (&str, &str),
    recorded: &Recorded,
) -> FactsAt {
    let live = Store::path(collector, key);
    for store in retain::stores(disk, root, collector, key) {
        match disk.read(&root.join(&store)) {
            // judged by the one rule, under the live key the ledger line names
            Ok(bytes) => {
                return served_text(&live, bytes, recorded)
                    .err()
                    .map_or(FactsAt::NotCollected, FactsAt::Unrecorded);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return FactsAt::Unrecorded(crate::facts::unreadable(&store, e)),
        }
    }
    FactsAt::NotCollected
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
