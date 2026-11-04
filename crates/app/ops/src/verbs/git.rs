//! `specforge git` (ADR 0339): the `Git` port for people and agents, on the repository the project is in, under the
//! project's effects ceiling (`git` read). Every answer is read in process: no git program runs.

use serde::Serialize;
use specforge_effects::git::{GitError, GitPlace, GitStatus, Located, ObjectKind};

use super::{USER, Verbs, slashed};
use crate::{OpError, OpErrorKind};

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

impl Verbs {
    /// Whether the user grant lets git be asked `question`: `Err` is E090 with the ceiling's words.
    pub fn permit_git(&self, question: &str) -> Result<(), OpError> {
        self.grant
            .permits_git(USER, question)
            .map_err(|denial| super::outside_ceiling(&denial.request, &denial))
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
