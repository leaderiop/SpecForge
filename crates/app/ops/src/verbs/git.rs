//! `specforge git` (ADR 0339): the `Git` port for people and agents, on the repository the project is in, under the
//! project's effects ceiling (`git` read, and `write` for a change). Everything runs in process: no git program runs.

use serde::Serialize;
use specforge_effects::git::{
    DiffQuery, DiffSide, GitBranchRef, GitCommit, GitError, GitFileDiff, GitPlace, GitRemote,
    GitStatus, GitSubmodule, LOG_MAX, Located, LogQuery, ObjectKind, Sha, Tracking, is_binary,
};

use super::{USER, Verbs, slashed};
use crate::{OpError, OpErrorKind, UNKNOWN_REVISION};
use specforge_effects::RealFs;

mod branching;
mod commit;
mod merging;

pub use branching::{
    LockAnswer, PrunedAnswer, RefAnswer, branch_create, branch_delete, switch, tag, worktree_add,
    worktree_lock, worktree_prune, worktree_remove,
};
pub use commit::{
    AddRequest, CommitAnswer, CommitRequest, HookAnswer, RmAnswer, add, cleaned, commit, hook_run,
    rm,
};
pub use merging::{
    FastForward, MergeAnswer, MergeRequest, PickEndAnswer, ResetAnswer, RestoreAnswer,
    RestoreRequest, StashAnswer, StashesAnswer, merge, pick, pick_end, reset, restore, stash_list,
    stash_pop, stash_push,
};
pub use specforge_effects::git::ResetMode;
pub use specforge_effects::git::{ConflictSide, Removal};

/// `git repo`'s answer: where the repository is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RepoAnswer {
    /// The working tree's top directory, absolute.
    pub top: String,
    pub git_dir: String,
    pub common_dir: String,
    /// A linked worktree: its git directory is not the common one.
    pub linked: bool,
    pub shallow: bool,
    /// The current directory from the top, `/`-ended as `git rev-parse --show-prefix` prints it (`""` at the top);
    /// none when the current directory is outside the working tree.
    pub prefix: Option<String>,
    /// The top of the repository this one is a submodule of (`--show-superproject-working-tree`), absolute; none when
    /// it is no submodule.
    pub superproject: Option<String>,
}

/// `git head`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HeadAnswer {
    /// None while `HEAD`'s branch has no commit.
    pub sha: Option<String>,
    /// None when `HEAD` is detached.
    pub branch: Option<String>,
    pub detached: bool,
    pub unborn: bool,
    #[serde(flatten)]
    pub tracking: Tracking,
}

/// `git rev`'s answer: one entry per revision asked, in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RevsAnswer {
    pub revs: Vec<Rev>,
}

/// What one revision names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Rev {
    pub rev: String,
    pub sha: Option<String>,
    pub kind: Option<ObjectKind>,
    pub exists: bool,
}

/// `git status`'s answer: the typed status, plus whether the tree is clean and its untracked paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatusAnswer {
    #[serde(flatten)]
    pub status: GitStatus,
    /// Nothing `HEAD` does not hold: no tracked change, conflict or untracked file (ADR 0327 D3).
    pub clean: bool,
    /// The paths of the tracked entries (a change, a rename, a conflict), in git's order: the lines of `git status
    /// --porcelain` but the `??` ones.
    pub tracked: Vec<String>,
    /// The paths of the untracked entries, in git's order.
    pub untracked: Vec<String>,
}

/// `git config`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConfigAnswer {
    pub key: String,
    /// The value git applies (the last); none when the key is unset.
    pub value: Option<String>,
    pub values: Vec<String>,
}

/// The place is in no repository (`PreconditionFailed`).
pub const NOT_A_REPOSITORY: &str = "not_a_repository";
/// No git port answers here (`PreconditionFailed`).
pub const GIT_UNAVAILABLE: &str = "git_unavailable";
/// The port failed (`Internal`).
pub const GIT_FAILED: &str = "git_failed";
/// A `show` path names a tree, not a file (`InvalidInput`).
pub const NOT_A_BLOB: &str = "not_a_blob";
/// The worktree has changes a checkout, merge or pick would lose (`PreconditionFailed`).
pub const DIRTY_TREE: &str = "dirty_tree";
/// The port refused or failed a write (`PreconditionFailed`).
pub const GIT_REFUSED: &str = "git_refused";
/// The repository asks for a signature specforge cannot make (`PreconditionFailed`).
pub const SIGNING_UNSUPPORTED: &str = "signing_unsupported";
/// A path to commit is outside the repository's worktree (`InvalidInput`).
pub const OUTSIDE_REPOSITORY: &str = "outside_repository";
/// A hook could not be started (`PreconditionFailed`).
pub const HOOK_CANNOT_RUN: &str = "hook_cannot_run";
/// A hook refused the commit (`PreconditionFailed`).
pub const HOOK_REFUSED: &str = "hook_refused";

/// `git log`'s answer: the commits, newest first, and how many (`rev-list --count`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LogAnswer {
    pub count: usize,
    /// The listing stopped at its `--max` (or at [`LOG_MAX`] when none was given) before `count` commits.
    pub truncated: bool,
    pub commits: Vec<GitCommit>,
}

/// What a log walks: a range, the paths it keeps, and how far.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogRequest {
    /// `A..B` (what B reaches and A does not), `A...B` (what either reaches and not both), one revision, or `HEAD`.
    pub range: Option<String>,
    pub paths: Vec<String>,
    pub first_parent: bool,
    pub max: Option<usize>,
    /// Oldest first; `max` still keeps the newest, as git applies `-n` before `--reverse`.
    pub reverse: bool,
    /// A regex a line of the message matches (`--grep`).
    pub grep: Option<String>,
    /// Dates as git reads them (`--since`, `--until`).
    pub since: Option<String>,
    pub until: Option<String>,
    pub no_merges: bool,
    /// The text, or with `pickaxe_regex` the regex, whose count in a file a commit changes (`-S`).
    pub pickaxe: Option<String>,
    pub pickaxe_regex: bool,
}

/// The side `git diff` compares its base with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffHead {
    /// A revision; `None` is `HEAD` when a base is named, else the worktree (`git diff HEAD`).
    Revision(Option<String>),
    Staged,
    Worktree,
}

/// What `git diff` compares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffRequest {
    pub base: Option<String>,
    pub head: DiffHead,
    pub paths: Vec<String>,
    pub patch: bool,
    /// Compare from where the base and the head's commit (HEAD for the index or the worktree) meet, as `git diff
    /// A...B` and `--merge-base` do.
    pub merge_base: bool,
}

/// `git diff`'s answer: the base commit, the head (a commit, `index` or `worktree`) and the changed files by path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiffAnswer {
    pub base: String,
    pub head: String,
    pub count: usize,
    /// The lines the text files gain and lose, as `--shortstat` totals them.
    pub added: u64,
    pub removed: u64,
    pub files: Vec<GitFileDiff>,
}

/// `git merge-base`'s answer: none when the two commits share no history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MergeBaseAnswer {
    pub sha: Option<String>,
}

/// `git show <rev>:<path>`'s answer: the blob, and its text when it is not binary and is UTF-8.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShowAnswer {
    pub sha: String,
    pub bytes: u64,
    /// A NUL in the first 8000 bytes, as git decides.
    pub binary: bool,
    /// Over [`specforge_effects::git::BLOB_CAP`]: the blob is sized, not read, so it has no text (and `binary` is not judged).
    pub truncated: bool,
    pub text: Option<String>,
}

/// `git branches`' answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BranchesAnswer {
    pub branches: Vec<GitBranchRef>,
}

/// `git worktrees`' answer: the main worktree first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorktreesAnswer {
    pub worktrees: Vec<Worktree>,
}

/// One worktree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Worktree {
    /// Its top directory, absolute.
    pub path: String,
    pub branch: Option<String>,
    pub sha: Option<String>,
    pub main: bool,
    pub locked: bool,
    /// The worktree the project is in.
    pub current: bool,
}

/// `git submodules`' answer: what `.gitmodules` declares.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SubmodulesAnswer {
    pub submodules: Vec<GitSubmodule>,
}

/// `git remotes`' answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RemotesAnswer {
    pub remotes: Vec<GitRemote>,
}

impl Verbs {
    /// Whether the user grant lets git be asked `question`: `Err` is E090 with the ceiling's words.
    pub fn permit_git(&self, question: &str) -> Result<(), OpError> {
        self.grant
            .permits_git(USER, question)
            .map_err(|denial| super::outside_ceiling(&denial.request, &denial))
    }

    /// [`Self::place`] for a write: the ceiling lets git be changed (`git` `write`) as well as asked.
    fn place_to_write(&self, question: &str) -> Result<GitPlace, OpError> {
        self.grant
            .permits_git_write(USER, question)
            .map_err(|denial| super::outside_ceiling(&denial.request, &denial))?;
        let place = self.place(question)?;
        // Before anything runs, a hook included: a repository another user owns runs none of its hooks.
        self.ports()
            .git
            .writable(&place)
            .map_err(commit::write_failed)?;
        Ok(place)
    }

    /// The repository the current directory is in, as git finds it, when the project holds the current directory (a
    /// repository nested in the project, or `--in`); else the project's. Once the ceiling lets git be asked
    /// `question`.
    fn place(&self, question: &str) -> Result<GitPlace, OpError> {
        self.permit_git(question)?;
        let project = RealFs::landing(self.root().dir());
        let from = if self.cwd().starts_with(&project) {
            self.cwd()
        } else {
            self.root().dir()
        };
        match self.ports().git.locate(from) {
            Located::Repo(place) => Ok(place),
            Located::NotARepository => Err(OpError::new(
                OpErrorKind::PreconditionFailed,
                NOT_A_REPOSITORY,
                format!("{} is in no git repository", self.root().dir().display()),
            )),
            Located::Unavailable(why) => Err(OpError::new(
                OpErrorKind::PreconditionFailed,
                GIT_UNAVAILABLE,
                format!("git cannot read the repository: {why}"),
            )),
        }
    }
}

/// A port's failure as the verb's.
fn failed(error: GitError) -> OpError {
    OpError::new(OpErrorKind::Internal, GIT_FAILED, error.0)
}

pub fn repo(verbs: &Verbs) -> Result<RepoAnswer, OpError> {
    let place = verbs.place("repo")?;
    let superproject = verbs.ports().git.superproject(&place).map_err(failed)?;
    let prefix = verbs.cwd().strip_prefix(&place.toplevel).ok().map(|rel| {
        let rel = slashed(rel);
        if rel.is_empty() {
            rel
        } else {
            format!("{rel}/")
        }
    });
    Ok(RepoAnswer {
        top: slashed(&place.toplevel),
        git_dir: slashed(&place.git_dir),
        common_dir: slashed(&place.common_dir),
        linked: place.git_dir != place.common_dir,
        shallow: place.shallow,
        prefix,
        superproject: superproject.as_deref().map(slashed),
    })
}

pub fn head(verbs: &Verbs) -> Result<HeadAnswer, OpError> {
    let place = verbs.place("head")?;
    let branch = verbs.ports().git.head(&place).map_err(failed)?;
    Ok(HeadAnswer {
        detached: branch.head.is_none(),
        unborn: branch.oid.is_none(),
        sha: branch.oid,
        branch: branch.head,
        // The port's head is the status protocol's branch header, a wire type the guests share.
        tracking: Tracking {
            upstream: branch.upstream,
            ahead: branch.ahead,
            behind: branch.behind,
        },
    })
}

/// How `git rev --sorted` orders the commits it is given: newest first by committer date, or oldest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevOrder {
    pub reverse: bool,
}

pub fn revs(
    verbs: &Verbs,
    asked: &[String],
    order: Option<RevOrder>,
) -> Result<RevsAnswer, OpError> {
    let place = verbs.place("rev")?;
    if let Some(order) = order {
        // as `rev-list --no-walk=sorted`: every revision names a commit, or none is answered
        let commits = asked
            .iter()
            .map(|rev| commit_named(verbs, &place, rev.trim()))
            .collect::<Result<Vec<_>, _>>()?;
        let mut sorted = verbs.ports().git.sorted(&place, &commits).map_err(failed)?;
        if order.reverse {
            sorted.reverse();
        }
        return Ok(RevsAnswer {
            revs: sorted
                .into_iter()
                .map(|sha| Rev {
                    rev: sha.as_str().to_string(),
                    sha: Some(sha.as_str().to_string()),
                    kind: Some(ObjectKind::Commit),
                    exists: true,
                })
                .collect(),
        });
    }
    let mut revs = Vec::with_capacity(asked.len());
    for rev in asked {
        let found = verbs.ports().git.resolve(&place, rev).map_err(failed)?;
        revs.push(Rev {
            rev: rev.clone(),
            exists: found.is_some(),
            sha: found.as_ref().map(|(sha, _)| sha.as_str().to_string()),
            kind: found.map(|(_, kind)| kind),
        });
    }
    Ok(RevsAnswer { revs })
}

pub fn status(verbs: &Verbs) -> Result<StatusAnswer, OpError> {
    let place = verbs.place("status")?;
    let status = verbs.ports().git.status(&place).map_err(failed)?;
    use specforge_effects::git::GitEntryKind as Kind;
    let paths = |kept: &dyn Fn(Kind) -> bool| {
        status
            .entries
            .iter()
            .filter(|e| kept(e.kind))
            .map(|e| e.path.clone())
            .collect()
    };
    let tracked = paths(&|kind| matches!(kind, Kind::Changed | Kind::Renamed | Kind::Unmerged));
    let untracked = paths(&|kind| kind == Kind::Untracked);
    Ok(StatusAnswer {
        clean: !status.dirty(),
        tracked,
        untracked,
        status,
    })
}

pub fn config(verbs: &Verbs, key: &str) -> Result<ConfigAnswer, OpError> {
    let place = verbs.place("config")?;
    let values = verbs.ports().git.config(&place, key).map_err(failed)?;
    Ok(ConfigAnswer {
        key: key.to_string(),
        value: values.last().cloned(),
        values,
    })
}

/// The commit `revision` names, or `unknown_revision`.
fn commit_named(verbs: &Verbs, place: &GitPlace, revision: &str) -> Result<Sha, OpError> {
    verbs
        .ports()
        .git
        .commit(place, revision)
        .map_err(failed)?
        .ok_or_else(|| {
            OpError::new(
                OpErrorKind::InvalidInput,
                UNKNOWN_REVISION,
                format!("{revision} names no commit"),
            )
        })
}

/// A user's value the port refused as no date or no pattern (`InvalidInput`).
fn invalid(error: GitError) -> OpError {
    OpError::new(
        OpErrorKind::InvalidInput,
        OpErrorKind::InvalidInput.as_str(),
        error.0,
    )
}

/// `pattern`, once it is a regex.
fn checked(pattern: String) -> Result<String, OpError> {
    match regex::Regex::new(&pattern) {
        Ok(_) => Ok(pattern),
        Err(error) => Err(invalid(GitError(format!(
            "{pattern:?} is no regex: {error}"
        )))),
    }
}

/// `HEAD` for an empty side of a range.
fn or_head(side: &str) -> &str {
    if side.is_empty() { "HEAD" } else { side }
}

pub fn log(verbs: &Verbs, request: &LogRequest) -> Result<LogAnswer, OpError> {
    let place = verbs.place("log")?;
    let named = |revision: &str| commit_named(verbs, &place, revision);
    let (tips, hidden) = match request.range.as_deref() {
        None => match verbs.ports().git.commit(&place, "HEAD").map_err(failed)? {
            Some(head) => (vec![head], Vec::new()),
            // an unborn HEAD has no history yet
            None => (Vec::new(), Vec::new()),
        },
        Some(range) => {
            if let Some((a, b)) = range.split_once("...") {
                let (a, b) = (named(or_head(a))?, named(or_head(b))?);
                let bases = verbs
                    .ports()
                    .git
                    .merge_bases(&place, &a, &b)
                    .map_err(failed)?;
                (vec![a, b], bases)
            } else if let Some((a, b)) = range.split_once("..") {
                (vec![named(or_head(b))?], vec![named(or_head(a))?])
            } else {
                (vec![named(range)?], Vec::new())
            }
        }
    };
    if tips.is_empty() {
        return Ok(LogAnswer {
            count: 0,
            truncated: false,
            commits: Vec::new(),
        });
    }
    let date = |text: &Option<String>| -> Result<Option<i64>, OpError> {
        text.as_deref()
            .map(|text| verbs.ports().git.date(text).map_err(invalid))
            .transpose()
    };
    let pickaxe = request.pickaxe.as_deref().map(|text| {
        if request.pickaxe_regex {
            text.to_string()
        } else {
            regex::escape(text)
        }
    });
    let query = LogQuery {
        tips,
        hidden,
        paths: request.paths.clone(),
        first_parent: request.first_parent,
        // with --reverse and no -n, the oldest commits are the ones listed, git's whole reversed log cut at the cap
        max: Some(request.max.unwrap_or(LOG_MAX)),
        oldest: request.reverse && request.max.is_none(),
        grep: request.grep.clone().map(checked).transpose()?,
        since: date(&request.since)?,
        until: date(&request.until)?,
        no_merges: request.no_merges,
        pickaxe: pickaxe.map(checked).transpose()?,
    };
    let mut log = verbs.ports().git.log(&place, &query).map_err(failed)?;
    if request.reverse {
        log.commits.reverse();
    }
    Ok(LogAnswer {
        count: log.count,
        truncated: log.count > log.commits.len(),
        commits: log.commits,
    })
}

pub fn diff(verbs: &Verbs, request: &DiffRequest) -> Result<DiffAnswer, OpError> {
    let place = verbs.place("diff")?;
    let mut base = commit_named(verbs, &place, request.base.as_deref().unwrap_or("HEAD"))?;
    let (head, named) = match &request.head {
        DiffHead::Revision(Some(revision)) => {
            let sha = commit_named(verbs, &place, revision)?;
            (DiffSide::Commit(sha.clone()), sha.as_str().to_string())
        }
        DiffHead::Revision(None) if request.base.is_some() => {
            let sha = commit_named(verbs, &place, "HEAD")?;
            (DiffSide::Commit(sha.clone()), sha.as_str().to_string())
        }
        DiffHead::Staged => (DiffSide::Index, "index".to_string()),
        DiffHead::Revision(None) | DiffHead::Worktree => {
            (DiffSide::Worktree, "worktree".to_string())
        }
    };
    if request.merge_base {
        let against = match &head {
            DiffSide::Commit(sha) => sha.clone(),
            DiffSide::Index | DiffSide::Worktree => commit_named(verbs, &place, "HEAD")?,
        };
        base = verbs
            .ports()
            .git
            .merge_base(&place, &base, &against)
            .map_err(failed)?
            .ok_or_else(|| {
                invalid(GitError(format!(
                    "{} and {} share no history: there is no merge base to compare from",
                    base.short(),
                    against.short()
                )))
            })?;
    }
    let query = DiffQuery {
        base: base.clone(),
        head,
        paths: request.paths.clone(),
        patch: request.patch,
    };
    let files = verbs.ports().git.diff(&place, &query).map_err(failed)?;
    let text = || files.iter().filter(|file| !file.binary);
    Ok(DiffAnswer {
        base: base.as_str().to_string(),
        head: named,
        count: files.len(),
        added: text().map(|file| u64::from(file.added)).sum(),
        removed: text().map(|file| u64::from(file.removed)).sum(),
        files,
    })
}

pub fn merge_base(verbs: &Verbs, a: &str, b: &str) -> Result<MergeBaseAnswer, OpError> {
    let place = verbs.place("merge-base")?;
    let (a, b) = (
        commit_named(verbs, &place, a)?,
        commit_named(verbs, &place, b)?,
    );
    let base = verbs
        .ports()
        .git
        .merge_base(&place, &a, &b)
        .map_err(failed)?;
    Ok(MergeBaseAnswer {
        sha: base.map(|sha| sha.as_str().to_string()),
    })
}

pub fn show(verbs: &Verbs, revision: &str) -> Result<ShowAnswer, OpError> {
    let place = verbs.place("show")?;
    let git = &verbs.ports().git;
    match git.resolve(&place, revision).map_err(failed)? {
        None => {
            return Err(OpError::new(
                OpErrorKind::InvalidInput,
                UNKNOWN_REVISION,
                format!("{revision} names nothing"),
            ));
        }
        Some((_, kind)) if kind != ObjectKind::Blob => {
            return Err(OpError::new(
                OpErrorKind::InvalidInput,
                NOT_A_BLOB,
                format!("{revision} names no file: give `<rev>:<path>`"),
            ));
        }
        Some(_) => {}
    }
    let Some(blob) = git.blob(&place, revision).map_err(failed)? else {
        return Err(failed(GitError(format!("git cannot give {revision}"))));
    };
    let binary = blob.bytes.as_deref().is_some_and(is_binary);
    Ok(ShowAnswer {
        sha: blob.sha.as_str().to_string(),
        bytes: blob.size,
        binary,
        truncated: blob.bytes.is_none(),
        text: blob
            .bytes
            .filter(|_| !binary)
            .and_then(|bytes| String::from_utf8(bytes).ok()),
    })
}

pub fn branches(verbs: &Verbs, remote: bool) -> Result<BranchesAnswer, OpError> {
    let place = verbs.place("branches")?;
    let branches = verbs.ports().git.branches(&place, remote).map_err(failed)?;
    Ok(BranchesAnswer { branches })
}

pub fn worktrees(verbs: &Verbs) -> Result<WorktreesAnswer, OpError> {
    let place = verbs.place("worktrees")?;
    let worktrees = verbs.ports().git.worktrees(&place).map_err(failed)?;
    Ok(WorktreesAnswer {
        worktrees: worktrees
            .into_iter()
            .map(|w| Worktree {
                current: w.path == place.toplevel,
                path: slashed(&w.path),
                branch: w.branch,
                sha: w.sha.map(|sha| sha.as_str().to_string()),
                main: w.main,
                locked: w.locked,
            })
            .collect(),
    })
}

pub fn remotes(verbs: &Verbs) -> Result<RemotesAnswer, OpError> {
    let place = verbs.place("remotes")?;
    let remotes = verbs.ports().git.remotes(&place).map_err(failed)?;
    Ok(RemotesAnswer { remotes })
}

pub fn submodules(verbs: &Verbs) -> Result<SubmodulesAnswer, OpError> {
    let place = verbs.place("submodules")?;
    let submodules = verbs.ports().git.submodules(&place).map_err(failed)?;
    Ok(SubmodulesAnswer { submodules })
}
