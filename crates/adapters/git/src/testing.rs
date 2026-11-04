//! `specforge_git::testing` (feature `testing`): a repository a test builds with gix, so no test needs the git program: files written into its working tree, an
//! index staged from them, commits made by a fixed author at a clock that ticks once per commit, and refs, config and
//! a linked worktree written as git keeps them.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a fixture's unwrap is its assertion: a repository it cannot build fails the test"
)]
#![allow(
    dead_code,
    reason = "each test binary uses the part of the fixture it needs"
)]

use std::cell::Cell;
use std::path::{Path, PathBuf};

use gix::bstr::ByteSlice;
use gix::index::entry::{Flags, Mode, Stage, Stat};
use gix::objs::tree::EntryKind;

/// A repository with a working tree, on branch `main` until a test moves `HEAD`.
pub struct Repo {
    dir: PathBuf,
    /// This worktree's own git directory: its `HEAD` and index.
    git_dir: PathBuf,
    /// Where the refs and config live: the git directory, or the main one for a linked worktree.
    common_dir: PathBuf,
    clock: Cell<i64>,
}

impl Repo {
    /// A new repository in `dir` (made when missing), `HEAD` on the unborn `main`.
    pub fn init(dir: &Path) -> Repo {
        std::fs::create_dir_all(dir).unwrap();
        gix::ThreadSafeRepository::init_opts(
            dir,
            gix::create::Kind::WithWorktree,
            gix::create::Options::default(),
            gix::open::Options::isolated(),
        )
        .unwrap();
        let repo = Repo {
            dir: dir.to_path_buf(),
            git_dir: dir.join(".git"),
            common_dir: dir.join(".git"),
            clock: Cell::new(1_700_000_000),
        };
        repo.head_on("main");
        repo.config("[user]\n\tname = Test\n\temail = test@example.com");
        repo
    }

    /// [`Self::init`], with the git directory at `git_dir`, outside the working tree (`--separate-git-dir`).
    pub fn init_separate(dir: &Path, git_dir: &Path) -> Repo {
        let repo = Repo::init(dir);
        std::fs::create_dir_all(git_dir.parent().unwrap()).unwrap();
        std::fs::rename(dir.join(".git"), git_dir).unwrap();
        std::fs::write(dir.join(".git"), format!("gitdir: {}\n", git_dir.display())).unwrap();
        Repo {
            git_dir: git_dir.to_path_buf(),
            common_dir: git_dir.to_path_buf(),
            ..repo
        }
    }

    /// The repository whose working tree is `dir`, made elsewhere (by [`Self::init`] or a surface).
    pub fn at(dir: &Path) -> Repo {
        let repo = gix::open_opts(dir, gix::open::Options::isolated()).unwrap();
        Repo {
            dir: dir.to_path_buf(),
            git_dir: repo.git_dir().to_path_buf(),
            common_dir: repo.common_dir().to_path_buf(),
            clock: Cell::new(1_700_100_000),
        }
    }

    /// The working tree's top directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The git directory.
    pub fn git_dir(&self) -> PathBuf {
        self.git_dir.clone()
    }

    /// The repository, opened apart from this machine's and this process's configuration.
    pub fn open(&self) -> gix::Repository {
        gix::open_opts(&self.dir, gix::open::Options::isolated()).unwrap()
    }

    /// Write `bytes` to `rel` in the working tree, with its missing parents.
    pub fn write(&self, rel: &str, bytes: impl AsRef<[u8]>) -> &Self {
        let at = self.dir.join(rel);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(at, bytes).unwrap();
        self
    }

    /// Remove `rel` from the working tree.
    pub fn delete(&self, rel: &str) -> &Self {
        std::fs::remove_file(self.dir.join(rel)).unwrap();
        self
    }

    /// The index holds every file of the working tree, and nothing else (`git add -A`). It knows no ignore rule:
    /// write an ignored file after the last commit.
    pub fn stage_all(&self) {
        let mut paths = Vec::new();
        walk(&self.dir, &self.dir, &mut paths);
        let repo = self.open();
        let mut index = gix::index::File::from_state(
            gix::index::State::new(repo.object_hash()),
            repo.index_path(),
        );
        for rel in paths {
            push(&repo, &mut index, &self.dir, &rel);
        }
        index.sort_entries();
        index.write(gix::index::write::Options::default()).unwrap();
    }

    /// The index takes each of `rels` as the working tree has it now: added, changed, or removed when it is gone
    /// (`git add -- <rels>`).
    pub fn stage(&self, rels: &[&str]) {
        let repo = self.open();
        let mut index = index_of(&repo);
        index.remove_entries(|_, path, _| rels.iter().any(|r| path == r.as_bytes()));
        for rel in rels {
            if self.dir.join(rel).symlink_metadata().is_ok() {
                push(&repo, &mut index, &self.dir, rel);
            }
        }
        index.sort_entries();
        index.write(gix::index::write::Options::default()).unwrap();
    }

    /// `rel` is in conflict in the index: `base`, `ours` and `theirs` at stages 1, 2 and 3, `ours` in the working tree.
    pub fn conflict(&self, rel: &str, base: &str, ours: &str, theirs: &str) {
        let repo = self.open();
        let mut index = index_of(&repo);
        index.remove_entries(|_, path, _| path == rel.as_bytes());
        for (stage, text) in [
            (Stage::Base, base),
            (Stage::Ours, ours),
            (Stage::Theirs, theirs),
        ] {
            let id = repo.write_blob(text).unwrap().detach();
            index.dangerously_push_entry(
                Stat::default(),
                id,
                Flags::from_stage(stage),
                Mode::FILE,
                rel.as_bytes().as_bstr(),
            );
        }
        index.sort_entries();
        index.write(gix::index::write::Options::default()).unwrap();
        self.write(rel, ours);
    }

    /// A commit of the index on `HEAD` (its branch, or `HEAD` itself when detached): its sha.
    pub fn commit(&self, message: &str) -> String {
        self.commit_onto(message, None)
    }

    /// A merge commit of the index on `HEAD`, with `other` as its second parent: its sha.
    pub fn merge(&self, message: &str, other: &str) -> String {
        self.commit_onto(message, Some(other))
    }

    fn commit_onto(&self, message: &str, other: Option<&str>) -> String {
        let repo = self.open();
        let index = index_of(&repo);
        let mut tree = repo.edit_tree(repo.empty_tree().id).unwrap();
        for entry in index.entries() {
            let kind = match entry.mode {
                Mode::SYMLINK => EntryKind::Link,
                Mode::FILE_EXECUTABLE => EntryKind::BlobExecutable,
                _ => EntryKind::Blob,
            };
            tree.upsert(entry.path(&index).to_str().unwrap(), kind, entry.id)
                .unwrap();
        }
        let tree = tree.write().unwrap().detach();
        let parents: Vec<gix::ObjectId> = repo
            .head_id()
            .ok()
            .map(|id| id.detach())
            .into_iter()
            .chain(other.map(|sha| gix::ObjectId::from_hex(sha.as_bytes()).unwrap()))
            .collect();
        self.commit_as(&repo, "HEAD", message, tree, parents)
    }

    /// [`Self::stage_all`], then [`Self::commit`].
    pub fn commit_all(&self, message: &str) -> String {
        self.stage_all();
        self.commit(message)
    }

    /// A commit of `tree` with `parents`, `reference` moved to it.
    fn commit_as(
        &self,
        repo: &gix::Repository,
        reference: &str,
        message: &str,
        tree: gix::ObjectId,
        parents: Vec<gix::ObjectId>,
    ) -> String {
        let now = self.clock.get();
        self.clock.set(now + 60);
        let time = format!("{now} +0000");
        let who = gix::actor::SignatureRef {
            name: "Test".into(),
            email: "test@example.com".into(),
            time: &time,
        };
        repo.commit_as(who, who, reference, message, tree, parents)
            .unwrap()
            .to_string()
    }

    /// The sha `revision` names.
    pub fn sha(&self, revision: &str) -> String {
        self.open().rev_parse_single(revision).unwrap().to_string()
    }

    /// The ref `name` (`refs/heads/x`, `refs/remotes/origin/x`, `refs/tags/x`) names `sha`.
    pub fn set_ref(&self, name: &str, sha: &str) -> &Self {
        let at = self.common_dir.join(name);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(at, format!("{sha}\n")).unwrap();
        self
    }

    /// The branch `name` names `sha`.
    pub fn branch(&self, name: &str, sha: &str) -> &Self {
        self.set_ref(&format!("refs/heads/{name}"), sha)
    }

    /// `HEAD` is on the branch `name` (born or not).
    pub fn head_on(&self, name: &str) -> &Self {
        std::fs::write(
            self.git_dir().join("HEAD"),
            format!("ref: refs/heads/{name}\n"),
        )
        .unwrap();
        self
    }

    /// `HEAD` is detached at `sha`.
    pub fn detach(&self, sha: &str) -> &Self {
        std::fs::write(self.git_dir().join("HEAD"), format!("{sha}\n")).unwrap();
        self
    }

    /// `text` appended to the repository's own config (`[section]` lines and their keys).
    pub fn config(&self, text: &str) -> &Self {
        let at = self.common_dir.join("config");
        let mut config = std::fs::read_to_string(&at).unwrap();
        config.push_str(text);
        config.push('\n');
        std::fs::write(at, config).unwrap();
        self
    }

    /// A remote `name` at `url`.
    pub fn remote(&self, name: &str, url: &str) -> &Self {
        self.config(&format!(
            "[remote \"{name}\"]\n\turl = {url}\n\tfetch = +refs/heads/*:refs/remotes/{name}/*"
        ))
    }

    /// The branch `branch` tracks `origin/<branch>`, which names `sha`.
    pub fn upstream(&self, branch: &str, sha: &str) -> &Self {
        self.set_ref(&format!("refs/remotes/origin/{branch}"), sha);
        self.config(&format!(
            "[branch \"{branch}\"]\n\tremote = origin\n\tmerge = refs/heads/{branch}"
        ))
    }

    /// A linked worktree of this repository at `path`, `HEAD` there on the branch `branch` (which names `sha`), its
    /// files checked out from `sha`'s tree and its index matching them.
    pub fn linked_worktree(&self, path: &Path, branch: &str, sha: &str) -> Repo {
        let id = path.file_name().unwrap().to_str().unwrap();
        let private = self.common_dir.join("worktrees").join(id);
        std::fs::create_dir_all(&private).unwrap();
        std::fs::create_dir_all(path).unwrap();
        self.branch(branch, sha);
        std::fs::write(private.join("HEAD"), format!("ref: refs/heads/{branch}\n")).unwrap();
        std::fs::write(private.join("commondir"), "../..\n").unwrap();
        std::fs::write(
            private.join("gitdir"),
            format!("{}\n", path.join(".git").display()),
        )
        .unwrap();
        std::fs::write(
            path.join(".git"),
            format!("gitdir: {}\n", private.display()),
        )
        .unwrap();
        let linked = Repo {
            dir: path.to_path_buf(),
            git_dir: private.clone(),
            common_dir: self.common_dir.clone(),
            clock: Cell::new(self.clock.get()),
        };
        let repo = linked.open();
        let commit = gix::ObjectId::from_hex(sha.as_bytes()).unwrap();
        let tree = repo.find_commit(commit).unwrap().tree_id().unwrap();
        let index = repo.index_from_tree(&tree).unwrap();
        for entry in index.entries() {
            let rel = entry.path(&index).to_str().unwrap().to_string();
            linked.write(&rel, &repo.find_blob(entry.id).unwrap().data);
        }
        linked.stage_all();
        linked
    }
}

/// Every file under `dir` but the git directory, from `top`, `/` separated.
fn walk(top: &Path, dir: &Path, out: &mut Vec<String>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap())
        .collect();
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        if entry.file_name() == ".git" {
            continue;
        }
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            walk(top, &path, out);
        } else {
            let rel = path
                .strip_prefix(top)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            out.push(rel);
        }
    }
}

/// The working tree's `rel` pushed onto `index` as an unconflicted entry, its blob written.
fn push(repo: &gix::Repository, index: &mut gix::index::File, top: &Path, rel: &str) {
    let at = top.join(rel);
    let meta = at.symlink_metadata().unwrap();
    let (mode, bytes) = if meta.file_type().is_symlink() {
        let target = std::fs::read_link(&at).unwrap();
        (
            Mode::SYMLINK,
            target.to_string_lossy().into_owned().into_bytes(),
        )
    } else {
        (mode_of(&meta), std::fs::read(&at).unwrap())
    };
    let id = repo.write_blob(bytes).unwrap().detach();
    let stat = Stat::from_fs(&gix::index::fs::Metadata::from_path_no_follow(&at).unwrap())
        .unwrap_or_default();
    index.dangerously_push_entry(stat, id, Flags::empty(), mode, rel.as_bytes().as_bstr());
}

#[cfg(unix)]
fn mode_of(meta: &std::fs::Metadata) -> Mode {
    use std::os::unix::fs::PermissionsExt as _;
    if meta.permissions().mode() & 0o111 != 0 {
        Mode::FILE_EXECUTABLE
    } else {
        Mode::FILE
    }
}

#[cfg(not(unix))]
fn mode_of(_: &std::fs::Metadata) -> Mode {
    Mode::FILE
}

/// The repository's index, or an empty one when it has none yet.
fn index_of(repo: &gix::Repository) -> gix::index::File {
    repo.open_index().unwrap_or_else(|_| {
        gix::index::File::from_state(
            gix::index::State::new(repo.object_hash()),
            repo.index_path(),
        )
    })
}
