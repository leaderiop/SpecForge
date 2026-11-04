//! The `git` port: read-only questions about the project's repository, performed in every mode and never journaled.

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use specforge_effects::contained::withheld;
use specforge_effects::git::{GitPlace, Located, Sha};
use specforge_effects::{OutcomeClass, SharedGit};
use specforge_protocol_types::{
    FileSize, GitBlob, GitBlobs, GitRepo, GitStatus, GitTreeEntry, HostError,
};

use super::{NOT_A_REPOSITORY, PortSession, UNAVAILABLE, failed, out_of_scope};

/// A commit a guest names: hex, so it can be neither an option nor a revision expression.
fn sha_of(text: &str) -> Result<Sha, HostError> {
    if !text.is_empty() && text.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(Sha::new(text))
    } else {
        Err(out_of_scope(format!("'{text}' is no commit id")))
    }
}

/// A path a guest names under the repository's top: relative, and never climbing.
fn git_path(text: &str) -> Result<PathBuf, HostError> {
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
    fn place(&self, question: &str) -> Result<(SharedGit, GitPlace), HostError> {
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

/// A commit as a guest reads it.
fn named(sha: Option<Sha>) -> Option<String> {
    sha.map(|sha| sha.as_str().to_string())
}

/// A repository path as a guest reads it, whatever the platform's separator.
fn posix(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn git_failed(error: specforge_effects::git::GitError) -> HostError {
    failed(UNAVAILABLE, error.0)
}
