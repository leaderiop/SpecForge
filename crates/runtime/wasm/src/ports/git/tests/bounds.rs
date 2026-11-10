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
fn a_guest_log_stops_at_the_commits_the_host_walks() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, head) = history(dir.path(), 40);
    let mut session = session(&repo);
    let query = GitLogQuery {
        tips: vec![head],
        ..GitLogQuery::default()
    };
    session.log_walk = 10;
    let answer = session.git_log(&query).unwrap();
    // stopped: no count, and the answer says so
    assert_eq!((answer.count, answer.truncated), (None, true));
    assert_eq!(answer.commits.len(), 10);
    session.log_walk = 40;
    assert_eq!(session.git_log(&query).unwrap().count, Some(40));
}

#[test]
fn a_guest_pickaxe_log_stops_at_the_bytes_its_answer_holds() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    let mut head = String::new();
    for i in 0..40 {
        repo.write(&format!("f{i}.txt"), "x\n".repeat(2048));
        head = repo.commit_all(&format!("c{i}"));
    }
    let mut session = session(&repo);
    // about three of the forty blobs the pickaxe would read
    session.answer_cap = 3 * 4096;
    let answer = session
        .git_log(&GitLogQuery {
            tips: vec![head],
            pickaxe: Some("needle".into()),
            ..GitLogQuery::default()
        })
        .unwrap();
    assert_eq!((answer.count, answer.truncated), (None, true));
    assert!(answer.commits.is_empty());
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
fn a_guest_diff_reads_no_more_files_than_its_answer_holds() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, head) = wide(dir.path(), 40, 4096);
    let mut session = session(&repo);
    // the names and about two files' worth of bytes
    session.answer_cap = 40 * 7 + 2 * 4096;
    let answer = session.git_diff(&counted(&head)).unwrap();
    let (read, unread): (Vec<_>, Vec<_>) = answer.files.iter().partition(|f| f.added > 0);
    // every file is named; two are read, the rest say they were not
    assert_eq!((answer.files.len(), read.len()), (40, 2));
    assert!(answer.truncated && unread.iter().all(|f| f.truncated && f.removed == 0));
    // a diff of one file reads it, though its bytes are past the bound
    session.answer_cap = 64;
    let one = session
        .git_diff(&GitDiffQuery {
            paths: vec!["f07.txt".into()],
            ..counted(&head)
        })
        .unwrap();
    assert_eq!(one.files[0].added, 2048);
}

/// Text of 20 lines tagged `tag`.
fn text(tag: &str) -> String {
    (0..20).map(|i| format!("{tag} line {i}\n")).collect()
}

/// A rename search follows the host's rules, never the repository's config: it searches, and compares at most so many
/// pairs.
#[test]
fn a_guest_diff_searches_renames_by_the_hosts_rules() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", text("a")).write("b.txt", text("b"));
    let base = repo.commit_all("base");
    // two renames, each with one line changed: a fuzzy search of four pairs pairs them
    repo.delete("a.txt")
        .delete("b.txt")
        .write("a2.txt", text("a").replace("line 3", "LINE 3"))
        .write("b2.txt", text("b").replace("line 3", "LINE 3"));
    let head = repo.commit_all("moved");
    let statuses = |base: &str, head: &str| -> Vec<GitChange> {
        session(&repo)
            .git_diff(&GitDiffQuery {
                base: Some(base.to_string()),
                renames: true,
                ..counted(head)
            })
            .unwrap()
            .files
            .iter()
            .map(|f| f.status)
            .collect()
    };
    // the repository's config would turn the search off, and its limit of 0 would compare every pair
    repo.config("[diff]\n\trenames = false\n\trenameLimit = 0");
    assert_eq!(
        statuses(&base, &head),
        [GitChange::Renamed, GitChange::Renamed]
    );
    // 40 fuzzy renames are 1600 pairs, past the host's 1000: none is compared, each file is listed apart
    for i in 0..40 {
        repo.write(&format!("m{i:02}.txt"), text(&format!("m{i}")));
    }
    let wide = repo.commit_all("wide");
    for i in 0..40 {
        repo.delete(&format!("m{i:02}.txt")).write(
            &format!("n{i:02}.txt"),
            text(&format!("m{i}")).replace("line 3", "LINE 3"),
        );
    }
    let moved = repo.commit_all("moved wide");
    let wide = statuses(&wide, &moved);
    assert_eq!(wide.len(), 80);
    assert!(!wide.contains(&GitChange::Renamed));
}
