//! The branches, worktrees and remotes `Gix` lists.

use gix::bstr::ByteSlice as _;
use specforge_effects::git::{
    GitBranch, GitBranchRef, GitError, GitRemote, GitWorktree, Sha, Tracking,
};

use crate::failed;

/// The local branches, or the remote-tracking ones, by name.
pub(crate) fn branches(
    repo: &gix::Repository,
    remote: bool,
) -> Result<Vec<GitBranchRef>, GitError> {
    let current = repo
        .head_name()
        .map_err(|e| failed("read HEAD", &e))?
        .map(|name| name.as_bstr().to_owned());
    let platform = repo
        .references()
        .map_err(|e| failed("list the references", &e))?;
    let listed = if remote {
        platform.remote_branches()
    } else {
        platform.local_branches()
    }
    .map_err(|e| failed("list the branches", &e))?;
    let mut branches = Vec::new();
    for reference in listed {
        let mut reference = reference.map_err(|e| failed("list the branches", &e))?;
        // the name before peeling: peeling a symbolic ref turns it into its target
        let name = reference.name().to_owned();
        // a remote's `HEAD` names its default branch, not a branch of its own
        if remote && name.as_bstr().ends_with(b"/HEAD") {
            continue;
        }
        let tip = reference.peel_to_id().ok().map(|id| id.detach());
        let tracking = if remote {
            Tracking::default()
        } else {
            upstream_of(repo, name.as_ref(), tip)?
        };
        branches.push(GitBranchRef {
            name: name.shorten().to_string(),
            sha: tip.map(|id| Sha::new(id.to_string())),
            current: current.as_ref().is_some_and(|c| c == name.as_bstr()),
            tracking,
        });
    }
    branches.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(branches)
}

/// The main worktree, then the linked ones by path.
pub(crate) fn worktrees(repo: &gix::Repository) -> Result<Vec<GitWorktree>, GitError> {
    let main = repo
        .main_repo()
        .map_err(|e| failed("open the main worktree", &e))?;
    let mut worktrees = Vec::new();
    if let Some(top) = main.workdir() {
        worktrees.push(worktree_of(&main, top.to_path_buf(), true, false)?);
    }
    let mut linked = Vec::new();
    for proxy in main
        .worktrees()
        .map_err(|e| failed("list the worktrees", &e))?
    {
        let locked = proxy.is_locked();
        let Ok(path) = proxy.base() else {
            continue;
        };
        let there = proxy
            .into_repo_with_possibly_inaccessible_worktree()
            .map_err(|e| failed("open a linked worktree", &e))?;
        linked.push(worktree_of(&there, path, false, locked)?);
    }
    linked.sort_by(|a, b| a.path.cmp(&b.path));
    worktrees.extend(linked);
    Ok(worktrees)
}

/// One worktree as the list names it.
fn worktree_of(
    repo: &gix::Repository,
    path: std::path::PathBuf,
    main: bool,
    locked: bool,
) -> Result<GitWorktree, GitError> {
    Ok(GitWorktree {
        path: path.canonicalize().unwrap_or(path),
        branch: local_branch(repo)?,
        sha: repo.head_id().ok().map(|id| Sha::new(id.to_string())),
        main,
        locked,
    })
}

/// The remotes, by name, with where each URL points.
pub(crate) fn remotes(repo: &gix::Repository) -> Vec<GitRemote> {
    let mut remotes = Vec::new();
    for name in repo.remote_names() {
        let Ok(remote) = repo.find_remote(name.as_ref()) else {
            continue;
        };
        let Some(url) = remote.url(gix::remote::Direction::Fetch) else {
            continue;
        };
        let push = remote.url(gix::remote::Direction::Push).unwrap_or(url);
        let (host, path) = split(url);
        remotes.push(GitRemote {
            url: shown(url),
            push_url: shown(push),
            host,
            path,
            head: default_branch(repo, name.as_ref()),
            name: name.to_string(),
        });
    }
    remotes
}

/// The branch `refs/remotes/<remote>/HEAD` points at, under the remote's own refs; none when it is absent, detached
/// or points elsewhere.
fn default_branch(repo: &gix::Repository, remote: &gix::bstr::BStr) -> Option<String> {
    let at = format!("refs/remotes/{remote}/");
    let head = repo.try_find_reference(&format!("{at}HEAD")).ok()??;
    let target = head
        .target()
        .try_name()?
        .as_bstr()
        .to_str()
        .ok()?
        .to_owned();
    target.strip_prefix(&at).map(str::to_string)
}

/// `url` without its credentials: never a password, and no user on http(s), where a token rides as the user; an ssh
/// login name is kept, since it is no secret and the URL needs it.
fn shown(url: &gix::Url) -> String {
    let mut url = url.clone();
    url.set_password(None);
    if matches!(url.scheme, gix::url::Scheme::Http | gix::url::Scheme::Https) {
        url.set_user(None);
    }
    url.to_bstring().to_string()
}

/// A URL's host and path: `owner/repo` (no leading slash, no `.git`) for one with a host, the path itself for a
/// local one.
fn split(url: &gix::Url) -> (Option<String>, String) {
    let path = url.path.to_str_lossy();
    match url.host() {
        Some(host) => {
            let path = path.trim_start_matches('/');
            let path = path.strip_suffix(".git").unwrap_or(path);
            (
                Some(host.to_string()),
                path.trim_end_matches('/').to_string(),
            )
        }
        None => (None, path.into_owned()),
    }
}

/// The branch `HEAD` is on, born or not; `None` when it is detached.
pub(crate) fn local_branch(repo: &gix::Repository) -> Result<Option<String>, GitError> {
    let name = repo.head_name().map_err(|e| failed("read HEAD", &e))?;
    Ok(name
        .filter(|name| name.category() == Some(gix::refs::Category::LocalBranch))
        .map(|name| name.shorten().to_string()))
}

/// `status`'s branch headers: `HEAD`'s commit and branch, its upstream, and how far each is ahead of the other.
pub(crate) fn branch_of(repo: &gix::Repository) -> Result<GitBranch, GitError> {
    let oid = repo.head_id().ok().map(|id| id.detach());
    let mut branch = GitBranch {
        oid: oid.map(|id| id.to_string()),
        head: local_branch(repo)?,
        ..GitBranch::default()
    };
    if let Some(name) = repo.head_name().map_err(|e| failed("read HEAD", &e))? {
        let tracking = upstream_of(repo, name.as_ref(), oid)?;
        (branch.upstream, branch.ahead, branch.behind) =
            (tracking.upstream, tracking.ahead, tracking.behind);
    }
    Ok(branch)
}

/// The upstream of the local branch `name` at `tip`, and how many commits each is ahead of the other.
pub(crate) fn upstream_of(
    repo: &gix::Repository,
    name: &gix::refs::FullNameRef,
    tip: Option<gix::ObjectId>,
) -> Result<Tracking, GitError> {
    let Some(Ok(tracking)) =
        repo.branch_remote_tracking_ref_name(name, gix::remote::Direction::Fetch)
    else {
        return Ok(Tracking::default());
    };
    let upstream = repo
        .try_find_reference(tracking.as_ref())
        .ok()
        .flatten()
        .and_then(|mut r| r.peel_to_id().ok())
        .map(|id| id.detach());
    let (ahead, behind) = match (tip, upstream) {
        (Some(tip), Some(upstream)) => (
            count_hiding(repo, tip, upstream)?,
            count_hiding(repo, upstream, tip)?,
        ),
        _ => (0, 0),
    };
    Ok(Tracking {
        upstream: Some(tracking.shorten().to_string()),
        ahead,
        behind,
    })
}

/// How many commits `tip` reaches that `hidden` does not.
fn count_hiding(
    repo: &gix::Repository,
    tip: gix::ObjectId,
    hidden: gix::ObjectId,
) -> Result<u32, GitError> {
    let walk = repo
        .rev_walk([tip])
        .with_hidden([hidden])
        .all()
        .map_err(|e| failed("walk the history", &e))?;
    let mut count = 0u32;
    for info in walk {
        info.map_err(|e| failed("walk the history", &e))?;
        count = count.saturating_add(1);
    }
    Ok(count)
}
