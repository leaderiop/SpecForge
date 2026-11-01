//! The commit a build is made from, read from the `.git` files without running git: SpecForge needs nothing
//! installed, not even to build it. Shared by `build.rs` and the CLI's unit tests.

use std::fs;
use std::path::{Path, PathBuf};

/// The commit the checkout holding `start` is at, and the files whose change moves it.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Commit {
    /// The first 9 hex digits of the commit; none without a `.git` or with a HEAD that names no commit.
    pub short: Option<String>,
    /// HEAD, the loose ref it names (else the directory that would hold it) and `packed-refs`: what the build
    /// script reruns on.
    pub watched: Vec<PathBuf>,
}

/// The commit of the first `.git` from `start` upward. `.git` is a directory, or a file `gitdir: <dir>` (a
/// worktree, whose `commondir` file names the directory holding the shared refs). HEAD is a detached hash or
/// `ref: <name>`, a loose ref under the git directory or the common one, else a line of `packed-refs`.
pub fn of(start: &Path) -> Commit {
    let mut commit = Commit::default();
    commit.short = read(start, &mut commit.watched);
    commit
}

fn read(start: &Path, watched: &mut Vec<PathBuf>) -> Option<String> {
    let dot_git = start
        .ancestors()
        .map(|dir| dir.join(".git"))
        .find(|path| path.exists())?;
    let git_dir = if dot_git.is_file() {
        let text = fs::read_to_string(&dot_git).ok()?;
        // A relative gitdir is relative to the directory holding the `.git` file; `join` keeps an absolute one.
        dot_git
            .parent()?
            .join(text.trim().strip_prefix("gitdir:")?.trim())
    } else {
        dot_git
    };
    let common = match fs::read_to_string(git_dir.join("commondir")) {
        Ok(relative) => git_dir.join(relative.trim()),
        Err(_) => git_dir.clone(),
    };
    let head_path = git_dir.join("HEAD");
    let head = fs::read_to_string(&head_path).ok()?;
    watched.push(head_path);
    let Some(name) = head.trim().strip_prefix("ref:") else {
        return short(head.trim());
    };
    let name = name.trim();
    // A per-worktree ref is under the git directory, a branch under the common one.
    for dir in [&git_dir, &common] {
        let loose = dir.join(name);
        if let Ok(hash) = fs::read_to_string(&loose) {
            watched.push(loose);
            return short(hash.trim());
        }
    }
    // Packed: a commit on the branch writes the loose ref, so its directory is watched too.
    if let Some(dir) = common.join(name).parent().filter(|dir| dir.is_dir()) {
        watched.push(dir.to_path_buf());
    }
    let packed_path = common.join("packed-refs");
    let packed = fs::read_to_string(&packed_path).ok()?;
    watched.push(packed_path);
    packed
        .lines()
        .filter_map(|line| line.split_once(' '))
        .find(|(_, reference)| *reference == name)
        .and_then(|(hash, _)| short(hash))
}

/// The first 9 hex digits of a full commit hash (SHA-1 or SHA-256); none for anything else.
fn short(hash: &str) -> Option<String> {
    (hash.len() >= 40 && hash.bytes().all(|b| b.is_ascii_hexdigit())).then(|| hash[..9].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "9229143f5aabbccddeeff00112233445566778899";

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    #[test]
    fn a_branch_is_read_from_its_loose_ref() {
        let dir = tempfile::tempdir().unwrap();
        let git = dir.path().join(".git");
        write(&git.join("HEAD"), "ref: refs/heads/main\n");
        write(&git.join("refs/heads/main"), &format!("{HASH}\n"));
        let below = dir.path().join("crates/cli");
        fs::create_dir_all(&below).unwrap();

        let commit = of(&below);

        assert_eq!(commit.short.as_deref(), Some("9229143f5"));
        assert_eq!(
            commit.watched,
            [git.join("HEAD"), git.join("refs/heads/main")]
        );
    }

    #[test]
    fn a_packed_branch_is_read_from_packed_refs() {
        let dir = tempfile::tempdir().unwrap();
        let git = dir.path().join(".git");
        write(&git.join("HEAD"), "ref: refs/heads/main\n");
        fs::create_dir_all(git.join("refs/heads")).unwrap();
        write(
            &git.join("packed-refs"),
            &format!(
                "# pack-refs with: peeled fully-peeled sorted\n{}0 refs/heads/other\n{HASH} refs/heads/main\n^{HASH}\n",
                &HASH[..39]
            ),
        );

        let commit = of(dir.path());

        assert_eq!(commit.short.as_deref(), Some("9229143f5"));
        assert_eq!(
            commit.watched,
            [
                git.join("HEAD"),
                git.join("refs/heads"),
                git.join("packed-refs")
            ]
        );
    }

    #[test]
    fn a_detached_head_is_the_commit() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join(".git/HEAD"), &format!("{HASH}\n"));

        assert_eq!(of(dir.path()).short.as_deref(), Some("9229143f5"));
    }

    #[test]
    fn a_worktree_reads_its_head_and_the_common_refs() {
        let dir = tempfile::tempdir().unwrap();
        let common = dir.path().join("main/.git");
        let git_dir = common.join("worktrees/wt");
        write(&git_dir.join("HEAD"), "ref: refs/heads/fix\n");
        write(&git_dir.join("commondir"), "../..\n");
        write(&common.join("refs/heads/fix"), &format!("{HASH}\n"));
        write(
            &dir.path().join("wt/.git"),
            &format!("gitdir: {}\n", git_dir.display()),
        );

        let commit = of(&dir.path().join("wt"));

        assert_eq!(commit.short.as_deref(), Some("9229143f5"));
        assert_eq!(commit.watched[0], git_dir.join("HEAD"));
    }

    #[test]
    fn no_git_and_an_unknown_ref_name_no_commit() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(of(dir.path()).short, None);

        write(&dir.path().join(".git/HEAD"), "ref: refs/heads/unborn\n");
        assert_eq!(of(dir.path()).short, None);
    }
}
