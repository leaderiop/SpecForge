//! `specforge git commit` (ADR 0339 D5): stage what the user names, run the repository's own `pre-commit`,
//! `commit-msg` and `post-commit` hooks through the Process port, and record the index as a commit by the identity
//! git's configuration names. The git program is never run.

use std::path::{Path, PathBuf};

use serde::Serialize;
use specforge_common::remedy::printable;
use specforge_effects::git::{GitError, GitPlace, Staging};
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
    /// Staged first, read from the current directory: the files at or under each, as the worktree has them.
    pub add: Vec<PathBuf>,
    /// Out of the index first (`git rm --cached`), the files left on disk.
    pub remove: Vec<PathBuf>,
    /// Every tracked file staged first as the worktree has it (`git commit -a`).
    pub all: bool,
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
        "git_refused",
        printable(&error.0).into_owned(),
    )
}

pub fn commit(verbs: &Verbs, request: &CommitRequest) -> Result<CommitAnswer, OpError> {
    let place = verbs.place_to_write("commit")?;
    refuse_signing(verbs, &place, Signed::Commit)?;
    let mut message = cleaned(&request.message).ok_or_else(empty_message)?;
    let staging = Staging {
        add: in_repository(verbs, &place, &request.add)?,
        remove: in_repository(verbs, &place, &request.remove)?,
        all: request.all,
    };
    let git = &verbs.ports().git;
    if staging != Staging::default() {
        git.stage(&place, &staging).map_err(write_failed)?;
    }
    let hooks = (!request.no_verify)
        .then(|| Hooks::of(verbs, &place))
        .transpose()?;
    if let Some(hooks) = &hooks {
        hooks.run("pre-commit", &[])?;
        message = hooks.edit_message("commit-msg", &message)?;
    }
    let committed = git
        .commit_index(&place, &message, request.allow_empty)
        .map_err(write_failed)?;
    if let Some(hooks) = &hooks {
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
            "signing_unsupported",
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
                    "outside_repository",
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
    let (located, _) = verbs.located(path, Access::Write)?;
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
    /// `core.hooksPath` (from the top when relative, `~/` from the home directory), else the common directory's
    /// `hooks`.
    pub(super) fn of(verbs: &'v Verbs, place: &'v GitPlace) -> Result<Self, OpError> {
        let configured = verbs
            .ports()
            .git
            .config(place, "core.hooksPath")
            .map_err(write_failed)?
            .pop();
        let dir = match configured {
            Some(path) => match (path.strip_prefix("~/"), std::env::home_dir()) {
                (Some(rest), Some(home)) => home.join(rest),
                _ => place.toplevel.join(path),
            },
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
        let Some(hook) = self.find(name) else {
            return Ok(());
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
            .map_err(|refused| refused.with_suggestion(no_verify(name)))?;
        let outcome = self.verbs.ports().process.run_within(
            &request,
            &self.place.toplevel,
            Output::Tee {
                capture: &mut std::io::sink(),
                shown: Shown::Stderr,
            },
            Ceiling::User,
        );
        match outcome.class {
            ProcessClass::Exited(0) => Ok(()),
            ProcessClass::NotFound | ProcessClass::CannotStart(_) | ProcessClass::NoDirectory => {
                Err(OpError::new(
                    OpErrorKind::PreconditionFailed,
                    "hook_cannot_run",
                    format!("the {name} hook {shown} {}", outcome.class),
                )
                .with_suggestion(no_verify(name)))
            }
            class => Err(OpError::new(
                OpErrorKind::PreconditionFailed,
                "hook_refused",
                format!("the {name} hook {shown} {class}: nothing was committed"),
            )
            .with_suggestion(no_verify(name))),
        }
    }

    /// Run the message hook `name` on `message`, written where git writes it (`COMMIT_EDITMSG` in the git
    /// directory): the message as the hook left it, cleaned again.
    fn edit_message(&self, name: &str, message: &str) -> Result<String, OpError> {
        if self.find(name).is_none() {
            return Ok(message.to_string());
        }
        let git = &self.verbs.ports().git;
        let file = git
            .write_commit_message(self.place, message)
            .map_err(write_failed)?;
        self.run(name, &[file.to_string_lossy().into_owned()])?;
        let edited = git.read_commit_message(self.place).map_err(write_failed)?;
        cleaned(&edited).ok_or_else(empty_message)
    }
}

fn no_verify(name: &str) -> String {
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
