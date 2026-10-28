//! One change to project files, written all or nothing (ADR 0128): each file is staged with what it holds and what it
//! will hold, checked, replaced in stage order, and put back in reverse when anything fails. What the change left is
//! its failure's writes, so an operation's error can never drop it (ADR 0022 D4).
//!
//! A failure is worded here, once: one code and kind per cause ([`FILE_CHANGED`], [`FILE_UNREADABLE`],
//! [`FILE_UNWRITABLE`]), the file named from the root the change was opened under, after what the operation was
//! doing, and a tail saying whether anything was left (ADR 0155).
//!
//! The four steps of [`ProjectChange::commit_then`]:
//! 1. **Check.** Each staged file is read through its [`Held`], in stage order, and compared with the bytes it was
//!    staged over. Any difference is [`FILE_CHANGED`]; a read that fails is [`FILE_UNREADABLE`]. Nothing has
//!    been written. An owned name (a migration's backup) is never read.
//! 2. **Write**, in stage order. The missing directories are made (and remembered). A file that was there is
//!    [`Held::replace`]d, so it holds the new content or the old, never part of either; one that was not is
//!    [`Held::create_new`]d, so a file that appeared since the check is changed. A write that fails is
//!    [`FILE_UNWRITABLE`]. An owned name is [`Held::create_owned`]d: whatever is at it is unlinked, and it is
//!    created new with the permissions it was staged with.
//! 3. **Then.** The caller's last step runs once every file is written.
//! 4. **Put back**, on any failure, in reverse stage order: a file that was there gets its bytes back, one that was
//!    not is unlinked where it landed, and the directories the change made are removed when empty. A file that
//!    cannot be put back is named in the message and is the error's writes. An owned name is unlinked; what was
//!    there is not restored (it was the operation's, and superseded).

use std::path::{Path, PathBuf};

use crate::{Held, OpError, OpErrorKind, Writes};

/// A file an operation planned on no longer holds what it read: nothing was written.
pub const FILE_CHANGED: &str = "file_changed";
/// A project file an operation must read could not be read.
pub const FILE_UNREADABLE: &str = "file_unreadable";
/// A project file an operation must write could not be written.
pub const FILE_UNWRITABLE: &str = "file_unwritable";

/// Project files staged to be written together.
pub(crate) struct ProjectChange {
    root: PathBuf,
    doing: String,
    staged: Vec<Staged>,
}

struct Staged {
    held: Held,
    was: Was,
    /// What the file will hold; `None`: the file is removed (an owned name never is).
    after: Option<Vec<u8>>,
}

/// What a staged file was when the change was planned.
enum Was {
    /// Not there: created new, unlinked on put-back.
    Missing,
    /// The bytes the caller planned on: checked, replaced, written back on put-back.
    Planned(Vec<u8>),
    /// A name the operation owns (a migration's backup): never read; written new at the name with these
    /// permissions (whatever is there, a link or a file, is unlinked first, never its target); unlinked on put-back.
    Owned(std::fs::Permissions),
}

/// What stopped a change. A file is named as its caller spelled it.
enum Cause {
    /// `file` no longer holds what it was staged over (or appeared, or went): nothing, or no file but those put
    /// back, was written.
    Changed { file: PathBuf },
    /// `file` could not be read to check it: nothing was written.
    Unreadable {
        file: PathBuf,
        error: std::io::Error,
    },
    /// `file` could not be written; it is as it was, and every file before it was put back but `left`.
    Unwritable {
        file: PathBuf,
        error: std::io::Error,
    },
    /// The caller's last step failed; every file was put back but `left`.
    Then(OpError),
}

impl ProjectChange {
    /// A change whose failures name its files from `root` (the spec root for sources, the project root for a lock
    /// plan), after `doing`, what the operation was doing (`cannot rename 'a'`).
    pub(crate) fn new(root: &Path, doing: impl Into<String>) -> Self {
        ProjectChange {
            root: root.to_path_buf(),
            doing: doing.into(),
            staged: Vec::new(),
        }
    }

    /// Stage `held` to hold `after`, over `before`: the bytes the caller planned on, or `None` for a file that is
    /// not there. A file whose `after` is its `before` is not staged. Each `real` is staged at most once.
    pub(crate) fn stage(&mut self, held: Held, before: Option<Vec<u8>>, after: Vec<u8>) {
        debug_assert!(
            self.staged
                .iter()
                .all(|s| matches!(s.was, Was::Owned(_)) || s.held.real != held.real),
            "{} is staged twice",
            held.at.display()
        );
        if before.as_deref() == Some(after.as_slice()) {
            return;
        }
        self.staged.push(Staged {
            held,
            was: before.map_or(Was::Missing, Was::Planned),
            after: Some(after),
        });
    }

    /// Stage the removal of `held`, a file that held `before` when the caller planned on it (ADR 0292: install
    /// removes what it marked). Checked like a replacement; put back by writing `before` again. Each `real` is staged
    /// at most once.
    pub(crate) fn stage_removal(&mut self, held: Held, before: Vec<u8>) {
        debug_assert!(
            self.staged
                .iter()
                .all(|s| matches!(s.was, Was::Owned(_)) || s.held.real != held.real),
            "{} is staged twice",
            held.at.display()
        );
        self.staged.push(Staged {
            held,
            was: Was::Planned(before),
            after: None,
        });
    }

    /// Stage a name the operation owns (see [`Was::Owned`]) to hold `after`, with `permissions`. Each owned
    /// `at` is staged at most once.
    pub(crate) fn stage_owned(
        &mut self,
        held: Held,
        after: Vec<u8>,
        permissions: std::fs::Permissions,
    ) {
        debug_assert!(
            self.staged.iter().all(|s| s.held.at != held.at),
            "{} is staged twice",
            held.at.display()
        );
        self.staged.push(Staged {
            held,
            was: Was::Owned(permissions),
            after: Some(after),
        });
    }

    /// [`Self::commit_then`] with no last step.
    pub(crate) fn commit(self) -> Result<Writes, OpError> {
        self.commit_then(|| Ok(()))
    }

    /// Check, write, then run `then`; see the module docs for the order and what a failure puts back. On success
    /// the writes are every staged file, as spelled. On failure the error is worded as the module docs say; `then`'s
    /// own error is returned as it is, with the files left.
    pub(crate) fn commit_then(
        self,
        then: impl FnOnce() -> Result<(), OpError>,
    ) -> Result<Writes, OpError> {
        for staged in &self.staged {
            let expected = match &staged.was {
                Was::Owned(_) => continue,
                Was::Missing => None,
                Was::Planned(bytes) => Some(bytes),
            };
            match staged.held.read_if_there() {
                Ok(now) if now.as_ref() == expected => {}
                Ok(_) => {
                    return Err(self.failed(
                        Cause::Changed {
                            file: staged.held.at.clone(),
                        },
                        Writes::none(),
                    ));
                }
                Err(error) => {
                    return Err(self.failed(
                        Cause::Unreadable {
                            file: staged.held.at.clone(),
                            error,
                        },
                        Writes::none(),
                    ));
                }
            }
        }

        let mut written = 0;
        let mut made = Vec::new();
        for staged in &self.staged {
            if let Err(cause) = Self::write(staged, &mut made) {
                return Err(self.put_back(written, &made, cause));
            }
            written += 1;
        }
        match then() {
            Ok(()) => Ok(self
                .staged
                .into_iter()
                .map(|staged| staged.held.at)
                .collect()),
            Err(e) => Err(self.put_back(written, &made, Cause::Then(e))),
        }
    }

    /// One staged file written; `made` collects the directories created for it.
    fn write(staged: &Staged, made: &mut Vec<PathBuf>) -> Result<(), Cause> {
        let held = &staged.held;
        let file = || held.at.clone();
        let unwritable = |error| Cause::Unwritable {
            file: file(),
            error,
        };
        // An owned name lies beside a file that is there: its directory exists.
        if !matches!(staged.was, Was::Owned(_)) {
            held.create_parents(made).map_err(unwritable)?;
        }
        let after = staged.after.as_deref().unwrap_or_default();
        match &staged.was {
            Was::Owned(permissions) => held
                .create_owned(after, permissions.clone())
                .map_err(unwritable),
            Was::Planned(_) if staged.after.is_none() => held.remove().map_err(unwritable),
            Was::Planned(_) => held.replace(after).map_err(unwritable),
            Was::Missing => match held.create_new(after) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    Err(Cause::Changed { file: file() })
                }
                Err(error) => {
                    // The open may have made the file before the write failed.
                    let _ = held.remove_created();
                    Err(Cause::Unwritable {
                        file: file(),
                        error,
                    })
                }
            },
        }
    }

    /// Put back the first `written` files in reverse, then remove `made` when empty.
    fn put_back(self, written: usize, made: &[PathBuf], cause: Cause) -> OpError {
        let mut left = Writes::none();
        for staged in self.staged[..written].iter().rev() {
            let held = &staged.held;
            let not_found_passes = |result: std::io::Result<()>| match result {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
                _ => Ok(()),
            };
            let restored = match &staged.was {
                Was::Planned(before) => match held.read_if_there() {
                    Ok(Some(now)) if now == *before => Ok(()),
                    // A removed file is written again.
                    Ok(None) => held.create_parent().and_then(|()| held.create_new(before)),
                    _ => held.replace(before),
                },
                Was::Missing => not_found_passes(held.remove_created()),
                Was::Owned(_) => not_found_passes(held.remove()),
            };
            if restored.is_err() {
                left.record(held.at.clone());
            }
        }
        let deepest_first: Vec<PathBuf> = made.iter().rev().cloned().collect();
        Held::remove_empty(&deepest_first);
        self.failed(cause, left)
    }

    /// `file` as the operation names it: its project path (ADR 0102).
    fn named(&self, file: &Path) -> String {
        specforge_common::ProjectPath::of(&self.root, file).into_string()
    }

    /// The one wording of a failure (ADR 0155); `left` is what could not be put back, and the error's writes.
    fn failed(&self, cause: Cause, left: Writes) -> OpError {
        let (kind, code, what) = match cause {
            Cause::Changed { file } => (
                OpErrorKind::Conflict,
                FILE_CHANGED,
                format!("{} changed since it was read", self.named(&file)),
            ),
            Cause::Unreadable { file, error } => (
                OpErrorKind::of_io(&error),
                FILE_UNREADABLE,
                format!("failed to read {}: {error}", self.named(&file)),
            ),
            Cause::Unwritable { file, error } => (
                OpErrorKind::of_io(&error),
                FILE_UNWRITABLE,
                format!("failed to write {}: {error}", self.named(&file)),
            ),
            Cause::Then(error) => return error.with_writes(left),
        };
        let left_names: Vec<String> = left.paths().map(|file| self.named(file)).collect();
        let error = refusal(kind, code, &what, &left_names);
        let error = if self.doing.is_empty() {
            error
        } else {
            error.prefixed(format!("{}: ", self.doing))
        };
        error.with_writes(left)
    }
}

/// `what`, then how the change was left: nothing written, or the `left` files that could not be put back.
fn refusal(kind: OpErrorKind, code: &'static str, what: &str, left: &[String]) -> OpError {
    let tail = if left.is_empty() {
        "nothing was written".to_string()
    } else {
        format!("{} could not be put back", left.join(", "))
    };
    OpError::new(kind, code, format!("{what}; {tail}"))
}

/// `file` changed since the operation read it, found before anything was staged: Conflict, `file_changed`.
pub(crate) fn changed(file: &Path) -> OpError {
    refusal(
        OpErrorKind::Conflict,
        FILE_CHANGED,
        &format!("{} changed since it was read", file.display()),
        &[],
    )
}

/// `file` could not be read before anything was staged: the OS error's kind, `file_unreadable`.
pub(crate) fn unreadable(file: &Path, error: &std::io::Error) -> OpError {
    refusal(
        OpErrorKind::of_io(error),
        FILE_UNREADABLE,
        &format!("failed to read {}: {error}", file.display()),
        &[],
    )
}

/// `file` could not be removed by an operation's last step: the OS error's kind, `file_unwritable`.
pub(crate) fn unwritable(file: &Path, error: &std::io::Error) -> OpError {
    refusal(
        OpErrorKind::of_io(error),
        FILE_UNWRITABLE,
        &format!("failed to write {}: {error}", file.display()),
        &[],
    )
}

pub(crate) mod journal;
pub(crate) mod locked;
#[cfg(test)]
mod owned_tests;
#[cfg(test)]
mod removal_tests;
#[cfg(test)]
mod tests;

/// The project's write lock when `serialized` (its extensions declare a harness field, a writing command or
/// collector, or a guard pass, ADR 0289); none otherwise, so a plain project gets no `.specforge/` and a read-only one
/// still formats.
pub(crate) fn lock_if(serialized: bool, root: &Path) -> std::io::Result<Option<std::fs::File>> {
    serialized
        .then(|| specforge_project::harness::lock(root))
        .transpose()
}
