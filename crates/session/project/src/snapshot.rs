//! A snapshot (ADR 0376 D1, D2): the compiled graph of one state of a project, at a git revision or in the working
//! tree, with the key that names that state. A revision's graph is built by [`load_at`], the one way a graph is built
//! from a commit: it reads the commit's `*.spec` files and harness ledger through the [`Git`] port, with no checkout
//! and no file written, and builds them through the one cold build ([`Environment::build_sources`]).
//! `Baseline::at` builds through it too.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use specforge_effects::git::{CommitFile, Git, GitPlace, Located, Sha, TreeEntry};
use specforge_graph::Graph;

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

/// The compiled graph of one state of a project, spec entities and fact nodes, and the key that names the state.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub key: SnapshotKey,
    pub graph: Arc<Graph>,
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
    /// The project of `env` as the commit `revision` names holds it, compiled with `env`'s extensions (ADR 0376 D3).
    /// The revision is resolved by the git port in process, never handed to a program.
    pub fn at_revision(env: &Environment, revision: &str) -> Result<Snapshot, AtError> {
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
        let loaded = load_at(env, &*history, &place, &sha, Wants { tree: false })
            .map_err(AtError::Unreadable)?;
        let (graph, _) = loaded.built.graph.into_parts();
        Ok(Snapshot {
            key: SnapshotKey::Revision {
                name: revision.to_string(),
                sha,
            },
            graph: Arc::new(graph),
        })
    }

    /// The working tree as `project` compiled it, keyed by the key its facts are filed by.
    pub fn of_project(project: &CompiledProject) -> Snapshot {
        Snapshot {
            key: SnapshotKey::WorkingTree {
                key: crate::facts::key(project.environment()).0,
            },
            graph: Arc::new(project.graph().clone()),
        }
    }
}

/// What [`load_at`] reads besides the specs and the ledger.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Wants {
    /// The commit's whole tree: for the checks and a compute pass that reads files (ADR 0300).
    pub tree: bool,
}

/// A commit's project as [`load_at`] read and built it.
pub(crate) struct LoadedAt {
    pub revision: Sha,
    /// The project's directory, from the repository's top.
    pub within: PathBuf,
    /// Each file read, by source key.
    pub held: BTreeMap<String, Vec<u8>>,
    /// The text of each source the environment holds, by source key.
    pub texts: BTreeMap<String, String>,
    /// The text of `specforge-harness` at the commit (empty when it has none).
    pub ledger: String,
    /// The commit's tree under the project, when asked for.
    pub tree: Vec<TreeEntry>,
    pub built: SourceBuild,
}

/// The project of `env` at `commit` of the repository `place`, read through `history` (no checkout) and built with
/// `env`'s extensions: its specs and its ledger, and its tree when `wants` asks for it.
pub(crate) fn load_at(
    env: &Environment,
    history: &dyn Git,
    place: &GitPlace,
    commit: &Sha,
    wants: Wants,
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
    Ok(LoadedAt::of_files(env, commit.clone(), within, files, tree))
}

impl LoadedAt {
    /// The project of `files` (from the repository's top; `within` is the project's directory there) at `revision`,
    /// built in `env`: each file held by its source key, the ledger's text, and the graph of the sources `env` holds.
    pub(crate) fn of_files(
        env: &Environment,
        revision: Sha,
        within: PathBuf,
        files: Vec<CommitFile>,
        tree: Vec<TreeEntry>,
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
        let written: BTreeMap<String, &str> =
            texts.iter().map(|(k, v)| (k.clone(), v.as_str())).collect();
        let keys: Vec<String> = texts.keys().cloned().collect();
        let built = env.build_sources(&keys, &written, |_| Read::Gone);
        LoadedAt {
            revision,
            within,
            held,
            texts,
            ledger,
            tree,
            built,
        }
    }
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
