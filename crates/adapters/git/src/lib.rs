//! The adapter behind `specforge-project`'s [`History`] port: the git command line (ADR 0287). Read-only, and
//! hardened: no stdin, no prompt, no lazy fetch, no optional lock, no fsmonitor, a fixed locale, the repository the
//! directory is in and not the one the caller's environment names, and a deadline on every call.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use specforge_common::process::{Ran, RunError, run_with};
use specforge_project::history::{CommitFile, GitPlace, History, HistoryError, Located, Sha};

/// Runs `git` from `PATH`.
pub struct GitCli {
    program: OsString,
    timeout: Duration,
}

impl Default for GitCli {
    fn default() -> Self {
        GitCli {
            program: "git".into(),
            timeout: Duration::from_secs(10),
        }
    }
}

/// How one call failed.
enum Failed {
    NotInstalled,
    Other(String),
}

impl GitCli {
    /// The program run in place of `git` (a test's way to fake a missing or broken git).
    pub fn with_program(program: impl Into<OsString>) -> Self {
        GitCli {
            program: program.into(),
            ..Self::default()
        }
    }

    fn call(&self, dir: &Path, args: &[&str], input: Option<Vec<u8>>) -> Result<Ran, Failed> {
        let mut command = Command::new(&self.program);
        command
            .args(["-c", "core.fsmonitor=false", "--literal-pathspecs", "-C"])
            .arg(dir)
            .args(args)
            .env("LC_ALL", "C")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_NO_LAZY_FETCH", "1")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE");
        run_with(&mut command, input, self.timeout).map_err(|error| match error {
            RunError::NotFound => Failed::NotInstalled,
            RunError::CannotStart(e) => Failed::Other(format!("git cannot start: {e}")),
            RunError::TimedOut => Failed::Other(format!(
                "git did not answer within {} seconds",
                self.timeout.as_secs()
            )),
            RunError::Lost => Failed::Other("git ended without an answer".to_string()),
        })
    }

    /// A call that must succeed.
    fn ok(
        &self,
        dir: &Path,
        args: &[&str],
        input: Option<Vec<u8>>,
    ) -> Result<Vec<u8>, HistoryError> {
        match self.call(dir, args, input) {
            Ok(ran) if ran.status.success() => Ok(ran.stdout),
            Ok(ran) => Err(HistoryError(format!(
                "git {} failed: {}",
                args.first().copied().unwrap_or(""),
                String::from_utf8_lossy(&ran.stderr).trim()
            ))),
            Err(Failed::NotInstalled) => Err(HistoryError("git is not installed".into())),
            Err(Failed::Other(why)) => Err(HistoryError(why)),
        }
    }

    /// A call whose exit 1 with no output is "no such thing".
    fn maybe(&self, dir: &Path, args: &[&str]) -> Result<Option<Sha>, HistoryError> {
        match self.call(dir, args, None) {
            Ok(ran) if ran.status.success() => {
                Ok(Some(Sha::new(String::from_utf8_lossy(&ran.stdout).trim())))
            }
            Ok(ran) if ran.status.code() == Some(1) && ran.stdout.is_empty() => Ok(None),
            Ok(ran) => Err(HistoryError(
                String::from_utf8_lossy(&ran.stderr).trim().to_string(),
            )),
            Err(Failed::NotInstalled) => Err(HistoryError("git is not installed".into())),
            Err(Failed::Other(why)) => Err(HistoryError(why)),
        }
    }
}

impl History for GitCli {
    fn locate(&self, dir: &Path) -> Located {
        let ran = match self.call(
            dir,
            &[
                "rev-parse",
                "--git-dir",
                "--git-common-dir",
                "--show-toplevel",
                "--is-shallow-repository",
            ],
            None,
        ) {
            Ok(ran) => ran,
            Err(Failed::NotInstalled) => {
                return Located::Unavailable("git is not installed".into());
            }
            Err(Failed::Other(why)) => return Located::Unavailable(why),
        };
        let stderr = String::from_utf8_lossy(&ran.stderr);
        if !ran.status.success() {
            return if stderr.contains("not a git repository") {
                Located::NotARepository
            } else if stderr.contains("dubious ownership") {
                Located::Unavailable(format!(
                    "git refuses the directory (dubious ownership): {}",
                    stderr.lines().next().unwrap_or("").trim()
                ))
            } else {
                Located::Unavailable(stderr.trim().to_string())
            };
        }
        let text = String::from_utf8_lossy(&ran.stdout);
        let lines: Vec<&str> = text.lines().collect();
        let [git_dir, common_dir, toplevel, shallow] = lines[..] else {
            return Located::Unavailable("git rev-parse answered an unexpected shape".into());
        };
        let absolute = |p: &str| {
            let p = PathBuf::from(p);
            if p.is_absolute() { p } else { dir.join(p) }
        };
        Located::Repo(GitPlace {
            toplevel: Path::new(toplevel)
                .canonicalize()
                .unwrap_or_else(|_| PathBuf::from(toplevel)),
            git_dir: absolute(git_dir),
            common_dir: absolute(common_dir),
            shallow: shallow == "true",
        })
    }

    fn commit(&self, place: &GitPlace, revision: &str) -> Result<Option<Sha>, HistoryError> {
        let spelled = format!("{revision}^{{commit}}");
        self.maybe(
            &place.toplevel,
            &["rev-parse", "--verify", "-q", "--end-of-options", &spelled],
        )
    }

    fn merge_base(&self, place: &GitPlace, a: &Sha, b: &Sha) -> Result<Option<Sha>, HistoryError> {
        self.maybe(
            &place.toplevel,
            &["merge-base", "--end-of-options", a.as_str(), b.as_str()],
        )
    }

    fn files(
        &self,
        place: &GitPlace,
        commit: &Sha,
        dir: &Path,
    ) -> Result<Vec<CommitFile>, HistoryError> {
        let mut args = vec!["ls-tree", "-r", "-z", "--full-tree", commit.as_str()];
        let pathspec = if dir.as_os_str().is_empty() {
            None
        } else {
            Some(format!("{}/", dir.to_string_lossy()))
        };
        if let Some(pathspec) = &pathspec {
            args.extend(["--", pathspec]);
        }
        let listing = self.ok(&place.toplevel, &args, None)?;
        // `<mode> SP <type> SP <object> TAB <path> NUL`; a symlink or a submodule is no file.
        let mut blobs: Vec<(PathBuf, String)> = Vec::new();
        for entry in listing.split(|b| *b == 0).filter(|e| !e.is_empty()) {
            let text = String::from_utf8_lossy(entry);
            let Some((meta, path)) = text.split_once('\t') else {
                continue;
            };
            let mut parts = meta.split(' ');
            let (mode, kind, object) = (parts.next(), parts.next(), parts.next());
            // only what a baseline compiles: the specs and the ledger, so a repository's images, lockfiles and vendored
            // trees are never read
            let wanted = path.ends_with(".spec")
                || path.rsplit('/').next() == Some(specforge_project::harness::LEDGER_FILE);
            if kind == Some("blob")
                && mode != Some("120000")
                && wanted
                && let Some(object) = object
            {
                blobs.push((PathBuf::from(path), object.to_string()));
            }
        }
        if blobs.is_empty() {
            return Ok(Vec::new());
        }
        let request: String = blobs.iter().map(|(_, o)| format!("{o}\n")).collect();
        let answer = self.ok(
            &place.toplevel,
            &["cat-file", "--batch"],
            Some(request.into_bytes()),
        )?;
        let mut at = 0;
        let mut files = Vec::with_capacity(blobs.len());
        for (path, object) in blobs {
            let end = answer[at..]
                .iter()
                .position(|b| *b == b'\n')
                .map(|i| at + i)
                .ok_or_else(|| HistoryError("git cat-file answered a short reply".into()))?;
            let header = String::from_utf8_lossy(&answer[at..end]).into_owned();
            let size = match header.split(' ').collect::<Vec<_>>()[..] {
                [sha, "blob", size] if sha == object => size.parse::<usize>().ok(),
                _ => None,
            }
            .ok_or_else(|| HistoryError(format!("git cat-file cannot give {object}: {header}")))?;
            let body = answer
                .get(end + 1..end + 1 + size)
                .ok_or_else(|| HistoryError("git cat-file answered a short reply".into()))?;
            files.push((path, body.to_vec()));
            at = end + 1 + size + 1;
        }
        Ok(files)
    }
}
