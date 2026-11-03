//! The adapter behind the effect plane's [`Git`] port: the git command line (ADR 0287, ADR 0335). Read-only, and
//! hardened: no stdin, no prompt, no optional lock, no fsmonitor, a fixed locale, the repository the directory is in
//! and not the one the caller's environment names, and a deadline on every call.
//!
//! Lazy fetch (ADR 0300 D5): a tree listing never looks a blob up, so it fetches nothing on any git. Sizes and bytes
//! are asked only for the files a pass reads; from git 2.44 `GIT_NO_LAZY_FETCH=1` makes git answer "missing" for a
//! blob a partial clone lacks. An older git does not know the variable and fetches those files, and only those.

use std::path::{Path, PathBuf};
use std::time::Duration;

use specforge_effects::git::{
    Blob, CommitFile, Git, GitError, GitPlace, Located, Sha, Sized, TreeEntry, WorktreeStatus,
};
use specforge_effects::process::request;
use specforge_effects::{EnvVar, Output, ProcessClass, SharedProcess};

/// Runs `git` from `PATH` through the effect plane's Process port (ADR 0335).
pub struct GitCli {
    process: SharedProcess,
    program: PathBuf,
    timeout: Duration,
}

impl GitCli {
    /// `git` from `PATH`, run by `process`, every call given up after 10 s.
    pub fn new(process: SharedProcess) -> Self {
        GitCli {
            process,
            program: PathBuf::from("git"),
            timeout: Duration::from_secs(10),
        }
    }

    /// Every call given up after `timeout`, for a caller with a tighter budget than the default 10 s (a hook's).
    pub fn with_timeout(process: SharedProcess, timeout: Duration) -> Self {
        GitCli {
            timeout,
            ..GitCli::new(process)
        }
    }
}

/// A call that ran to its exit.
struct Ran {
    code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// How one call failed.
enum Failed {
    NotInstalled,
    /// The directory it was asked about does not exist: no repository is there.
    NoDirectory,
    Other(String),
}

impl std::fmt::Display for Failed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Failed::NotInstalled => f.write_str("git is not installed"),
            Failed::NoDirectory => f.write_str("git's directory does not exist"),
            Failed::Other(why) => f.write_str(why),
        }
    }
}

impl GitCli {
    /// The program run in place of `git` (a test's way to fake a missing or broken git).
    pub fn with_program(process: SharedProcess, program: impl AsRef<Path>) -> Self {
        GitCli {
            program: program.as_ref().to_path_buf(),
            ..Self::new(process)
        }
    }

    /// One call in `dir` (git's `-C`: the program runs there): its exit code and output, or how it failed.
    fn call(&self, dir: &Path, args: &[&str], input: Option<Vec<u8>>) -> Result<Ran, Failed> {
        // A request carries words: a program path that is not one is refused, never mangled into another path.
        let Some(program) = self.program.to_str() else {
            return Err(Failed::Other(format!(
                "git's program path is not UTF-8: {}",
                self.program.display()
            )));
        };
        let argv = [program, "-c", "core.fsmonitor=false", "--literal-pathspecs"]
            .into_iter()
            .chain(args.iter().copied());
        let mut asked = request(argv, self.timeout);
        asked.stdin = input;
        asked.env = vec![
            EnvVar::set("LC_ALL", "C"),
            EnvVar::set("GIT_TERMINAL_PROMPT", "0"),
            EnvVar::set("GIT_NO_LAZY_FETCH", "1"),
            EnvVar::set("GIT_OPTIONAL_LOCKS", "0"),
            EnvVar::removed("GIT_DIR"),
            EnvVar::removed("GIT_WORK_TREE"),
            EnvVar::removed("GIT_INDEX_FILE"),
        ];
        let ran = self.process.run(&asked, dir, Output::Buffer);
        match ran.class {
            ProcessClass::Exited(code) => Ok(Ran {
                code,
                stdout: ran.stdout,
                stderr: ran.stderr,
            }),
            ProcessClass::NotFound => Err(Failed::NotInstalled),
            ProcessClass::NoDirectory => Err(Failed::NoDirectory),
            ProcessClass::DeadlineExceeded => Err(Failed::Other(format!(
                "git did not answer within {} seconds",
                self.timeout.as_secs()
            ))),
            other => Err(Failed::Other(format!("git {other}"))),
        }
    }

    /// A call that must succeed.
    fn ok(&self, dir: &Path, args: &[&str], input: Option<Vec<u8>>) -> Result<Vec<u8>, GitError> {
        match self.call(dir, args, input) {
            Ok(ran) if ran.code == 0 => Ok(ran.stdout),
            Ok(ran) => Err(GitError(format!(
                "git {} failed: {}",
                args.first().copied().unwrap_or(""),
                String::from_utf8_lossy(&ran.stderr).trim()
            ))),
            Err(failed) => Err(GitError(failed.to_string())),
        }
    }

    /// A call whose exit 1 with no output is "no such thing".
    fn maybe(&self, dir: &Path, args: &[&str]) -> Result<Option<Sha>, GitError> {
        match self.call(dir, args, None) {
            Ok(ran) if ran.code == 0 => {
                Ok(Some(Sha::new(String::from_utf8_lossy(&ran.stdout).trim())))
            }
            Ok(ran) if ran.code == 1 && ran.stdout.is_empty() => Ok(None),
            Ok(ran) => Err(GitError(
                String::from_utf8_lossy(&ran.stderr).trim().to_string(),
            )),
            Err(failed) => Err(GitError(failed.to_string())),
        }
    }

    /// `git ls-tree -r -z --full-tree` of `commit` under `dir` (from the top). Never `-l`: that looks up every blob's
    /// size, and in a partial clone git older than 2.44 fetches each missing blob to answer.
    fn ls_tree(&self, place: &GitPlace, commit: &Sha, dir: &Path) -> Result<Vec<u8>, GitError> {
        let mut args = vec!["ls-tree", "-r", "-z", "--full-tree", commit.as_str()];
        let pathspec = if dir.as_os_str().is_empty() {
            None
        } else {
            // a pathspec that is not UTF-8 would be mangled into another one and list nothing: refuse it instead
            let dir = dir.to_str().ok_or_else(|| {
                GitError("the project's directory is not valid UTF-8, so git cannot list it".into())
            })?;
            Some(format!("{dir}/"))
        };
        if let Some(pathspec) = &pathspec {
            args.extend(["--", pathspec]);
        }
        self.ok(&place.toplevel, &args, None)
    }
}

impl Git for GitCli {
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
            Err(Failed::NoDirectory) => return Located::NotARepository,
            Err(failed) => return Located::Unavailable(failed.to_string()),
        };
        let stderr = String::from_utf8_lossy(&ran.stderr);
        if ran.code != 0 {
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

    fn commit(&self, place: &GitPlace, revision: &str) -> Result<Option<Sha>, GitError> {
        let spelled = format!("{revision}^{{commit}}");
        self.maybe(
            &place.toplevel,
            &["rev-parse", "--verify", "-q", "--end-of-options", &spelled],
        )
    }

    fn merge_base(&self, place: &GitPlace, a: &Sha, b: &Sha) -> Result<Option<Sha>, GitError> {
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
        wanted: &dyn Fn(&Path) -> bool,
    ) -> Result<Vec<CommitFile>, GitError> {
        let listing = self.ls_tree(place, commit, dir)?;
        // `<mode> SP <type> SP <object> TAB <path> NUL`; a symlink or a submodule is no file.
        let mut blobs: Vec<(PathBuf, String)> = Vec::new();
        for entry in listing.split(|b| *b == 0).filter(|e| !e.is_empty()) {
            let text = String::from_utf8_lossy(entry);
            let Some((meta, path)) = text.split_once('\t') else {
                continue;
            };
            let mut parts = meta.split(' ');
            let (mode, kind, object) = (parts.next(), parts.next(), parts.next());
            // only what the caller wants, so a repository's images, lockfiles and vendored trees are never read
            if kind == Some("blob")
                && mode != Some("120000")
                && wanted(Path::new(path))
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
                .ok_or_else(|| GitError("git cat-file answered a short reply".into()))?;
            let header = String::from_utf8_lossy(&answer[at..end]).into_owned();
            let size = match header.split(' ').collect::<Vec<_>>()[..] {
                [sha, "blob", size] if sha == object => size.parse::<usize>().ok(),
                _ => None,
            }
            .ok_or_else(|| GitError(format!("git cat-file cannot give {object}: {header}")))?;
            let body = answer
                .get(end + 1..end + 1 + size)
                .ok_or_else(|| GitError("git cat-file answered a short reply".into()))?;
            files.push((path, body.to_vec()));
            at = end + 1 + size + 1;
        }
        Ok(files)
    }

    fn tree(&self, place: &GitPlace, commit: &Sha, dir: &Path) -> Result<Vec<TreeEntry>, GitError> {
        let listing = self.ls_tree(place, commit, dir)?;
        // `<mode> SP <type> SP <object> TAB <path> NUL`
        let mut entries = Vec::new();
        for entry in listing.split(|b| *b == 0).filter(|e| !e.is_empty()) {
            let text = String::from_utf8_lossy(entry);
            let Some((meta, path)) = text.split_once('\t') else {
                continue;
            };
            let parts: Vec<&str> = meta.split_whitespace().collect();
            let [mode, kind, _] = parts[..] else {
                continue;
            };
            entries.push((PathBuf::from(path), kind == "blob" && mode != "120000"));
        }
        Ok(entries)
    }

    fn sizes(
        &self,
        place: &GitPlace,
        commit: &Sha,
        paths: &[PathBuf],
    ) -> Result<Vec<Sized>, GitError> {
        // `--batch-check` prints `<object> blob <size>` or `<name> missing`; one line per name asked
        let askable = |path: &&PathBuf| !path.to_string_lossy().contains('\n');
        let request: String = paths
            .iter()
            .filter(askable)
            .map(|p| format!("{}:{}\n", commit.as_str(), p.to_string_lossy()))
            .collect();
        if request.is_empty() {
            return Ok(paths.iter().map(|p| (p.clone(), None)).collect());
        }
        let answer = self.ok(
            &place.toplevel,
            &["cat-file", "--batch-check"],
            Some(request.into_bytes()),
        )?;
        let text = String::from_utf8_lossy(&answer);
        let mut lines = text.lines();
        let short = || GitError("git cat-file answered a short reply".into());
        let mut found = Vec::with_capacity(paths.len());
        for path in paths {
            if !askable(&path) {
                found.push((path.clone(), None));
                continue;
            }
            let line = lines.next().ok_or_else(short)?;
            let size = match line.rsplit(' ').collect::<Vec<_>>()[..] {
                [size, "blob", _] => size.parse::<u64>().ok(),
                _ => None,
            };
            found.push((path.clone(), size));
        }
        Ok(found)
    }

    fn blobs(
        &self,
        place: &GitPlace,
        commit: &Sha,
        paths: &[PathBuf],
    ) -> Result<Vec<Blob>, GitError> {
        // a path with a newline cannot be asked of `--batch`; it reads as missing
        let askable = |path: &&PathBuf| !path.to_string_lossy().contains('\n');
        let request: String = paths
            .iter()
            .filter(askable)
            .map(|p| format!("{}:{}\n", commit.as_str(), p.to_string_lossy()))
            .collect();
        if request.is_empty() {
            return Ok(paths.iter().map(|p| (p.clone(), None)).collect());
        }
        let answer = self.ok(
            &place.toplevel,
            &["cat-file", "--batch"],
            Some(request.into_bytes()),
        )?;
        let short = || GitError("git cat-file answered a short reply".into());
        let mut at = 0;
        let mut found = Vec::with_capacity(paths.len());
        for path in paths {
            if !askable(&path) {
                found.push((path.clone(), None));
                continue;
            }
            let end = answer[at..]
                .iter()
                .position(|b| *b == b'\n')
                .map(|i| at + i)
                .ok_or_else(short)?;
            let header = String::from_utf8_lossy(&answer[at..end]).into_owned();
            at = end + 1;
            match header.split(' ').collect::<Vec<_>>()[..] {
                [_, "blob", size] => {
                    let size = size.parse::<usize>().map_err(|_| short())?;
                    let body = answer.get(at..at + size).ok_or_else(short)?;
                    found.push((path.clone(), Some(body.to_vec())));
                    at += size + 1;
                }
                // missing, or a tree or commit: no file
                _ => found.push((path.clone(), None)),
            }
        }
        Ok(found)
    }
    // shortcut: these calls use the adapter's 10 s deadline, past a hook's budget of `timeout_ms + 5 s`; a host that
    // kills the hook first fails open, as upstream does when its probe times out. Upgrade for a hard bound (ADR 0320).
    fn branch(&self, place: &GitPlace) -> Result<String, GitError> {
        let out = self.ok(&place.toplevel, &["branch", "--show-current"], None)?;
        Ok(String::from_utf8_lossy(&out).trim().to_string())
    }

    fn status(&self, place: &GitPlace) -> Result<WorktreeStatus, GitError> {
        let out = self.ok(
            &place.toplevel,
            &["status", "--porcelain=v2", "--branch"],
            None,
        )?;
        let porcelain = String::from_utf8_lossy(&out).into_owned();
        Ok(WorktreeStatus {
            dirty: dirty(&porcelain),
            porcelain,
        })
    }
}

/// Whether a `git status --porcelain=v2` lists a tracked change (`1`, `2`), a conflict (`u`) or an untracked file
/// (`?`).
fn dirty(porcelain: &str) -> bool {
    porcelain
        .lines()
        .any(|line| matches!(line.split(' ').next(), Some("1" | "2" | "u" | "?")))
}

#[cfg(test)]
mod status_tests {
    use super::dirty;

    #[test]
    fn a_tree_is_dirty_by_a_tracked_change_a_conflict_or_an_untracked_file() {
        assert!(!dirty("# branch.oid abc\n# branch.head main\n"));
        for line in [
            "1 .M N... 100644 100644 100644 a b f",
            "2 R. x",
            "u UU x",
            "? new.txt",
        ] {
            assert!(dirty(&format!("# branch.head main\n{line}\n")), "{line}");
        }
        assert!(!dirty("! ignored.txt\n"));
    }
}
