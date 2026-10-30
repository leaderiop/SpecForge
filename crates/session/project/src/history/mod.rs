//! The history port (ADR 0287): what the host asks of version control. A project that declares a history flag
//! (an immutable or append-only field, an append-only kind, a pass reading the baseline) is judged against the
//! project as it was at a commit; the port answers where the repository is, which commit a revision names, and
//! the files under a directory at a commit. [`specforge_git`'s `GitCli`](https://docs.rs/specforge-git) is the
//! adapter; tests use [`crate::testing::FakeHistory`].

pub mod check;

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// A full commit id (hex).
#[derive(
    Debug,
    Clone,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
pub struct Sha(String);

impl Sha {
    pub fn new(hex: impl Into<String>) -> Self {
        Sha(hex.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The first seven characters, as a diagnostic names a commit.
    pub fn short(&self) -> &str {
        &self.0[..self.0.len().min(7)]
    }
}

/// Where a project's repository is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitPlace {
    /// The working tree's top directory, canonical.
    pub toplevel: PathBuf,
    /// The git directory (`.git`, or a worktree's own), absolute.
    pub git_dir: PathBuf,
    /// The directory the refs live in (shared by worktrees), absolute.
    pub common_dir: PathBuf,
    /// A shallow clone: history before its boundary is not there.
    pub shallow: bool,
}

/// What locating a repository found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Located {
    Repo(GitPlace),
    NotARepository,
    /// Git cannot be asked (not installed, refuses the directory, no answer in time), and why.
    Unavailable(String),
}

/// Why a question about history had no answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryError(pub String);

impl std::fmt::Display for HistoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A file at a commit: its path from the repository's top, and its bytes.
pub type CommitFile = (PathBuf, Vec<u8>);

/// A file asked of a commit by name: its path, and its bytes when the commit holds it as a file.
pub type Blob = (PathBuf, Option<Vec<u8>>);

/// An entry of a commit's tree: its path from the repository's top, and its size in bytes when it is a regular file
/// (`None` for a link or a submodule, which the host does not read at a commit).
pub type TreeEntry = (PathBuf, Option<u64>);

/// What the host asks of version control. Every method is read-only, and none runs unless a project declares
/// history.
pub trait History: Send + Sync {
    /// The repository `dir` is in.
    fn locate(&self, dir: &Path) -> Located;
    /// The commit `revision` names in `place`; `None` when it names none (an unborn `HEAD`, an unknown name).
    fn commit(&self, place: &GitPlace, revision: &str) -> Result<Option<Sha>, HistoryError>;
    /// The nearest common ancestor of two commits.
    fn merge_base(&self, place: &GitPlace, a: &Sha, b: &Sha) -> Result<Option<Sha>, HistoryError>;
    /// Every file under `dir` (from the repository's top) at `commit`.
    fn files(
        &self,
        place: &GitPlace,
        commit: &Sha,
        dir: &Path,
    ) -> Result<Vec<CommitFile>, HistoryError>;
    /// Every entry under `dir` (from the repository's top) at `commit`, with no content: one question (ADR 0300).
    fn tree(
        &self,
        place: &GitPlace,
        commit: &Sha,
        dir: &Path,
    ) -> Result<Vec<TreeEntry>, HistoryError>;
    /// The named files (from the repository's top) at `commit`, in the order asked: `None` for one the commit does
    /// not hold as a file. One question for all of them (a judged rule's file inputs, ADR 0290).
    fn blobs(
        &self,
        place: &GitPlace,
        commit: &Sha,
        paths: &[PathBuf],
    ) -> Result<Vec<Blob>, HistoryError>;
}

pub type SharedHistory = Arc<dyn History>;

static DEFAULT: RwLock<Option<SharedHistory>> = RwLock::new(None);

/// Install the history every environment without one of its own asks (the binary installs the git adapter once).
pub fn set_default(history: Option<SharedHistory>) {
    *DEFAULT.write().unwrap_or_else(|e| e.into_inner()) = history;
}

/// The installed default, if any.
pub fn default_history() -> Option<SharedHistory> {
    DEFAULT.read().unwrap_or_else(|e| e.into_inner()).clone()
}
