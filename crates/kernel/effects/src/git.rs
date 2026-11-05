//! The `Git` port (ADR 0287, ADR 0335): what the host asks of version control. A project that declares a history
//! flag (an immutable or append-only field, an append-only kind, a pass reading the baseline) is judged against the
//! project as it was at a commit; the port answers where the repository is, which commit a revision names, the files
//! under a directory at a commit, and a worktree's branch and status. Its adapter is `specforge-git`'s `Gix`;
//! tests use [`crate::testing::FakeGit`].

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The typed status a hook is given (protocol 2.0.0) is what the port answers.
pub use specforge_protocol_types::{GitBranch, GitChange, GitEntry, GitEntryKind, GitStatus};

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

impl GitError {
    /// What a port answers for a question it does not implement: a trait method's default.
    pub fn unanswered() -> Self {
        GitError("this git port cannot answer it".into())
    }

    /// What a port answers for a write it does not implement: a trait method's default.
    pub fn unwritable() -> Self {
        GitError("this git port cannot write".into())
    }
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

/// The most bytes of one blob, or of one worktree file, a history read takes in: a larger one is reported (its size,
/// `truncated`), never read.
pub const BLOB_CAP: u64 = 16 * 1024 * 1024;

/// The most bytes of one file's patch: a longer one is cut at a line, and the file says `truncated`.
pub const PATCH_CAP: usize = 1024 * 1024;

/// The most commits a log lists when none is asked; its count still counts every commit it walks (but past a `-S`).
pub const LOG_MAX: usize = 1000;

/// Whether `bytes` are binary as git decides: a NUL in the first 8000.
pub fn is_binary(bytes: &[u8]) -> bool {
    bytes[..bytes.len().min(8000)].contains(&0)
}

/// What a log walked: how many commits it keeps (`rev-list --count`) and the first of them, newest first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitLog {
    /// Every commit the log keeps, or none when a `-S` log (which reads each commit's blobs) stopped at the first
    /// commit past its listing rather than read the rest of the history to count it.
    pub count: Option<usize>,
    pub commits: Vec<GitCommit>,
}

/// A blob a revision names: its id, its size, and its bytes when it is at most [`BLOB_CAP`] (`None`: not read).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitBlob {
    pub sha: Sha,
    pub size: u64,
    pub bytes: Option<Vec<u8>>,
}

/// An entry of a commit's tree: its path from the repository's top, and whether it is a regular file (`false` for a
/// link or a submodule, which the host does not read at a commit). No size: a listing never looks a blob up, so it
/// cannot fetch one (ADR 0300 D5); [`Git::sizes`] asks for the few files a pass reads.
pub type TreeEntry = (PathBuf, bool);

/// The kind of a git object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectKind {
    Commit,
    Tree,
    Blob,
    Tag,
}

/// Who made a commit, and when.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct GitPerson {
    pub name: String,
    pub email: String,
    /// ISO 8601 with the signer's offset, as `%aI` prints it.
    pub time: String,
}

/// One commit of a log.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct GitCommit {
    pub sha: Sha,
    pub parents: Vec<Sha>,
    pub author: GitPerson,
    pub committer: GitPerson,
    /// The message's first paragraph on one line (`%s`).
    pub subject: String,
    /// The rest of the message, trailers included (`%b`), trimmed.
    pub body: String,
    /// The trailers of the message's last paragraph, every value of each token in order (`Gsd-Task: 7`).
    pub trailers: BTreeMap<String, Vec<String>>,
}

/// Which commits a log walks: those `tips` reach that `hidden` do not, newest first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogQuery {
    pub tips: Vec<Sha>,
    pub hidden: Vec<Sha>,
    /// Only the commits that change something at one of these paths (from the top; a directory holds what is under
    /// it). Empty keeps every commit.
    pub paths: Vec<String>,
    /// Follow only each commit's first parent.
    pub first_parent: bool,
    /// At most this many listed: the newest (`git log -n`), or the oldest with [`LogQuery::oldest`].
    pub max: Option<usize>,
    /// [`LogQuery::max`] keeps the oldest commits walked rather than the newest.
    pub oldest: bool,
    /// Only the commits a line of whose message this regex matches (`--grep`, Rust's regex syntax).
    pub grep: Option<String>,
    /// Only the commits whose committer date is at or after this time, seconds since the epoch (`--since`).
    pub since: Option<i64>,
    /// Only the commits whose committer date is at or before this time (`--until`).
    pub until: Option<i64>,
    /// No commit with more than one parent (`--no-merges`).
    pub no_merges: bool,
    /// Only the commits that change how many times this regex matches in a file of [`LogQuery::paths`] against their
    /// parent (`git log -S --pickaxe-regex`): a merge never does, a root commit is compared with nothing.
    pub pickaxe: Option<String>,
}

/// The side a diff compares its base with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffSide {
    Commit(Sha),
    /// The index (`--staged`).
    Index,
    /// The tracked files as they are on disk (`git diff <base>`): untracked files are not in it.
    Worktree,
}

/// A diff between a commit and a side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffQuery {
    pub base: Sha,
    pub head: DiffSide,
    /// Only the files at or under these paths (from the top). Empty keeps every file.
    pub paths: Vec<String>,
    /// Give each text file's unified hunks.
    pub patch: bool,
}

/// How a file changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffStatus {
    Added,
    Deleted,
    Modified,
    Renamed,
    Copied,
    TypeChanged,
}

/// One changed file of a diff.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct GitFileDiff {
    pub path: String,
    /// The path it had at the base, for a rename or a copy.
    pub old_path: Option<String>,
    pub status: DiffStatus,
    pub added: u32,
    pub removed: u32,
    /// Either side holds a NUL in its first 8000 bytes, as git decides: no lines are counted.
    pub binary: bool,
    /// A side is over [`BLOB_CAP`] (not read: no lines are counted), or the patch was cut at [`PATCH_CAP`].
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
    /// The unified hunks (`@@ … @@`, three lines of context), when asked and the file is text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<String>,
}

/// A branch as `git branch -vv` lists it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct GitBranchRef {
    /// `main`, or `origin/main` for a remote-tracking branch.
    pub name: String,
    pub sha: Option<Sha>,
    /// `HEAD` is on it in the worktree asked.
    pub current: bool,
    #[serde(flatten)]
    pub tracking: Tracking,
}

/// The remote-tracking branch a local branch follows, and how many commits each is ahead of the other: what `git
/// branch -vv` shows in its brackets. A remote-tracking branch, or a local one that follows none, has the default.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct Tracking {
    /// `origin/main`.
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
}

/// A worktree of the repository: the main one or a linked one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitWorktree {
    pub path: PathBuf,
    /// The branch its `HEAD` is on; none when detached.
    pub branch: Option<String>,
    pub sha: Option<Sha>,
    /// The main worktree, not one `git worktree add` made.
    pub main: bool,
    pub locked: bool,
}

/// A remote and where its URL points.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct GitRemote {
    pub name: String,
    pub url: String,
    /// Where a push goes: its push URL, else its URL.
    pub push_url: String,
    /// The URL's host (`github.com`); none for a local path.
    pub host: Option<String>,
    /// The URL's path: `owner/repo` with `.git` stripped for a host, the path itself for a local one.
    pub path: String,
    /// The remote's default branch, as its `refs/remotes/<name>/HEAD` names it (`main`, as `git symbolic-ref --short`
    /// gives it less the remote's name); none when the clone recorded none.
    pub head: Option<String>,
}

/// A submodule `.gitmodules` declares.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct GitSubmodule {
    pub name: String,
    /// Where it is checked out, from the repository's top.
    pub path: String,
    /// Where it is cloned from, credentials removed; none when unset or unreadable.
    pub url: Option<String>,
}

/// What `git add` stages, or a commit before it records the index: paths from the repository's top.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Staging {
    /// The files at or under each path as the worktree has them, ignore rules kept; a deleted file leaves the index.
    /// A path that matches no file, on disk or in the index, is refused, as `git add` refuses it; so is an ignored
    /// one unless [`Staging::force`].
    pub add: Vec<String>,
    /// Out of the index at or under each path, the files left on disk (`git rm --cached`).
    pub remove: Vec<String>,
    /// Every tracked file as the worktree has it, a deleted one removed; untracked files stay out (`git commit -a`).
    pub all: bool,
    /// Stage an ignored path `add` names too (`git add --force`).
    pub force: bool,
    /// Left out of `add` and `all`: the files at or under each, whatever else names them (`:!path`).
    pub except: Vec<String>,
}

/// Which side of a conflict a file is restored from: the index's stage 2 (`--ours`) or 3 (`--theirs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictSide {
    Ours,
    Theirs,
}

/// What `git restore` puts back, and from where.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Restoring {
    /// The commit the paths come from; the index when none, or `HEAD` for `staged`.
    pub source: Option<Sha>,
    /// The index's entries (`--staged`).
    pub staged: bool,
    /// The files (`--worktree`); with `staged`, both from the source.
    pub worktree: bool,
    /// A conflicted file from that side (`--ours`, `--theirs`).
    pub side: Option<ConflictSide>,
}

/// How `git merge` merges.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Merging {
    /// `Some(true)`: a fast-forward or nothing (`--ff-only`); `Some(false)`: always a merge (`--no-ff`).
    pub fast_forward: Option<bool>,
    /// The merge commit's message.
    pub message: String,
    /// Leave the merge staged, unrecorded (`--no-commit`).
    pub no_commit: bool,
    /// Leave the merged tree staged as one change with no second parent (`--squash`).
    pub squash: bool,
}

/// How `git rm` removes: what it may take and whether the files go too.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Removal {
    /// Every file under a directory named (`-r`); without it a directory is refused.
    pub recursive: bool,
    /// The index entries alone, the files left on disk (`--cached`).
    pub cached: bool,
    /// Remove a file whose index or worktree differs from `HEAD` too (`-f`).
    pub force: bool,
    /// A path that matches no entry is passed over, not refused (`--ignore-unmatch`).
    pub ignore_unmatch: bool,
}

/// How a commit records the index.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Recording {
    /// Record even when the tree matches `HEAD`'s (`--allow-empty`).
    pub allow_empty: bool,
    /// Only the index's entries at or under these paths (from the repository's top), over `HEAD`'s tree: the other
    /// staged changes stay staged (`git commit -- <paths>`).
    pub only: Vec<String>,
    /// The commit whose author (name, email and time) this one takes (`git commit -C`).
    pub author_of: Option<Sha>,
}

/// A commit the port recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Committed {
    pub sha: Sha,
    /// The branch it moved; none when `HEAD` was detached.
    pub branch: Option<String>,
    pub parents: Vec<Sha>,
    /// The paths it changed against its first parent, or every path of a root commit, from the top.
    pub files: Vec<String>,
}

/// A path a merge, a cherry-pick, a revert or a stash left in conflict, and how (`both_modified`, `both_added`,
/// `deleted_by_us`, `deleted_by_them`), as `git status` names it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct GitConflict {
    pub path: String,
    pub kind: String,
}

/// What a merge, a cherry-pick or a revert did: the commit `HEAD` is on after it, or none when it stopped on
/// conflicts, which are left in the index and the files for the user to resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Merged {
    pub sha: Option<Sha>,
    /// `HEAD` moved to the other commit and nothing was recorded.
    pub fast_forward: bool,
    pub conflicts: Vec<GitConflict>,
}

/// How far a reset goes: the ref (`soft`), the index too (`mixed`), the files too (`hard`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResetMode {
    Soft,
    Mixed,
    Hard,
}

/// One stash entry, newest first.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct GitStash {
    pub index: usize,
    pub message: String,
    pub sha: Sha,
}

/// What the host asks of version control. Reads first: none runs for a check unless a project declares history or a
/// hook declares the `git` input (ADR 0320, ADR 0337 D2). `specforge git` asks the rest for the user, writes included
/// (ADR 0339); a write is the user's own act and nothing else asks for one.
pub trait Git: Send + Sync {
    /// The repository `dir` is in.
    fn locate(&self, dir: &Path) -> Located;
    /// The commit `revision` names in `place`; `None` when it names none (an unborn `HEAD`, an unknown name).
    fn commit(&self, place: &GitPlace, revision: &str) -> Result<Option<Sha>, GitError>;
    /// `commits` newest first by committer date, each once, as `git rev-list --no-walk=sorted` orders them (a tie
    /// keeps the order given).
    fn sorted(&self, _place: &GitPlace, _commits: &[Sha]) -> Result<Vec<Sha>, GitError> {
        Err(GitError::unanswered())
    }
    /// The time `text` names as git reads a date (`--since`): ISO 8601, RFC 2822, a unix time, or relative to now
    /// (`24 hours ago`, `yesterday`), seconds since the epoch.
    fn date(&self, _text: &str) -> Result<i64, GitError> {
        Err(GitError::unanswered())
    }
    /// The nearest common ancestor of two commits.
    fn merge_base(&self, place: &GitPlace, a: &Sha, b: &Sha) -> Result<Option<Sha>, GitError>;
    /// Every best common ancestor of two commits: a criss-cross history has more than one, and `A...B` hides them all.
    fn merge_bases(&self, place: &GitPlace, a: &Sha, b: &Sha) -> Result<Vec<Sha>, GitError> {
        Ok(self.merge_base(place, a, b)?.into_iter().collect())
    }
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
        Err(GitError::unanswered())
    }
    /// The worktree's typed status in `place` (read by gix, ADR 0337).
    fn status(&self, _place: &GitPlace) -> Result<GitStatus, GitError> {
        Err(GitError::unanswered())
    }
    /// `HEAD`'s commit and branch, its upstream, and how far each is ahead of the other: [`Self::status`]'s branch,
    /// with no worktree read.
    fn head(&self, place: &GitPlace) -> Result<GitBranch, GitError> {
        self.status(place).map(|status| status.branch)
    }
    /// The object `revision` names and its kind (`commit`, `tree`, `blob` or `tag`); `None` when it names none.
    fn resolve(
        &self,
        _place: &GitPlace,
        _revision: &str,
    ) -> Result<Option<(Sha, ObjectKind)>, GitError> {
        Err(GitError::unanswered())
    }
    /// Every value `key` (`section.name` or `section.sub.name`) has in the repository's configuration, the last the
    /// one git applies: empty when it has none.
    fn config(&self, _place: &GitPlace, _key: &str) -> Result<Vec<String>, GitError> {
        Err(GitError::unanswered())
    }
    /// The path `key` names, interpolated as git interpolates a path (`~/` and `~user/` from a home directory): none
    /// when unset. A relative path is returned as written.
    fn config_path(&self, _place: &GitPlace, _key: &str) -> Result<Option<PathBuf>, GitError> {
        Err(GitError::unanswered())
    }
    /// `key` read as git reads a boolean (`true`/`yes`/`on`, a non-zero integer, or the bare key): `None` when unset,
    /// an error when it holds no boolean.
    fn config_flag(&self, _place: &GitPlace, _key: &str) -> Result<Option<bool>, GitError> {
        Err(GitError::unanswered())
    }
    /// How many commits `query` walks, and the first `query.max` of them (every one when none), newest first.
    fn log(&self, _place: &GitPlace, _query: &LogQuery) -> Result<GitLog, GitError> {
        Err(GitError::unanswered())
    }
    /// The files that differ between `query`'s base and its side, by path.
    fn diff(&self, _place: &GitPlace, _query: &DiffQuery) -> Result<Vec<GitFileDiff>, GitError> {
        Err(GitError::unanswered())
    }
    /// The blob `revision` names (`HEAD:path`, `:path` for the index); `None` when it names no blob.
    fn blob(&self, _place: &GitPlace, _revision: &str) -> Result<Option<GitBlob>, GitError> {
        Err(GitError::unanswered())
    }
    /// The local branches, or the remote-tracking ones, by name.
    fn branches(&self, _place: &GitPlace, _remote: bool) -> Result<Vec<GitBranchRef>, GitError> {
        Err(GitError::unanswered())
    }
    /// The main worktree, then the linked ones by path.
    fn worktrees(&self, _place: &GitPlace) -> Result<Vec<GitWorktree>, GitError> {
        Err(GitError::unanswered())
    }
    /// The top of the repository whose submodule `place` is, as `git rev-parse --show-superproject-working-tree`
    /// finds it: the repository above `place`'s top whose index holds that top as a submodule. None when there is none.
    fn superproject(&self, _place: &GitPlace) -> Result<Option<PathBuf>, GitError> {
        Err(GitError::unanswered())
    }
    /// The submodules the worktree's `.gitmodules` declares, in its order (`git config --file .gitmodules`): one whose
    /// path is absolute or leaves the top is left out. Empty when there is no `.gitmodules`.
    fn submodules(&self, _place: &GitPlace) -> Result<Vec<GitSubmodule>, GitError> {
        Err(GitError::unanswered())
    }
    /// The remotes, by name.
    fn remotes(&self, _place: &GitPlace) -> Result<Vec<GitRemote>, GitError> {
        Err(GitError::unanswered())
    }
    /// Whether a write may change the repository at `place`: `Err` names why not (another user owns it; an operation
    /// the port cannot continue, a rebase, is in progress). A verb asks it before it runs anything, a hook included.
    fn writable(&self, _place: &GitPlace) -> Result<(), GitError> {
        Err(GitError::unwritable())
    }
    /// Write `message` where git hands a commit message to its `commit-msg` hook (`COMMIT_EDITMSG` in the git
    /// directory): that file's path.
    fn write_commit_message(&self, _place: &GitPlace, _message: &str) -> Result<PathBuf, GitError> {
        Err(GitError::unwritable())
    }
    /// The message as the `commit-msg` hook left it in `COMMIT_EDITMSG`.
    fn read_commit_message(&self, _place: &GitPlace) -> Result<String, GitError> {
        Err(GitError::unanswered())
    }
    /// Write `staging` into the index.
    fn stage(&self, _place: &GitPlace, _staging: &Staging) -> Result<(), GitError> {
        Err(GitError::unwritable())
    }
    /// Create the branch `name` at `at`; `Err` when it exists or is no valid branch name.
    /// Take the index entries at `paths` (from the repository's top; `*` the top itself) out of the index, a
    /// conflicted one's every stage, and unless `removal.cached` their files, as `git rm` does: the paths removed.
    /// Nothing is removed when one is refused: a path matching no entry, a directory without `recursive`, a file
    /// with changes unless `force`.
    fn remove(
        &self,
        _place: &GitPlace,
        _paths: &[String],
        _removal: Removal,
    ) -> Result<Vec<String>, GitError> {
        Err(GitError::unwritable())
    }
    fn branch_create(&self, _place: &GitPlace, _name: &str, _at: &Sha) -> Result<(), GitError> {
        Err(GitError::unwritable())
    }
    /// Delete the branch `name`: the commit it named.
    fn branch_delete(&self, _place: &GitPlace, _name: &str) -> Result<Sha, GitError> {
        Err(GitError::unwritable())
    }
    /// Create the tag `name` at `at`: annotated with `message` by the configured identity, else a plain ref. The object
    /// it names (the tag object for an annotated one).
    fn tag(
        &self,
        _place: &GitPlace,
        _name: &str,
        _at: &Sha,
        _message: Option<&str>,
    ) -> Result<Sha, GitError> {
        Err(GitError::unwritable())
    }
    /// Check out the branch `name` and move `HEAD` to it. A file the worktree changed that the checkout would
    /// overwrite refuses it, and nothing moves.
    fn switch(&self, _place: &GitPlace, _name: &str) -> Result<(), GitError> {
        Err(GitError::unwritable())
    }
    /// Add a linked worktree at `path` (absent or empty) on the existing branch `branch`.
    fn worktree_add(&self, _place: &GitPlace, _path: &Path, _branch: &str) -> Result<(), GitError> {
        Err(GitError::unwritable())
    }
    /// Remove the linked worktree at `path`, its directory and its record.
    fn worktree_remove(&self, _place: &GitPlace, _path: &Path) -> Result<(), GitError> {
        Err(GitError::unwritable())
    }
    /// Make `dir` a repository with its worktree, as `git init` does: true when one was made, false when `dir` holds
    /// one already (left as it is).
    fn init(&self, _dir: &Path) -> Result<bool, GitError> {
        Err(GitError::unwritable())
    }
    /// Lock the linked worktree whose top is `path`, so no prune or removal drops it (`reason` kept in its `locked`
    /// file), or with `lock` false unlock it. Refused when it is already in the state asked, as git refuses.
    fn worktree_lock(
        &self,
        _place: &GitPlace,
        _path: &Path,
        _lock: bool,
        _reason: Option<&str>,
    ) -> Result<(), GitError> {
        Err(GitError::unwritable())
    }
    /// Drop the records of the linked worktrees whose directories are gone: where each was.
    fn worktree_prune(&self, _place: &GitPlace) -> Result<Vec<PathBuf>, GitError> {
        Err(GitError::unwritable())
    }
    /// Merge `theirs` into `HEAD` as `merging` says: a fast-forward when `fast_forward` is not `Some(false)` and
    /// history allows (never with `squash`), refused when it is `Some(true)` and history does not; else a merge
    /// commit with `message`, or the conflicts. With `no_commit` the merge is left in the index, `MERGE_HEAD` kept
    /// for the commit that concludes it; with `squash` it is left in the index as one change, no `MERGE_HEAD`, its
    /// message in `SQUASH_MSG`.
    fn merge(
        &self,
        _place: &GitPlace,
        _theirs: &Sha,
        _merging: &Merging,
    ) -> Result<Merged, GitError> {
        Err(GitError::unwritable())
    }
    /// Move `HEAD` to `to`, and the index (`mixed`) and the files (`hard`) with it.
    fn reset(&self, _place: &GitPlace, _to: &Sha, _mode: ResetMode) -> Result<(), GitError> {
        Err(GitError::unwritable())
    }
    /// The files at `paths` back as `restoring` says: the files as the index (or its `source`) has them; with
    /// `staged`, the index's entries as `HEAD` (or the `source`) has them, the files too with `worktree`; with a
    /// `side`, a conflicted file as that stage of the index has it, the conflict left for a `git add`.
    fn restore(
        &self,
        _place: &GitPlace,
        _paths: &[String],
        _restoring: &Restoring,
    ) -> Result<(), GitError> {
        Err(GitError::unwritable())
    }
    /// Stash the worktree's and the index's changes (and the untracked files with `untracked`): the stash's commit.
    fn stash_push(
        &self,
        _place: &GitPlace,
        _message: Option<&str>,
        _untracked: bool,
    ) -> Result<Sha, GitError> {
        Err(GitError::unwritable())
    }
    /// Apply the stash at `index` and drop it; refused when it would overwrite a changed file.
    fn stash_pop(&self, _place: &GitPlace, _index: usize) -> Result<(), GitError> {
        Err(GitError::unwritable())
    }
    /// The stashes, newest first.
    fn stashes(&self, _place: &GitPlace) -> Result<Vec<GitStash>, GitError> {
        Err(GitError::unanswered())
    }
    /// Apply `commit` onto `HEAD` (`revert`: its inverse) and record it, or the conflicts. Unless `record`, it is
    /// applied onto the index as it stands and left staged (`--no-commit`), a revert's `REVERT_HEAD` kept as git
    /// keeps it.
    fn pick(
        &self,
        _place: &GitPlace,
        _commit: &Sha,
        _revert: bool,
        _record: bool,
    ) -> Result<Merged, GitError> {
        Err(GitError::unwritable())
    }
    /// End the cherry-pick or revert in progress: its state goes, the index and the files stay (`--quit`), or with
    /// `abort` the paths it staged or left conflicted go back to `HEAD` too (`--abort`). Whether one was in
    /// progress; `abort` with none is refused, as git refuses it.
    fn pick_end(&self, _place: &GitPlace, _abort: bool) -> Result<bool, GitError> {
        Err(GitError::unwritable())
    }
    /// Record the index (or, with `recording.only`, `HEAD`'s tree with the index's entries at those paths) as a
    /// commit on `HEAD` with `message` (kept byte for byte), committed by the identity the repository's
    /// configuration names and authored by it or by `recording.author_of`'s author, and move `HEAD`'s branch to it.
    /// `Err` when the tree matches `HEAD`'s and `allow_empty` is false, or the index holds a conflict.
    fn commit_index(
        &self,
        _place: &GitPlace,
        _message: &str,
        _recording: &Recording,
    ) -> Result<Committed, GitError> {
        Err(GitError::unwritable())
    }
    /// The message of `commit`, byte for byte as it is stored (`git cat-file`'s, after the headers).
    fn message(&self, _place: &GitPlace, _commit: &Sha) -> Result<String, GitError> {
        Err(GitError::unanswered())
    }
}

/// A git port as the composition root shares it.
pub type SharedGit = Arc<dyn Git>;
