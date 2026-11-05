//! `specforge git commit` (ADR 0339 D5): stage what the user names, run the repository's own `pre-commit`,
//! `prepare-commit-msg`, `commit-msg` and `post-commit` hooks through the Process port, and record the index as a
//! commit by the identity git's configuration names. The git program is never run.

use std::path::{Path, PathBuf};

use serde::Serialize;
use specforge_common::remedy::printable;
use specforge_effects::git::{GitError, GitPlace, Recording, Removal, Staging};
use specforge_effects::process::{OUTPUT_CAP, USER_CEILING};
use specforge_effects::{
    Ceiling, EffectRequest, EnvVar, Output, ProcessClass, ProcessRequest, Shown,
};

use super::super::{Access, Verbs};
use crate::{OpError, OpErrorKind};

/// What `git commit` is asked.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommitRequest {
    pub message: String,
    /// The commit whose message and author this one reuses (`-C`), in place of `message`.
    pub reuse_message: Option<String>,
    /// Commit only the files at or under these paths, read from the current directory, as the worktree has them;
    /// what else is staged stays staged (`git commit -- <paths>`).
    pub only: Vec<PathBuf>,
    /// Staged first, read from the current directory: the files at or under each, as the worktree has them.
    pub add: Vec<PathBuf>,
    /// Out of the index first (`git rm --cached`), the files left on disk.
    pub remove: Vec<PathBuf>,
    /// Every tracked file staged first as the worktree has it (`git commit -a`).
    pub all: bool,
    /// Stage an ignored path `add` names too (`git add --force`).
    pub force: bool,
    pub allow_empty: bool,
    /// Run no hook (`--no-verify`).
    pub no_verify: bool,
}

/// `git commit`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommitAnswer {
    pub sha: String,
    /// The branch it moved; none when `HEAD` was detached.
    pub branch: Option<String>,
    pub parents: Vec<String>,
    /// The paths it changed against its first parent, from the repository's top.
    pub files: Vec<String>,
}

/// A write the port refused or failed, its words from the repository made printable.
pub(super) fn write_failed(error: GitError) -> OpError {
    OpError::new(
        OpErrorKind::PreconditionFailed,
        super::GIT_REFUSED,
        printable(&error.0).into_owned(),
    )
}

pub fn commit(verbs: &Verbs, request: &CommitRequest) -> Result<CommitAnswer, OpError> {
    let place = verbs.place_to_write("commit")?;
    refuse_signing(verbs, &place, Signed::Commit)?;
    let git = &verbs.ports().git;
    let reused = request
        .reuse_message
        .as_deref()
        .map(|revision| super::commit_named(verbs, &place, revision))
        .transpose()?;
    let typed = match &reused {
        Some(sha) => git.message(&place, sha).map_err(write_failed)?,
        None => request.message.clone(),
    };
    let mut message = cleaned(&typed).ok_or_else(empty_message)?;
    let only = in_repository(verbs, &place, &request.only)?;
    let mut add = in_repository(verbs, &place, &request.add)?;
    add.extend(only.iter().cloned());
    let staging = Staging {
        add,
        remove: in_repository(verbs, &place, &request.remove)?,
        all: request.all,
        force: request.force,
        except: Vec::new(),
    };
    if staging != Staging::default() {
        git.stage(&place, &staging).map_err(write_failed)?;
    }
    let hooks = Hooks::of(verbs, &place)?;
    let verify = !request.no_verify;
    if verify {
        hooks.run("pre-commit", &[])?;
    }
    // As git: the message is written to COMMIT_EDITMSG whatever hooks exist, `prepare-commit-msg` is handed it as
    // a `message` (`-m`) even under --no-verify, and `commit-msg` then; what they leave is cleaned again.
    let file = git
        .write_commit_message(&place, &message)
        .map_err(write_failed)?
        .to_string_lossy()
        .into_owned();
    // as git hands the hook its source: `commit` and the commit for -C, else `message`
    let source = match &reused {
        Some(sha) => vec![file.clone(), "commit".to_string(), sha.as_str().to_string()],
        None => vec![file.clone(), "message".to_string()],
    };
    hooks.run("prepare-commit-msg", &source)?;
    if verify {
        hooks.run("commit-msg", &[file])?;
    }
    message = git
        .read_commit_message(&place)
        .map_err(write_failed)
        .and_then(|edited| cleaned(&edited).ok_or_else(empty_message))?;
    let committed = git
        .commit_index(
            &place,
            &message,
            &Recording {
                allow_empty: request.allow_empty,
                only,
                author_of: reused,
            },
        )
        .map_err(write_failed)?;
    if verify {
        // as git: the commit is made, whatever post-commit answers
        let _ = hooks.run("post-commit", &[]);
    }
    Ok(CommitAnswer {
        sha: committed.sha.as_str().to_string(),
        branch: committed.branch,
        parents: committed
            .parents
            .iter()
            .map(|p| p.as_str().to_string())
            .collect(),
        files: committed.files,
    })
}

/// What `git add` is asked.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AddRequest {
    /// Read from the current directory: the files at or under each, as the worktree has them.
    pub paths: Vec<PathBuf>,
    /// Every file under the top, untracked ones too, when no path is named (`git add -A`).
    pub all: bool,
    /// Left out, whatever else names them (`git add -- ':!path'`).
    pub except: Vec<PathBuf>,
    /// Stage an ignored path too (`git add --force`).
    pub force: bool,
}

/// `git add`: stage the files named as the worktree has them, a deleted one leaving the index, and answer the
/// status after.
pub fn add(verbs: &Verbs, request: &AddRequest) -> Result<super::StatusAnswer, OpError> {
    let place = verbs.place_to_write("add")?;
    let mut add = in_repository(verbs, &place, &request.paths)?;
    if add.is_empty() && request.all {
        add.push("*".to_string());
    }
    let staging = Staging {
        add,
        force: request.force,
        except: in_repository(verbs, &place, &request.except)?,
        ..Staging::default()
    };
    verbs
        .ports()
        .git
        .stage(&place, &staging)
        .map_err(write_failed)?;
    super::status(verbs)
}

/// `git rm`'s answer: the paths taken out of the index, from the repository's top.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RmAnswer {
    pub removed: Vec<String>,
}

/// `git rm`: the index entries at `paths` (read from the current directory) out of the index, and unless
/// `removal.cached` their files off the disk.
pub fn rm(verbs: &Verbs, paths: &[PathBuf], removal: Removal) -> Result<RmAnswer, OpError> {
    let place = verbs.place_to_write("rm")?;
    let paths = in_repository(verbs, &place, paths)?;
    let removed = verbs
        .ports()
        .git
        .remove(&place, &paths, removal)
        .map_err(write_failed)?;
    Ok(RmAnswer { removed })
}

/// `git hook run`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HookAnswer {
    /// The hook was there and ran (and passed); false only under `ignore_missing`.
    pub ran: bool,
}

/// `git hook run <name> -- <args>`: the repository's hook `name`, found and run as a commit runs its hooks. A hook
/// that is not there is refused unless `ignore_missing`; a name is a hook's file name, never a path.
pub fn hook_run(
    verbs: &Verbs,
    name: &str,
    args: &[String],
    ignore_missing: bool,
) -> Result<HookAnswer, OpError> {
    let named = !name.is_empty()
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if !named {
        return Err(OpError::new(
            OpErrorKind::InvalidInput,
            OpErrorKind::InvalidInput.as_str(),
            format!(
                "{} names no hook: a hook's name is its file's, such as pre-commit",
                printable(name)
            ),
        ));
    }
    let place = verbs.place_to_write("hook run")?;
    let ran = Hooks::of(verbs, &place)?.run_for(name, args, false)?;
    if !ran && !ignore_missing {
        return Err(OpError::new(
            OpErrorKind::PreconditionFailed,
            super::HOOK_CANNOT_RUN,
            format!("the repository has no {name} hook git would run"),
        )
        .with_suggestion("specforge git hook run --ignore-missing answers ran: false instead"));
    }
    Ok(HookAnswer { ran })
}

/// What git signs when the repository asks: every commit, or every annotated tag.
#[derive(Debug, Clone, Copy)]
pub(super) enum Signed {
    Commit,
    Tag,
}

/// specforge signs nothing: a repository that asks for `what` to be signed gets none rather than an unsigned one.
pub(super) fn refuse_signing(verbs: &Verbs, place: &GitPlace, what: Signed) -> Result<(), OpError> {
    let key = match what {
        Signed::Commit => "commit.gpgSign",
        Signed::Tag => "tag.gpgSign",
    };
    let signs = verbs
        .ports()
        .git
        .config_flag(place, key)
        .map_err(write_failed)?;
    if signs == Some(true) {
        let (kind, last) = match what {
            Signed::Commit => ("commit", "commit"),
            Signed::Tag => ("annotated tag", "tag"),
        };
        return Err(OpError::new(
            OpErrorKind::PreconditionFailed,
            super::SIGNING_UNSUPPORTED,
            format!(
                "the repository asks every {kind} to be signed ({key}), and specforge signs none"
            ),
        )
        .with_suggestion(format!(
            "sign it with the git program, or set {key} to false for this {last}'s repository"
        )));
    }
    Ok(())
}

fn empty_message() -> OpError {
    OpError::new(
        OpErrorKind::InvalidInput,
        OpErrorKind::InvalidInput.as_str(),
        "the commit message is empty",
    )
}

/// `message` as git keeps a message given with `-m` (its `whitespace` cleanup): each line's trailing whitespace
/// dropped, blank lines at either end dropped and runs of them made one, a newline at the end. Every other byte, a
/// trailer's included, is kept. `None` when nothing is left.
pub fn cleaned(message: &str) -> Option<String> {
    let mut out = String::new();
    let mut blank = false;
    for line in message.lines().map(str::trim_end) {
        if line.is_empty() {
            blank = !out.is_empty();
            continue;
        }
        if blank {
            out.push('\n');
            blank = false;
        }
        out.push_str(line);
        out.push('\n');
    }
    (!out.is_empty()).then_some(out)
}

/// Each typed path (read from the current directory) as a path from the repository's top: one the project's roots do
/// not hold for a write (outside them, or the host's own state), or one outside the working tree, is refused.
pub(super) fn in_repository(
    verbs: &Verbs,
    place: &GitPlace,
    typed: &[PathBuf],
) -> Result<Vec<String>, OpError> {
    typed
        .iter()
        .map(|path| {
            let at = held(verbs, path)?;
            let rel = at.strip_prefix(&place.toplevel).map_err(|_| {
                OpError::new(
                    OpErrorKind::InvalidInput,
                    super::OUTSIDE_REPOSITORY,
                    format!(
                        "{} is outside the repository at {}",
                        printable(&path.to_string_lossy()),
                        printable(&place.toplevel.to_string_lossy())
                    ),
                )
            })?;
            let rel = rel.to_string_lossy().replace('\\', "/");
            // the top itself: every path under it
            Ok(if rel.is_empty() { "*".to_string() } else { rel })
        })
        .collect()
}

/// Where `path` (read from the current directory) lands, once the project's roots hold it for a write and it is not
/// the host's own state or a credential file. shortcut: the name itself is checked, not each file under a directory
/// (`.specforge/` is git-ignored by `specforge init`); upgrade if a directory-wide write must skip them.
pub(super) fn held(verbs: &Verbs, path: &Path) -> Result<PathBuf, OpError> {
    let located = verbs.located(path, &Access::Write)?;
    if specforge_effects::contained::withheld(&located.rel) {
        return Err(OpError::new(
            OpErrorKind::PermissionDenied,
            OpErrorKind::PermissionDenied.as_str(),
            format!(
                "{} is withheld: the host's own state and credential files are never written through git",
                printable(&path.to_string_lossy())
            ),
        ));
    }
    Ok(located.at)
}

/// The repository's hooks, where git finds them.
pub(super) struct Hooks<'v> {
    verbs: &'v Verbs,
    place: &'v GitPlace,
    dir: PathBuf,
}

impl<'v> Hooks<'v> {
    /// `core.hooksPath` as git interpolates it (`~/`, `~user/`), from the top when relative, else the common
    /// directory's `hooks`.
    pub(super) fn of(verbs: &'v Verbs, place: &'v GitPlace) -> Result<Self, OpError> {
        let configured = verbs
            .ports()
            .git
            .config_path(place, "core.hooksPath")
            .map_err(write_failed)?;
        let dir = match configured {
            Some(path) => place.toplevel.join(path),
            None => place.common_dir.join("hooks"),
        };
        Ok(Hooks { verbs, place, dir })
    }

    /// The hook `name`, when git would run it: a file, executable on Unix (git passes over one that is not).
    fn find(&self, name: &str) -> Option<PathBuf> {
        let hook = self.dir.join(name);
        let meta = std::fs::metadata(&hook).ok()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            (meta.is_file() && meta.permissions().mode() & 0o111 != 0).then_some(hook)
        }
        #[cfg(not(unix))]
        meta.is_file().then_some(hook)
    }

    /// Run the hook `name` with `args` in the working tree's top, when it is there: its output goes to stderr, so
    /// stdout keeps the answer. A hook that fails refuses; one that cannot run refuses too (fail closed) and names
    /// `--no-verify`.
    pub(super) fn run(&self, name: &str, args: &[String]) -> Result<(), OpError> {
        self.run_for(name, args, true).map(|_| ())
    }

    /// [`Hooks::run`], and whether the hook was there to run; a failure names the commit it stops only when
    /// `committing`.
    fn run_for(&self, name: &str, args: &[String], committing: bool) -> Result<bool, OpError> {
        let Some(hook) = self.find(name) else {
            return Ok(false);
        };
        let advice = |error: OpError| match committing {
            true => error.with_suggestion(no_verify(name)),
            false => error,
        };
        let program = hook.to_string_lossy().replace('\\', "/");
        // Windows runs a hook script through the `sh` the runner finds, as Git for Windows does.
        let mut argv = if cfg!(windows) {
            vec!["sh".to_string(), program]
        } else {
            vec![program]
        };
        argv.extend(args.iter().cloned());
        let request = ProcessRequest {
            argv,
            env: vec![EnvVar {
                name: "GIT_INDEX_FILE".to_string(),
                value: Some(
                    self.place
                        .git_dir
                        .join("index")
                        .to_string_lossy()
                        .into_owned(),
                ),
            }],
            stdin: Some(Vec::new()),
            deadline_ms: u64::try_from(USER_CEILING.as_millis()).unwrap_or(u64::MAX),
            output_cap: OUTPUT_CAP,
            ..ProcessRequest::default()
        };
        let shown = printable(&hook.to_string_lossy()).into_owned();
        self.verbs
            .permit(&EffectRequest::Process(request.clone()))
            .map_err(advice)?;
        let outcome = self.verbs.ports().process.run_within(
            &request,
            &self.place.toplevel,
            Output::Tee {
                capture: &mut std::io::sink(),
                shown: Shown::Stderr,
            },
            Ceiling::User,
        );
        let stopped = if committing {
            ": nothing was committed"
        } else {
            ""
        };
        match outcome.class {
            ProcessClass::Exited(0) => Ok(true),
            ProcessClass::NotFound | ProcessClass::CannotStart(_) | ProcessClass::NoDirectory => {
                Err(advice(OpError::new(
                    OpErrorKind::PreconditionFailed,
                    super::HOOK_CANNOT_RUN,
                    format!("the {name} hook {shown} {}", outcome.class),
                )))
            }
            class => Err(advice(OpError::new(
                OpErrorKind::PreconditionFailed,
                super::HOOK_REFUSED,
                format!("the {name} hook {shown} {class}{stopped}"),
            ))),
        }
    }
}

fn no_verify(name: &str) -> String {
    if name == "prepare-commit-msg" {
        return format!(
            "fix what the {name} hook reports: git runs it even when no hook is to run, and so does specforge"
        );
    }
    format!(
        "fix what the {name} hook reports, or run specforge git commit --no-verify to commit without the repository's hooks"
    )
}

#[cfg(test)]
mod tests {
    use super::cleaned;

    #[test]
    fn a_message_is_cleaned_as_git_cleans_one_given_with_m() {
        assert_eq!(
            cleaned("  \n\nsubject  \n\n\n\nbody\t\n\nGsd-Task: 7\n\n").as_deref(),
            Some("subject\n\nbody\n\nGsd-Task: 7\n")
        );
        assert_eq!(cleaned("one line").as_deref(), Some("one line\n"));
        assert_eq!(cleaned(" \n\t\n"), None);
    }
}
