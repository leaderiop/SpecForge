//! `specforge git` (ADR 0339): the `Git` port for people and agents, on the repository the project is in, under the
//! project's effects ceiling (`git` read, and `write` for a change). Everything runs in process: no git program runs.

use serde::Serialize;
use specforge_effects::git::{
    DiffQuery, DiffSide, GitBranchRef, GitCommit, GitError, GitFileDiff, GitPlace, GitRemote,
    GitStatus, Located, LogQuery, ObjectKind, Sha,
};

use super::{USER, Verbs, slashed};
use crate::{OpError, OpErrorKind};

mod branching;
mod commit;
mod merging;

pub use branching::{
    PrunedAnswer, RefAnswer, branch_create, branch_delete, switch, tag, worktree_add,
    worktree_prune, worktree_remove,
};
pub use commit::{CommitAnswer, CommitRequest, cleaned, commit};
pub use merging::{
    FastForward, MergeAnswer, ResetAnswer, RestoreAnswer, StashAnswer, StashesAnswer, merge, pick,
    reset, restore, stash_list, stash_pop, stash_push,
};
pub use specforge_effects::git::ResetMode;

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
    /// The branch's upstream, `origin/main`.
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
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

/// A revision that names no commit: the code `check --since` refuses one with.
const UNKNOWN_REVISION: &str = "unknown_revision";

/// `git log`'s answer: the commits, newest first, and how many (`rev-list --count`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LogAnswer {
    pub count: usize,
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
}

/// `git diff`'s answer: the base commit, the head (a commit, `index` or `worktree`) and the changed files by path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiffAnswer {
    pub base: String,
    pub head: String,
    pub count: usize,
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
    pub bytes: usize,
    /// A NUL in the first 8000 bytes, as git decides.
    pub binary: bool,
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

    /// The repository the project is in, once the ceiling lets git be asked `question`.
    fn place(&self, question: &str) -> Result<GitPlace, OpError> {
        self.permit_git(question)?;
        match self.ports().git.locate(self.root().dir()) {
            Located::Repo(place) => Ok(place),
            Located::NotARepository => Err(OpError::new(
                OpErrorKind::PreconditionFailed,
                "not_a_repository",
                format!("{} is in no git repository", self.root().dir().display()),
            )),
            Located::Unavailable(why) => Err(OpError::new(
                OpErrorKind::PreconditionFailed,
                "git_unavailable",
                format!("git cannot read the repository: {why}"),
            )),
        }
    }
}

/// A port's failure as the verb's.
fn failed(error: GitError) -> OpError {
    OpError::new(OpErrorKind::Internal, "git_failed", error.0)
}

pub fn repo(verbs: &Verbs) -> Result<RepoAnswer, OpError> {
    let place = verbs.place("repo")?;
    Ok(RepoAnswer {
        top: slashed(&place.toplevel),
        git_dir: slashed(&place.git_dir),
        common_dir: slashed(&place.common_dir),
        linked: place.git_dir != place.common_dir,
        shallow: place.shallow,
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
        upstream: branch.upstream,
        ahead: branch.ahead,
        behind: branch.behind,
    })
}

pub fn revs(verbs: &Verbs, asked: &[String]) -> Result<RevsAnswer, OpError> {
    let place = verbs.place("rev")?;
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
    let untracked = status
        .entries
        .iter()
        .filter(|e| e.kind == specforge_effects::git::GitEntryKind::Untracked)
        .map(|e| e.path.clone())
        .collect();
    Ok(StatusAnswer {
        clean: !status.dirty(),
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
                let base = verbs
                    .ports()
                    .git
                    .merge_base(&place, &a, &b)
                    .map_err(failed)?;
                (vec![a, b], base.into_iter().collect())
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
            commits: Vec::new(),
        });
    }
    let query = LogQuery {
        tips,
        hidden,
        paths: request.paths.clone(),
        first_parent: request.first_parent,
        max: request.max,
    };
    let commits = verbs.ports().git.log(&place, &query).map_err(failed)?;
    Ok(LogAnswer {
        count: commits.len(),
        commits,
    })
}

pub fn diff(verbs: &Verbs, request: &DiffRequest) -> Result<DiffAnswer, OpError> {
    let place = verbs.place("diff")?;
    let base = commit_named(verbs, &place, request.base.as_deref().unwrap_or("HEAD"))?;
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
    let query = DiffQuery {
        base: base.clone(),
        head,
        paths: request.paths.clone(),
        patch: request.patch,
    };
    let files = verbs.ports().git.diff(&place, &query).map_err(failed)?;
    Ok(DiffAnswer {
        base: base.as_str().to_string(),
        head: named,
        count: files.len(),
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
                "not_a_blob",
                format!("{revision} names no file: give `<rev>:<path>`"),
            ));
        }
        Some(_) => {}
    }
    let Some((sha, bytes)) = git.blob(&place, revision).map_err(failed)? else {
        return Err(failed(GitError(format!("git cannot give {revision}"))));
    };
    let binary = bytes[..bytes.len().min(8000)].contains(&0);
    Ok(ShowAnswer {
        sha: sha.as_str().to_string(),
        bytes: bytes.len(),
        binary,
        text: if binary {
            None
        } else {
            String::from_utf8(bytes).ok()
        },
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
