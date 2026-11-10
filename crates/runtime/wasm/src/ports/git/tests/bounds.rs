//! What a guest's log and diff cost the host: counted over real repositories, never timed. Each fixture is many
//! commits or many files, and each test counts what the host walked or read for a guest whose answer holds little.

use std::sync::Arc;

use specforge_effects::testing::{self, MemoryConsent};
use specforge_effects::{EffectJournal, Effects, EffectsCeiling, Grant, Ports, Provenance, Roots};
use specforge_extension_sdk::prelude::{Capability, ContributionsBuilder, ExtensionMeta};
use specforge_git::Gix;
use specforge_git::testing::Repo;
use specforge_protocol_types::{GitChange, GitDiffHead, GitDiffQuery, GitLogQuery, Operation};

use crate::ports::{PortCall, PortSession};
use crate::sandbox::Limits;

/// A collector's session of an extension granted git reads, over the real port, in `repo`.
fn session(repo: &Repo) -> PortSession {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@t/a", "1.0.0"));
    c.capability(Capability::git("repo").why("read the history"));
    let roots = Roots::new(repo.dir(), repo.dir());
    let grant = Grant::decide(
        &c.declaration(),
        &Provenance::third_party(b"x"),
        &EffectsCeiling::default(),
        &roots,
        &MemoryConsent::approving_everything(),
    );
    let ports = Ports {
        git: Arc::new(Gix::default()),
        ..testing::ports()
    };
    let effects = Effects::online(ports.clone(), EffectJournal::at(repo.dir()));
    let call = PortCall::new(effects, &ports, roots, grant, Vec::new());
    PortSession::effectful("@t/a", Operation::Collect, Some(call), Limits::CEILING)
}

/// A history of `n` commits, each adding one file.
fn history(dir: &std::path::Path, n: usize) -> (Repo, String) {
    let repo = Repo::init(dir);
    let mut head = String::new();
    for i in 0..n {
        repo.write(&format!("f{i}.txt"), format!("{i}\n"));
        head = repo.commit_all(&format!("c{i}"));
    }
    (repo, head)
}

#[test]
fn a_guest_log_walks_the_whole_history_whatever_its_answer_holds() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, head) = history(dir.path(), 40);
    let mut session = session(&repo);
    // room for about two commits
    session.answer_cap = 400;
    let answer = session
        .git_log(&GitLogQuery {
            tips: vec![head],
            ..GitLogQuery::default()
        })
        .unwrap();
    assert!(answer.truncated && answer.commits.len() <= 3);
    // the walk counted every commit of the history
    assert_eq!(answer.count, Some(40));
}

/// A commit adding `n` files of `size` bytes each.
fn wide(dir: &std::path::Path, n: usize, size: usize) -> (Repo, String) {
    let repo = Repo::init(dir);
    for i in 0..n {
        repo.write(&format!("f{i:02}.txt"), "x\n".repeat(size / 2));
    }
    let head = repo.commit_all("wide");
    (repo, head)
}

fn counted(head: &str) -> GitDiffQuery {
    GitDiffQuery {
        base: None,
        head: GitDiffHead::Commit(head.to_string()),
        paths: Vec::new(),
        patch: false,
        hunks: false,
        renames: false,
    }
}

#[test]
fn a_guest_diff_reads_every_file_whatever_its_answer_holds() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, head) = wide(dir.path(), 40, 4096);
    let mut session = session(&repo);
    // the names and about two files' worth of bytes
    session.answer_cap = 40 * 7 + 2 * 4096;
    let answer = session.git_diff(&counted(&head)).unwrap();
    let read = answer.files.iter().filter(|f| f.added > 0).count();
    assert_eq!((answer.files.len(), read), (40, 40));
}

/// A rename search follows the repository's own config: whether to search, and how many pairs to compare.
#[test]
fn a_guest_diff_follows_the_repositorys_rename_config() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    let text = |tag: &str| {
        (0..20)
            .map(|i| format!("{tag} line {i}\n"))
            .collect::<String>()
    };
    repo.write("a.txt", text("a")).write("b.txt", text("b"));
    let base = repo.commit_all("base");
    // two renames, each with one line changed: a fuzzy search of four pairs pairs them
    repo.delete("a.txt")
        .delete("b.txt")
        .write("a2.txt", text("a").replace("line 3", "LINE 3"))
        .write("b2.txt", text("b").replace("line 3", "LINE 3"));
    let head = repo.commit_all("moved");
    let query = GitDiffQuery {
        base: Some(base),
        renames: true,
        ..counted(&head)
    };
    let statuses = |repo: &Repo| -> Vec<GitChange> {
        session(repo)
            .git_diff(&query)
            .unwrap()
            .files
            .iter()
            .map(|f| f.status)
            .collect()
    };
    assert_eq!(statuses(&repo), [GitChange::Renamed, GitChange::Renamed]);
    // the repository's config turns the search off
    repo.config("[diff]\n\trenames = false");
    assert_eq!(statuses(&repo).len(), 4);
}
