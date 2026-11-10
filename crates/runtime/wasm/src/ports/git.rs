//! The `git` port: read-only questions about the project's repository, performed in every mode and never journaled.

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use specforge_effects::contained::{RealFs, withheld};
use specforge_effects::git::{
    DiffQuery, DiffSide, DiffStatus, GitCommit, GitFileDiff, GitPerson, GitPlace, Located,
    LogQuery, Sha, Work,
};
use specforge_effects::{EffectRequest, FsOp, FsRequest, OutcomeClass, SharedGit};
use specforge_protocol_types::{
    FileSize, GitBlob, GitBlobs, GitChange, GitDiffAnswer, GitDiffHead, GitDiffQuery,
    GitFileChange, GitLogAnswer, GitLogCommit, GitLogQuery, GitRepo, GitSignature, GitStatus,
    GitTrailer, GitTreeEntry, HostError, Root,
};

use super::{NOT_A_REPOSITORY, PortSession, UNAVAILABLE, failed, out_of_scope};

/// A commit a guest names: hex, so it can be neither an option nor a revision expression.
pub(super) fn sha_of(text: &str) -> Result<Sha, HostError> {
    if !text.is_empty() && text.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(Sha::new(text))
    } else {
        Err(out_of_scope(format!("'{text}' is no commit id")))
    }
}

/// A path a guest names under the repository's top: relative, and never climbing.
pub(super) fn git_path(text: &str) -> Result<PathBuf, HostError> {
    let path = PathBuf::from(text);
    let normal = Path::new(text)
        .components()
        .all(|part| matches!(part, Component::Normal(_) | Component::CurDir));
    // A backslash is a separator on one platform and a name on another: what `withheld` judged may not be what opens.
    if normal && !text.contains('\\') {
        Ok(path)
    } else {
        Err(out_of_scope(format!(
            "'{text}' is not a path from the repository's top"
        )))
    }
}

impl PortSession {
    /// The git port and the project's directory, when the grant lets this call read git.
    fn git(&self, question: &str) -> Result<(SharedGit, &Path), HostError> {
        let (kind, call) = self.lent()?;
        call.grant
            .permits_git(kind, question)
            .map_err(|refusal| Self::refused(call, refusal))?;
        Ok((Arc::clone(&call.git), call.roots.project()))
    }

    /// The project's repository, for a question that needs one.
    pub(super) fn place(&self, question: &str) -> Result<(SharedGit, GitPlace), HostError> {
        let (git, dir) = self.git(question)?;
        match git.locate(dir) {
            Located::Repo(place) => Ok((git, place)),
            Located::NotARepository => Err(failed(
                NOT_A_REPOSITORY,
                format!("{} is in no git repository", dir.display()),
            )),
            Located::Unavailable(why) => Err(failed(UNAVAILABLE, why)),
        }
    }

    pub fn git_locate(&mut self) -> Result<Option<GitRepo>, HostError> {
        let (git, dir) = self.git("locate")?;
        match git.locate(dir) {
            Located::Repo(place) => Ok(Some(GitRepo {
                toplevel: place.toplevel.display().to_string(),
                shallow: place.shallow,
            })),
            Located::NotARepository => Ok(None),
            Located::Unavailable(why) => Err(failed(UNAVAILABLE, why)),
        }
    }

    pub fn git_commit(&mut self, revision: &str) -> Result<Option<String>, HostError> {
        // No option, and nothing but a name: `<revision>^{commit}` is what the port asks.
        if revision.is_empty()
            || revision.starts_with('-')
            || revision.chars().any(char::is_whitespace)
        {
            return Err(out_of_scope(format!("'{revision}' is no revision")));
        }
        let (git, place) = self.place("commit")?;
        git.commit(&place, revision).map(named).map_err(git_failed)
    }

    pub fn git_merge_base(&mut self, a: &str, b: &str) -> Result<Option<String>, HostError> {
        let (a, b) = (sha_of(a)?, sha_of(b)?);
        let (git, place) = self.place("merge-base")?;
        git.merge_base(&place, &a, &b)
            .map(named)
            .map_err(git_failed)
    }

    pub fn git_tree(&mut self, commit: &str, dir: &str) -> Result<Vec<GitTreeEntry>, HostError> {
        let (commit, dir) = (sha_of(commit)?, git_path(dir)?);
        let (git, place) = self.place("tree")?;
        let entries: Vec<GitTreeEntry> = git
            .tree(&place, &commit, &dir)
            .map_err(git_failed)?
            .into_iter()
            .map(|(path, file)| GitTreeEntry {
                path: posix(&path),
                file,
            })
            .collect();
        let bytes: u64 = entries.iter().map(|entry| entry.path.len() as u64).sum();
        self.fits(bytes, "a tree")?;
        Ok(entries)
    }

    pub fn git_sizes(
        &mut self,
        commit: &str,
        paths: &[String],
    ) -> Result<Vec<FileSize>, HostError> {
        let commit = sha_of(commit)?;
        let paths: Vec<PathBuf> = paths
            .iter()
            .map(|path| git_path(path))
            .collect::<Result<_, _>>()?;
        let (git, place) = self.place("sizes")?;
        Ok(git
            .sizes(&place, &commit, &paths)
            .map_err(git_failed)?
            .into_iter()
            .map(|(path, size)| FileSize {
                path: posix(&path),
                size,
            })
            .collect())
    }

    pub fn git_blobs(&mut self, commit: &str, paths: &[String]) -> Result<GitBlobs, HostError> {
        let commit = sha_of(commit)?;
        let mut wanted = Vec::with_capacity(paths.len());
        for path in paths {
            // A file the disk withholds is withheld at a commit too.
            if withheld(path) {
                return Err(out_of_scope(format!("`{path}` is withheld")));
            }
            wanted.push(git_path(path)?);
        }
        let (git, place) = self.place("blobs")?;
        let mut room = self.answer_cap;
        let mut truncated = false;
        let blobs = git
            .blobs(&place, &commit, &wanted)
            .map_err(git_failed)?
            .into_iter()
            .map(|(path, bytes)| {
                let bytes = bytes.filter(|bytes| {
                    let fits = !truncated && bytes.len() as u64 <= room;
                    truncated |= !fits;
                    room = room.saturating_sub(bytes.len() as u64);
                    fits
                });
                GitBlob {
                    path: posix(&path),
                    bytes,
                }
            })
            .collect();
        Ok(GitBlobs { blobs, truncated })
    }

    pub fn git_branch(&mut self) -> Result<String, HostError> {
        let (git, place) = self.place("branch")?;
        git.branch(&place).map_err(git_failed)
    }

    pub fn git_status(&mut self) -> Result<GitStatus, HostError> {
        let (git, place) = self.place("status")?;
        let status = git.status(&place).map_err(git_failed)?;
        let bytes: u64 = status
            .entries
            .iter()
            .map(|entry| entry.path.len() as u64)
            .sum();
        self.fits(bytes, "a status")?;
        Ok(status)
    }

    /// The log of the project's repository, as many commits as the answer's cap holds. A guest names commits by id and
    /// paths from the top; its pickaxe never reads a withheld file, and is refused one it names. The walk is counted
    /// work: it steps onto at most [`specforge_effects::git::GUEST_LOG_WALK`] commits and reads at most the answer's cap of commits and
    /// blobs, then stops, with no count, and the answer says `truncated`.
    pub fn git_log(&mut self, query: &GitLogQuery) -> Result<GitLogAnswer, HostError> {
        let shas = |names: &[String]| {
            names
                .iter()
                .map(|n| sha_of(n))
                .collect::<Result<Vec<_>, _>>()
        };
        let (tips, hidden) = (shas(&query.tips)?, shas(&query.hidden)?);
        let paths = repository_paths(&query.paths)?;
        if query.pickaxe.is_some()
            && let Some(path) = query.paths.iter().find(|path| withheld(path))
        {
            return Err(out_of_scope(format!(
                "`{path}` is withheld: a pickaxe does not read it"
            )));
        }
        let (git, place) = self.place("log")?;
        let log = git
            .log(
                &place,
                &LogQuery {
                    tips,
                    hidden,
                    paths,
                    first_parent: query.first_parent,
                    max: query.max.map(|max| max as usize),
                    oldest: query.oldest,
                    grep: query.grep.clone(),
                    since: query.since,
                    until: query.until,
                    no_merges: query.no_merges,
                    pickaxe: query.pickaxe.clone(),
                    withhold: true,
                    budget: Some(Work {
                        commits: self.log_walk,
                        bytes: self.answer_cap,
                    }),
                },
            )
            .map_err(git_failed)?;
        let mut room = self.answer_cap;
        let mut answer = GitLogAnswer {
            count: log.count.map(|count| count as u64),
            truncated: log.stopped,
            ..GitLogAnswer::default()
        };
        for commit in log.commits {
            let commit = log_commit(commit);
            let bytes = commit_bytes(&commit);
            if bytes > room {
                answer.truncated = true;
                break;
            }
            room -= bytes;
            answer.commits.push(commit);
        }
        Ok(answer)
    }

    /// A diff of the project's repository. A withheld file keeps its status and counts, never its patch nor its hunks;
    /// past the answer's cap the files keep no text, and the answer says `truncated`. The diff reads at most the
    /// answer's cap of its files (counted bytes): a file past it keeps its status, no counts, and says `truncated`, as
    /// does the answer. A worktree's text is the disk's:
    /// a file the call's fs grant does not read is withheld from a worktree diff ([`Self::reads_on_disk`]).
    pub fn git_diff(&mut self, query: &GitDiffQuery) -> Result<GitDiffAnswer, HostError> {
        let base = query.base.as_deref().map(sha_of).transpose()?;
        let head = match &query.head {
            GitDiffHead::Commit(sha) => DiffSide::Commit(sha_of(sha)?),
            GitDiffHead::Index => DiffSide::Index,
            GitDiffHead::Worktree => DiffSide::Worktree,
        };
        let paths = repository_paths(&query.paths)?;
        let (git, place) = self.place("diff")?;
        let files = git
            .diff(
                &place,
                &DiffQuery {
                    base,
                    head,
                    paths,
                    patch: query.patch,
                    hunks: query.hunks,
                    renames: query.renames,
                    budget: Some(self.answer_cap),
                },
            )
            .map_err(git_failed)?;
        let names: u64 = files
            .iter()
            .map(|file| (file.path.len() + file.old_path.as_ref().map_or(0, String::len)) as u64)
            .sum();
        self.fits(names, "a diff's paths")?;
        let mut room = self.answer_cap - names;
        let mut answer = GitDiffAnswer::default();
        let on_disk = query.head == GitDiffHead::Worktree;
        for file in files {
            answer.truncated |= file.unread;
            let mut change = file_change(file);
            if on_disk && !change.withheld && !self.reads_on_disk(&place.toplevel, &change.path) {
                change.withheld = true;
                change.patch = None;
                change.hunks = None;
            }
            let bytes = change.patch.as_ref().map_or(0, String::len) as u64
                + change.hunks.as_ref().map_or(0, Vec::len) as u64 * 16;
            if answer.truncated || bytes > room {
                answer.truncated |= bytes > 0;
                change.patch = None;
                change.hunks = None;
            } else {
                room -= bytes;
            }
            answer.files.push(change);
        }
        Ok(answer)
    }

    /// Whether this call's fs grant reads the file at `path` from the repository's top `top`, as an fs read of it would
    /// be judged: a root of the call holds it, through every link on its way (a linked directory leading out of the
    /// root does not), and the grant lets a read of it in that root. No denial is reported: the diff still names the
    /// file, with its status and counts.
    fn reads_on_disk(&self, top: &Path, path: &str) -> bool {
        let Ok((kind, call)) = self.lent() else {
            return false;
        };
        let at = top.join(path);
        let roots = [Root::Project, Root::Spec].into_iter().chain(
            call.roots
                .named()
                .map(|(name, _)| Root::Named(name.to_string())),
        );
        roots.into_iter().any(|root| {
            let Some(base) = call.roots.get(&root) else {
                return false;
            };
            let Some(rel) = at
                .strip_prefix(base)
                .or_else(|_| at.strip_prefix(RealFs::landing(base)))
                .ok()
                .map(posix)
            else {
                return false;
            };
            let request = |op| FsRequest {
                root: root.to_string(),
                path: rel.clone(),
                op,
            };
            let read = EffectRequest::Fs(request(FsOp::Read { max_bytes: 0 }));
            call.grant.permits(kind, &read).is_ok()
                && call.fs.run(&call.roots, &request(FsOp::Stat)).class == OutcomeClass::Ok
        })
    }

    /// A listing that has no way to say it was cut fails `capped` past the answer's cap.
    fn fits(&self, bytes: u64, what: &str) -> Result<(), HostError> {
        if bytes > self.answer_cap {
            return Err(failed(
                OutcomeClass::Capped,
                format!(
                    "{what} of {bytes} bytes is past this call's {} byte answer",
                    self.answer_cap
                ),
            ));
        }
        Ok(())
    }
}

/// The paths a guest names, each from the repository's top and never climbing.
fn repository_paths(paths: &[String]) -> Result<Vec<String>, HostError> {
    paths
        .iter()
        .map(|path| git_path(path).map(|path| posix(&path)))
        .collect()
}

fn signature(person: GitPerson) -> GitSignature {
    GitSignature {
        name: person.name,
        email: person.email,
        time: person.time,
        seconds: person.seconds,
    }
}

fn log_commit(commit: GitCommit) -> GitLogCommit {
    GitLogCommit {
        sha: commit.sha.as_str().to_string(),
        parents: commit
            .parents
            .iter()
            .map(|sha| sha.as_str().to_string())
            .collect(),
        author: signature(commit.author),
        committer: signature(commit.committer),
        subject: commit.subject,
        body: commit.body,
        trailers: commit
            .trailers
            .into_iter()
            .map(|(token, values)| GitTrailer { token, values })
            .collect(),
    }
}

/// The bytes a commit takes of an answer: its text, counted as the guest receives it.
fn commit_bytes(commit: &GitLogCommit) -> u64 {
    let person = |p: &GitSignature| p.name.len() + p.email.len() + p.time.len() + 8;
    let trailers: usize = commit
        .trailers
        .iter()
        .map(|t| t.token.len() + t.values.iter().map(String::len).sum::<usize>())
        .sum();
    (commit.sha.len()
        + commit.parents.iter().map(String::len).sum::<usize>()
        + person(&commit.author)
        + person(&commit.committer)
        + commit.subject.len()
        + commit.body.len()
        + trailers) as u64
}

/// A changed file as a guest reads it: a withheld one (either path) without its text.
fn file_change(file: GitFileDiff) -> GitFileChange {
    let held = withheld(&file.path) || file.old_path.as_deref().is_some_and(withheld);
    GitFileChange {
        path: file.path,
        old_path: file.old_path,
        status: match file.status {
            DiffStatus::Added => GitChange::Added,
            DiffStatus::Deleted => GitChange::Deleted,
            DiffStatus::Modified => GitChange::Modified,
            DiffStatus::Renamed => GitChange::Renamed,
            DiffStatus::Copied => GitChange::Copied,
            DiffStatus::TypeChanged => GitChange::TypeChanged,
        },
        added: file.added,
        removed: file.removed,
        binary: file.binary,
        truncated: file.truncated,
        withheld: held,
        patch: file.patch.filter(|_| !held),
        hunks: file.hunks.filter(|_| !held),
    }
}

/// A commit as a guest reads it.
fn named(sha: Option<Sha>) -> Option<String> {
    sha.map(|sha| sha.as_str().to_string())
}

/// A repository path as a guest reads it, whatever the platform's separator.
pub(super) fn posix(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub(super) fn git_failed(error: specforge_effects::git::GitError) -> HostError {
    failed(UNAVAILABLE, error.0)
}

#[cfg(test)]
mod tests;
