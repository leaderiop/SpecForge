//! The `Git` port (ADR 0287, ADR 0335): what the host asks of version control. A project that declares a history
//! flag (an immutable or append-only field, an append-only kind, a pass reading the baseline) is judged against the
//! project as it was at a commit; the port answers where the repository is, which commit a revision names, the files
//! under a directory at a commit, and a worktree's branch and status. Its adapter is `specforge-git`'s `GitCli`;
//! tests use [`crate::testing::FakeGit`].

use std::path::{Path, PathBuf};
use std::sync::Arc;

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
pub struct GitError(pub String);

/// What git says of a worktree: its porcelain status as written (what a hook is given, ADR 0320), and whether it
/// holds a change HEAD does not (a collect's key, ADR 0327 D3).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorktreeStatus {
    /// `git status --porcelain=v2 --branch`, as git wrote it.
    pub porcelain: String,
    /// It lists a tracked change, a conflict or an untracked file.
    pub dirty: bool,
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A file at a commit: its path from the repository's top, and its bytes.
pub type CommitFile = (PathBuf, Vec<u8>);

/// A file asked of a commit by name: its path, and its bytes when the commit holds it as a file.
pub type Blob = (PathBuf, Option<Vec<u8>>);

/// A file asked of a commit by name: its path, and its size in bytes when the commit holds it as a file.
pub type Sized = (PathBuf, Option<u64>);

/// An entry of a commit's tree: its path from the repository's top, and whether it is a regular file (`false` for a
/// link or a submodule, which the host does not read at a commit). No size: a listing never looks a blob up, so it
/// cannot fetch one (ADR 0300 D5); [`Git::sizes`] asks for the few files a pass reads.
pub type TreeEntry = (PathBuf, bool);

/// What the host asks of version control. Every method is read-only, and none runs unless a project declares
/// history or a hook declares `reads_git` (ADR 0320).
pub trait Git: Send + Sync {
    /// The repository `dir` is in.
    fn locate(&self, dir: &Path) -> Located;
    /// The commit `revision` names in `place`; `None` when it names none (an unborn `HEAD`, an unknown name).
    fn commit(&self, place: &GitPlace, revision: &str) -> Result<Option<Sha>, GitError>;
    /// The nearest common ancestor of two commits.
    fn merge_base(&self, place: &GitPlace, a: &Sha, b: &Sha) -> Result<Option<Sha>, GitError>;
    /// Every file under `dir` (from the repository's top) at `commit` whose path `wanted` keeps: no other blob is
    /// read, so a repository's images, lockfiles and vendored trees never are.
    fn files(
        &self,
        place: &GitPlace,
        commit: &Sha,
        dir: &Path,
        wanted: &dyn Fn(&Path) -> bool,
    ) -> Result<Vec<CommitFile>, GitError>;
    /// Every entry under `dir` (from the repository's top) at `commit`, with no content and no size: one question
    /// (ADR 0300).
    fn tree(&self, place: &GitPlace, commit: &Sha, dir: &Path) -> Result<Vec<TreeEntry>, GitError>;
    /// The sizes of the named files (from the repository's top) at `commit`, in the order asked: `None` for one the
    /// commit does not hold as a file or git cannot give. One question for all of them.
    fn sizes(
        &self,
        place: &GitPlace,
        commit: &Sha,
        paths: &[PathBuf],
    ) -> Result<Vec<Sized>, GitError>;
    /// The named files (from the repository's top) at `commit`, in the order asked: `None` for one the commit does
    /// not hold as a file. One question for all of them (a judged rule's file inputs, ADR 0290).
    fn blobs(
        &self,
        place: &GitPlace,
        commit: &Sha,
        paths: &[PathBuf],
    ) -> Result<Vec<Blob>, GitError>;
    /// The branch `HEAD` is on in `place` (`git branch --show-current`): empty when it is detached (ADR 0320).
    fn branch(&self, _place: &GitPlace) -> Result<String, GitError> {
        Err(GitError("this git port cannot answer it".into()))
    }
    /// The worktree's status in `place` (`git status --porcelain=v2 --branch`, ADR 0320, ADR 0327 D3).
    fn status(&self, _place: &GitPlace) -> Result<WorktreeStatus, GitError> {
        Err(GitError("this git port cannot answer it".into()))
    }
}

/// A git port as the composition root shares it.
pub type SharedGit = Arc<dyn Git>;
