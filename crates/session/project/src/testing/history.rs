//! A scripted [`History`] for tests: it answers what it was given and counts the files it was asked for.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use crate::history::{
    Blob, CommitFile, GitPlace, History, HistoryError, Located, Sha, SharedHistory, Sized,
    TreeEntry, default_history, set_default,
};

/// Answers `located`, `head` and `files` as given.
pub struct FakeHistory {
    pub located: Located,
    pub head: Result<Option<Sha>, HistoryError>,
    pub files: Result<Vec<CommitFile>, HistoryError>,
    /// Other revisions it knows by name: their commit and the files at it.
    pub named: Vec<(String, Sha, Vec<CommitFile>)>,
    /// The other files it holds at every commit (what `blobs` answers).
    pub blobs: Vec<CommitFile>,
    /// What `blobs` answers instead, when set: git failing to read the files.
    pub blobs_fail: Option<HistoryError>,
    /// The entries its tree holds besides the files and `blobs`, with the size `sizes` answers (`None`: a link or a
    /// submodule).
    pub listed: Vec<(PathBuf, Option<u64>)>,
    /// Regular files its tree lists that git cannot give: `sizes` answers `None` for them.
    pub lost: Vec<PathBuf>,
    /// Every path `blobs` was asked for, in order.
    pub blobs_paths: Mutex<Vec<PathBuf>>,
    /// What `sizes` answers instead, when set: git failing to read the sizes.
    pub sizes_fail: Option<HistoryError>,
    /// What `tree` answers instead, when set: git failing to list the tree.
    pub tree_fail: Option<HistoryError>,
    asked: AtomicUsize,
    trees: AtomicUsize,
    blobs_asks: AtomicUsize,
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
            blobs: Vec::new(),
            blobs_fail: None,
            listed: Vec::new(),
            lost: Vec::new(),
            blobs_paths: Mutex::new(Vec::new()),
            sizes_fail: None,
            tree_fail: None,
            asked: AtomicUsize::new(0),
            trees: AtomicUsize::new(0),
            blobs_asks: AtomicUsize::new(0),
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
            blobs: Vec::new(),
            blobs_fail: None,
            listed: Vec::new(),
            lost: Vec::new(),
            blobs_paths: Mutex::new(Vec::new()),
            sizes_fail: None,
            tree_fail: None,
            asked: AtomicUsize::new(0),
            trees: AtomicUsize::new(0),
            blobs_asks: AtomicUsize::new(0),
            any: AtomicUsize::new(0),
        }
    }

    /// How many times the files at a commit were asked for.
    pub fn files_asked(&self) -> usize {
        self.asked.load(Ordering::SeqCst)
    }

    /// How many times a commit's tree was asked for.
    pub fn trees_asked(&self) -> usize {
        self.trees.load(Ordering::SeqCst)
    }

    /// How many times named files were asked for.
    pub fn blobs_asked(&self) -> usize {
        self.blobs_asks.load(Ordering::SeqCst)
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
    fn tree(&self, _: &GitPlace, commit: &Sha, _: &Path) -> Result<Vec<TreeEntry>, HistoryError> {
        self.any.fetch_add(1, Ordering::SeqCst);
        self.trees.fetch_add(1, Ordering::SeqCst);
        if let Some(error) = &self.tree_fail {
            return Err(error.clone());
        }
        let files = match self.named.iter().find(|(_, sha, _)| sha == commit) {
            Some((_, _, files)) => files.clone(),
            None => self.files.clone()?,
        };
        Ok(files
            .iter()
            .chain(&self.blobs)
            .map(|(path, _)| (path.clone(), true))
            .chain(
                self.listed
                    .iter()
                    .map(|(path, size)| (path.clone(), size.is_some())),
            )
            .chain(self.lost.iter().map(|path| (path.clone(), true)))
            .collect())
    }
    fn sizes(&self, _: &GitPlace, _: &Sha, paths: &[PathBuf]) -> Result<Vec<Sized>, HistoryError> {
        self.any.fetch_add(1, Ordering::SeqCst);
        if let Some(error) = &self.sizes_fail {
            return Err(error.clone());
        }
        let size = |path: &PathBuf| {
            let held = self.blobs.iter().find(|(held, _)| held == path);
            held.map(|(_, bytes)| bytes.len() as u64).or_else(|| {
                let listed = self.listed.iter().find(|(held, _)| held == path);
                listed.and_then(|(_, size)| *size)
            })
        };
        Ok(paths
            .iter()
            .map(|path| (path.clone(), size(path)))
            .collect())
    }
    fn blobs(&self, _: &GitPlace, _: &Sha, paths: &[PathBuf]) -> Result<Vec<Blob>, HistoryError> {
        self.any.fetch_add(1, Ordering::SeqCst);
        self.blobs_asks.fetch_add(1, Ordering::SeqCst);
        self.blobs_paths
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .extend(paths.iter().cloned());
        if let Some(error) = &self.blobs_fail {
            return Err(error.clone());
        }
        Ok(paths
            .iter()
            .map(|path| {
                let held = self.blobs.iter().find(|(held, _)| held == path);
                (path.clone(), held.map(|(_, bytes)| bytes.clone()))
            })
            .collect())
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
