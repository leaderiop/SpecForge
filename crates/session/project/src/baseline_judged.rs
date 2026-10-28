//! The baseline's judgements (ADR 0290, U9): the judged evaluation run at the baseline commit over that commit's files.
//! The files a rule's `file(..)` inputs and `cited` read are not among those a baseline compiles (specs and the
//! ledger), so the evaluation runs once, notes the paths its disk was asked for and did not hold, asks the history for
//! them in one question, and runs again. There is no test report at a commit: test inputs are not compared, and the
//! I205 the evaluation says is dropped with the rest of its diagnostics.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};

use specforge_graph::Graph;
use specforge_registry::{Disk, EntitySnapshot};

use crate::Environment;
use crate::judged::{DigestMemo, Evaluated, evaluate};

/// Asks for files by their path from the project's root; answers in the order asked, `None` for one that is not there.
pub(crate) type Fetch<'a> = &'a dyn Fn(&[String]) -> Vec<(String, Option<Vec<u8>>)>;

/// The files of one commit, by path from the project's root, as a [`Disk`] under `root`.
struct CommitDisk<'a> {
    root: &'a Path,
    files: &'a BTreeMap<String, Vec<u8>>,
    /// The paths asked for that the commit's files do not hold.
    absent: RefCell<BTreeSet<String>>,
}

impl CommitDisk<'_> {
    fn key(&self, path: &Path) -> Option<String> {
        let rest = path.strip_prefix(self.root).ok()?;
        Some(
            rest.components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/"),
        )
    }
}

impl Disk for CommitDisk<'_> {
    fn exists(&self, path: &Path) -> bool {
        let Some(key) = self.key(path) else {
            return false;
        };
        let dir = format!("{key}/");
        let held = self.files.contains_key(&key) || self.files.keys().any(|k| k.starts_with(&dir));
        if !held {
            self.absent.borrow_mut().insert(key);
        }
        held
    }

    fn list(&self, dir: &Path) -> Option<Vec<String>> {
        let key = self.key(dir)?;
        let prefix = if key.is_empty() {
            String::new()
        } else {
            format!("{key}/")
        };
        let names: BTreeSet<String> = self
            .files
            .keys()
            .filter_map(|k| k.strip_prefix(&prefix))
            .filter_map(|rest| rest.split('/').next().map(str::to_string))
            .collect();
        (!names.is_empty()).then(|| names.into_iter().collect())
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        let key = self
            .key(path)
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        match self.files.get(&key) {
            Some(bytes) => Ok(bytes.clone()),
            None => {
                self.absent.borrow_mut().insert(key);
                Err(io::Error::from(io::ErrorKind::NotFound))
            }
        }
    }
}

/// The judged evaluation of `graph` at a commit: `files` are what the commit holds that a baseline compiles, `ledger`
/// its `specforge-harness`, `fetch` the rest on demand.
pub(crate) fn judge(
    env: &Environment,
    graph: &Graph,
    entities: &EntitySnapshot,
    mut files: BTreeMap<String, Vec<u8>>,
    ledger: &[u8],
    fetch: Fetch<'_>,
) -> Evaluated {
    let mut asked: BTreeSet<String> = BTreeSet::new();
    loop {
        let disk = CommitDisk {
            root: env.root(),
            files: &files,
            absent: RefCell::new(BTreeSet::new()),
        };
        let evaluated = evaluate(env, graph, entities, &disk, &DigestMemo::default(), ledger);
        let wanted: Vec<String> = disk
            .absent
            .into_inner()
            .into_iter()
            .filter(|key| asked.insert(key.clone()))
            .collect();
        if wanted.is_empty() {
            return evaluated;
        }
        for (key, bytes) in fetch(&wanted) {
            files.extend(bytes.map(|bytes| (key, bytes)));
        }
    }
}

/// `fetch` over a history: the paths are the project's own, `within` the project's directory in the repository.
pub(crate) fn paths(within: &Path, keys: &[String]) -> Vec<PathBuf> {
    keys.iter().map(|key| within.join(key)).collect()
}
