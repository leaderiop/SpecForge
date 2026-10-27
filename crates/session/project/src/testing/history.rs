//! A scripted [`History`] for tests: it answers what it was given and counts the files it was asked for.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use crate::history::{
    CommitFile, GitPlace, History, HistoryError, Located, Sha, SharedHistory, default_history,
    set_default,
};

/// Answers `located`, `head` and `files` as given.
pub struct FakeHistory {
    pub located: Located,
    pub head: Result<Option<Sha>, HistoryError>,
    pub files: Result<Vec<CommitFile>, HistoryError>,
    /// Other revisions it knows by name: their commit and the files at it.
    pub named: Vec<(String, Sha, Vec<CommitFile>)>,
    asked: AtomicUsize,
    any: AtomicUsize,
}

impl FakeHistory {
    /// A repository at `toplevel` whose `HEAD` is `sha` and whose files at it are `files`.
    pub fn repo(toplevel: &Path, sha: &str, files: Vec<CommitFile>) -> Self {
        FakeHistory {
            located: Located::Repo(GitPlace {
                toplevel: toplevel
                    .canonicalize()
                    .unwrap_or_else(|_| toplevel.to_path_buf()),
                git_dir: toplevel.join(".git"),
                common_dir: toplevel.join(".git"),
                shallow: false,
            }),
            head: Ok(Some(Sha::new(sha))),
            files: Ok(files),
            named: Vec::new(),
            asked: AtomicUsize::new(0),
            any: AtomicUsize::new(0),
        }
    }

    /// A directory that is no repository.
    pub fn none() -> Self {
        FakeHistory {
            located: Located::NotARepository,
            head: Ok(None),
            files: Ok(Vec::new()),
            named: Vec::new(),
            asked: AtomicUsize::new(0),
            any: AtomicUsize::new(0),
        }
    }

    /// How many times the files at a commit were asked for.
    pub fn files_asked(&self) -> usize {
        self.asked.load(Ordering::SeqCst)
    }

    /// How many questions of any kind were asked.
    pub fn questions(&self) -> usize {
        self.any.load(Ordering::SeqCst)
    }
}

impl History for FakeHistory {
    fn locate(&self, _: &Path) -> Located {
        self.any.fetch_add(1, Ordering::SeqCst);
        self.located.clone()
    }
    fn commit(&self, _: &GitPlace, revision: &str) -> Result<Option<Sha>, HistoryError> {
        self.any.fetch_add(1, Ordering::SeqCst);
        if revision == "HEAD" {
            return self.head.clone();
        }
        Ok(self
            .named
            .iter()
            .find(|(name, ..)| name == revision)
            .map(|(_, sha, _)| sha.clone()))
    }
    fn merge_base(&self, _: &GitPlace, a: &Sha, _: &Sha) -> Result<Option<Sha>, HistoryError> {
        self.any.fetch_add(1, Ordering::SeqCst);
        Ok(Some(a.clone()))
    }
    fn files(&self, _: &GitPlace, commit: &Sha, _: &Path) -> Result<Vec<CommitFile>, HistoryError> {
        self.any.fetch_add(1, Ordering::SeqCst);
        self.asked.fetch_add(1, Ordering::SeqCst);
        if let Some((_, _, files)) = self.named.iter().find(|(_, sha, _)| sha == commit) {
            return Ok(files.clone());
        }
        self.files.clone()
    }
}

static INSTALL: Mutex<()> = Mutex::new(());

/// The process's default history while it lives (one test at a time holds it), restored on drop.
pub struct Installed {
    previous: Option<SharedHistory>,
    _hold: MutexGuard<'static, ()>,
}

impl Drop for Installed {
    fn drop(&mut self) {
        set_default(self.previous.take());
    }
}

/// Make `history` the process's default for the guard's life.
pub fn install(history: Arc<dyn History>) -> Installed {
    let hold = INSTALL.lock().unwrap_or_else(|e| e.into_inner());
    let previous = default_history();
    set_default(Some(history));
    Installed {
        previous,
        _hold: hold,
    }
}
