//! The files a compute pass reads at the baseline (ADR 0300): what exists at the commit comes from its tree (no
//! content), and the bytes of the files the passes' `reads` name are fetched from the commit in one question. A path is
//! absent only when the tree lacks it; a link, a submodule, a directory and a file git gave no bytes for are
//! unreadable, never absent. The tree carries no sizes: the size of a file is asked only for the files a pass reads,
//! and the 64 MiB rule is applied before any bytes are fetched.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, ErrorKind};
use std::path::Path;

use specforge_common::regular::MAX_FILE_BYTES;
use specforge_graph::Computed;
use specforge_registry::{Disk, EntitySnapshot};
use specforge_wasm::WasmRuntime;

use crate::Environment;
use crate::baseline_judged::Fetch;
use crate::compute::Reads;
use crate::judged::Evaluated;

/// A commit's tree by path from the project's root: `true` for a regular file, `false` for a link or a submodule.
pub(crate) type Tree = BTreeMap<String, bool>;

/// The sizes of the files at the commit, in the order asked: `None` for one git cannot give.
pub(crate) type Sizes<'a> = &'a dyn Fn(&[String]) -> Vec<(String, Option<u64>)>;

/// What the commit is asked for besides its tree: the sizes of files and their bytes.
#[derive(Clone, Copy)]
pub(crate) struct Git<'a> {
    pub sizes: Sizes<'a>,
    pub fetch: Fetch<'a>,
}

/// The commit's files as a [`Disk`] under `root`; a file not fetched yet is noted in `wanted` and reads as an error.
struct TreeDisk<'a> {
    root: &'a Path,
    tree: &'a Tree,
    fetched: RefCell<BTreeMap<String, Option<Vec<u8>>>>,
    /// The files over the cap: their size was asked and their bytes never are.
    too_large: RefCell<BTreeSet<String>>,
    wanted: RefCell<BTreeSet<String>>,
}

impl TreeDisk<'_> {
    fn key(&self, path: &Path) -> Option<String> {
        let rest = path.strip_prefix(self.root).ok()?;
        Some(
            rest.components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/"),
        )
    }

    /// Whether some entry lies under the directory `key`.
    fn is_dir_key(&self, key: &str) -> bool {
        let dir = format!("{key}/");
        self.tree
            .range(dir.clone()..)
            .next()
            .is_some_and(|(k, _)| k.starts_with(&dir))
    }

    /// Whether a proper prefix of `key` is a link or a submodule.
    fn under_opaque(&self, key: &str) -> bool {
        key.match_indices('/')
            .any(|(at, _)| self.tree.get(&key[..at]) == Some(&false))
    }
}

impl Disk for TreeDisk<'_> {
    fn exists(&self, path: &Path) -> bool {
        let Some(key) = self.key(path) else {
            return false;
        };
        key.is_empty()
            || self.tree.contains_key(&key)
            || self.is_dir_key(&key)
            || self.under_opaque(&key)
    }

    fn list(&self, dir: &Path) -> Option<Vec<String>> {
        let key = self.key(dir)?;
        let prefix = if key.is_empty() {
            String::new()
        } else {
            format!("{key}/")
        };
        let names: BTreeSet<&str> = self
            .tree
            .range(prefix.clone()..)
            .map_while(|(k, _)| k.strip_prefix(&prefix))
            .filter_map(|rest| rest.split('/').next())
            .collect();
        (!names.is_empty()).then(|| names.into_iter().map(str::to_string).collect())
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        let key = self
            .key(path)
            .ok_or_else(|| io::Error::from(ErrorKind::NotFound))?;
        match self.tree.get(&key) {
            Some(true) if self.too_large.borrow().contains(&key) => {
                Err(io::Error::from(ErrorKind::FileTooLarge))
            }
            Some(true) => match self.fetched.borrow().get(&key) {
                Some(Some(bytes)) => Ok(bytes.clone()),
                Some(None) => Err(io::Error::other("git gave no bytes for the file")),
                None => {
                    self.wanted.borrow_mut().insert(key);
                    Err(io::Error::other("the file is not fetched yet"))
                }
            },
            Some(false) => Err(io::Error::other("a link or a submodule is not read")),
            None if self.under_opaque(&key) => Err(io::Error::other("under a link or a submodule")),
            None if self.is_dir_key(&key) => Err(io::Error::other("a directory")),
            None => Err(io::Error::from(ErrorKind::NotFound)),
        }
    }
}

/// A name git can be asked for: not one with a newline, nor one that is not UTF-8 (its key is lossy). Those are
/// unreadable for good, so they never make a baseline incomplete.
fn askable(key: &str) -> bool {
    !key.contains(['\n', '\u{FFFD}'])
}

/// What the compute passes of `env` compute over `snapshot` (and the `judged` values) at a commit holding `tree`.
/// `held` are the files already in hand (specs and ledger); `sizes` gives the size of each file a pass reads and
/// `fetch` the bytes of those within the cap, once each.
pub(crate) fn compute_at(
    env: &Environment,
    snapshot: &EntitySnapshot,
    runtime: &dyn WasmRuntime,
    judged: &Evaluated,
    tree: &Tree,
    held: BTreeMap<String, Vec<u8>>,
    git: Git<'_>,
) -> (Computed, bool) {
    let Git { sizes, fetch } = git;
    // a file git could not give though it can be asked for: the baseline may be whole later (a blobless clone)
    let mut incomplete = false;
    let disk = TreeDisk {
        root: env.root(),
        tree,
        fetched: RefCell::new(held.into_iter().map(|(k, v)| (k, Some(v))).collect()),
        too_large: RefCell::new(BTreeSet::new()),
        wanted: RefCell::new(BTreeSet::new()),
    };
    let seen = if judged.values.is_empty() {
        snapshot.clone()
    } else {
        snapshot.with_computed(&judged.values, env.registries().fields())
    };
    // Learn the files the reads name without calling a guest, so each pass runs once.
    for pass in env.registries().compute_passes() {
        if !pass.reads().is_empty() {
            crate::file_reads::read(pass.reads(), &seen, &env.file_source(&disk));
        }
    }
    loop {
        let wanted: Vec<String> = std::mem::take(&mut *disk.wanted.borrow_mut())
            .into_iter()
            .collect();
        // Every wanted key leaves with an answer, whatever the callbacks return, so the loop ends.
        let known: BTreeMap<String, Option<u64>> = if wanted.is_empty() {
            BTreeMap::new()
        } else {
            sizes(&wanted).into_iter().collect()
        };
        let mut ask = Vec::new();
        for key in wanted {
            match known.get(&key).copied().flatten() {
                None => {
                    incomplete |= askable(&key);
                    disk.fetched.borrow_mut().insert(key, None);
                }
                Some(size) if size > MAX_FILE_BYTES => {
                    disk.too_large.borrow_mut().insert(key);
                }
                Some(_) => ask.push(key),
            }
        }
        let got: BTreeMap<String, Option<Vec<u8>>> = if ask.is_empty() {
            BTreeMap::new()
        } else {
            fetch(&ask).into_iter().collect()
        };
        for key in ask {
            let bytes = got.get(&key).cloned().flatten();
            incomplete |= bytes.is_none();
            disk.fetched.borrow_mut().insert(key, bytes);
        }
        let (computed, _) = crate::compute::run(
            env,
            snapshot,
            runtime,
            None,
            &judged.values,
            &judged.judgements,
            Reads {
                disk: &disk,
                tests: None,
            },
        );
        // a pass read a path only a value computed before it names: fetch it and run again
        if disk.wanted.borrow().is_empty() {
            return (computed, incomplete);
        }
    }
}
