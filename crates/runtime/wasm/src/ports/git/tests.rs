//! The guest's `log` and `diff`: typed answers of the project's own repository, commits named by id only, a withheld
//! file's text never handed over, and an answer held to the call's cap.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use specforge_effects::git::{DiffStatus, GitCommit, GitFileDiff, GitHunk, GitLog, GitPerson};
use specforge_effects::testing::{self, FakeFs, FakeGit, MemoryConsent};
use specforge_effects::{EffectJournal, Effects, EffectsCeiling, Grant, Ports, Provenance, Roots};
use specforge_extension_sdk::prelude::{Capability, ContributionsBuilder, ExtensionMeta};
use specforge_protocol_types::{
    Denial, FsAccess, GitChange, GitDiffHead, GitDiffQuery, GitLogQuery, HostError, Operation,
};
use specforge_test_macros::test as specforge_test;

use crate::ports::{PortCall, PortSession};
use crate::sandbox::Limits;

const SHA: &str = "abcdefabcdefabcdefabcdefabcdefabcdefabcd";

/// A collector's session of an extension granted git reads, over `git`.
fn session(git: &Arc<FakeGit>, root: &Path) -> PortSession {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@t/a", "1.0.0"));
    c.capability(Capability::git("repo").why("read the history"));
    let roots = Roots::new(root, root);
    let grant = Grant::decide(
        &c.declaration(),
        &Provenance::third_party(b"x"),
        &EffectsCeiling::default(),
        &roots,
        &MemoryConsent::approving_everything(),
    );
    let ports = Ports {
        git: Arc::clone(git) as _,
        ..testing::ports()
    };
    let effects = Effects::online(ports.clone(), EffectJournal::at(root));
    let call = PortCall::new(effects, &ports, roots, grant, Vec::new());
    PortSession::effectful("@t/a", Operation::Collect, Some(call), Limits::CEILING)
}

fn person(seconds: i64) -> GitPerson {
    GitPerson {
        name: "A".into(),
        email: "a@example.test".into(),
        time: "2026-01-01T00:00:00+00:00".into(),
        seconds,
    }
}

fn file(path: &str, status: DiffStatus) -> GitFileDiff {
    GitFileDiff {
        path: path.into(),
        old_path: None,
        status,
        added: 2,
        removed: 1,
        binary: false,
        truncated: false,
        patch: Some("@@ -1 +1,2 @@\n-a\n+b\n+c\n".into()),
        hunks: Some(vec![GitHunk {
            before_start: 0,
            before_end: 1,
            after_start: 0,
            after_end: 2,
        }]),
    }
}

fn diff_query() -> GitDiffQuery {
    GitDiffQuery {
        base: None,
        head: GitDiffHead::Commit(SHA.into()),
        paths: Vec::new(),
        patch: true,
        hunks: true,
        renames: false,
    }
}

#[test]
fn a_guest_log_lists_typed_commits_with_trailers() {
    let dir = tempfile::tempdir().unwrap();
    let mut fake = FakeGit::repo(dir.path(), SHA, Vec::new());
    fake.log = GitLog {
        count: Some(1),
        commits: vec![GitCommit {
            sha: specforge_effects::git::Sha::new(SHA),
            parents: Vec::new(),
            author: person(1_700_000_000),
            committer: person(1_700_000_060),
            subject: "feat(01-01): x".into(),
            body: "Gsd-Task: 01-01/T1".into(),
            trailers: BTreeMap::from([("Gsd-Task".to_string(), vec!["01-01/T1".to_string()])]),
        }],
    };
    let git = Arc::new(fake);
    let answer = session(&git, dir.path())
        .git_log(&GitLogQuery {
            tips: vec![SHA.into()],
            pickaxe: Some("x".into()),
            paths: vec!["src".into()],
            ..GitLogQuery::default()
        })
        .unwrap();
    assert_eq!((answer.count, answer.truncated), (Some(1), false));
    let commit = &answer.commits[0];
    assert_eq!(commit.author.seconds, 1_700_000_000);
    assert_eq!(commit.trailers[0].token, "Gsd-Task");
    assert_eq!(commit.trailers[0].values, ["01-01/T1"]);
    // the port never reads a withheld file for a guest's pickaxe
    let asked = git.logs.lock().unwrap();
    assert!(asked[0].withhold);
    assert_eq!(asked[0].paths, ["src"]);
}

#[test]
fn a_guest_log_names_commits_by_sha_only() {
    let dir = tempfile::tempdir().unwrap();
    let git = Arc::new(FakeGit::repo(dir.path(), SHA, Vec::new()));
    let mut session = session(&git, dir.path());
    for (tips, hidden, paths) in [
        (vec!["HEAD".to_string()], vec![], vec![]),
        (vec!["--all".to_string()], vec![], vec![]),
        (vec![SHA.to_string()], vec!["main~1".to_string()], vec![]),
        (
            vec![SHA.to_string()],
            vec![],
            vec!["../outside".to_string()],
        ),
        (vec![SHA.to_string()], vec![], vec!["/etc".to_string()]),
    ] {
        let answer = session.git_log(&GitLogQuery {
            tips,
            hidden,
            paths,
            ..GitLogQuery::default()
        });
        assert!(
            matches!(answer, Err(HostError::Denied(Denial::Scope(_)))),
            "{answer:?}"
        );
    }
    let diff = session.git_diff(&GitDiffQuery {
        base: Some("HEAD~1".into()),
        ..diff_query()
    });
    assert!(matches!(diff, Err(HostError::Denied(Denial::Scope(_)))));
    assert_eq!((git.logs_asked(), git.diffs_asked()), (0, 0));
}

#[test]
fn a_pickaxe_over_a_withheld_path_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let git = Arc::new(FakeGit::repo(dir.path(), SHA, Vec::new()));
    for path in [".env", "deploy/id_rsa", "certs/server.pem"] {
        let answer = session(&git, dir.path()).git_log(&GitLogQuery {
            tips: vec![SHA.into()],
            paths: vec![path.into()],
            pickaxe: Some("TOKEN".into()),
            ..GitLogQuery::default()
        });
        assert!(
            matches!(&answer, Err(HostError::Denied(Denial::Scope(why))) if why.contains("withheld")),
            "{path}: {answer:?}"
        );
    }
    assert_eq!(git.logs_asked(), 0);
}

#[specforge_test(
    behavior = "grant_a_host_port_call",
    verify = "a guest diff never carries the text of a withheld file"
)]
fn a_withheld_file_in_a_guest_diff_has_no_text() {
    let dir = tempfile::tempdir().unwrap();
    let mut fake = FakeGit::repo(dir.path(), SHA, Vec::new());
    let mut renamed = file("env.txt", DiffStatus::Renamed);
    renamed.old_path = Some(".env".into());
    fake.diffs = vec![
        file(".env", DiffStatus::Modified),
        renamed,
        file("src/a.js", DiffStatus::Added),
    ];
    let git = Arc::new(fake);
    let answer = session(&git, dir.path()).git_diff(&diff_query()).unwrap();
    let [env, moved, source] = &answer.files[..] else {
        panic!("three files: {answer:?}");
    };
    for held in [env, moved] {
        assert!(held.withheld, "{held:?}");
        assert_eq!((held.patch.as_ref(), held.hunks.as_ref()), (None, None));
        // its status and counts stay
        assert_eq!((held.added, held.removed), (2, 1));
    }
    assert_eq!(moved.status, GitChange::Renamed);
    assert!(!source.withheld && source.patch.is_some() && source.hunks.is_some());
    assert_eq!(source.status, GitChange::Added);
    assert!(!answer.truncated);
    let asked = git.diffed.lock().unwrap();
    assert_eq!(asked[0].base, None);
    assert!(asked[0].hunks && !asked[0].renames);
}

#[test]
fn a_guest_diff_past_the_answer_cap_is_truncated() {
    let dir = tempfile::tempdir().unwrap();
    let mut fake = FakeGit::repo(dir.path(), SHA, Vec::new());
    fake.diffs = (0..4)
        .map(|n| file(&format!("f{n}.txt"), DiffStatus::Modified))
        .collect();
    let git = Arc::new(fake);
    let mut session = session(&git, dir.path());
    let names = 4 * "f0.txt".len() as u64;
    let text = "@@ -1 +1,2 @@\n-a\n+b\n+c\n".len() as u64 + 16;
    // room for the names and two files' text
    session.answer_cap = names + 2 * text + 1;
    let answer = session.git_diff(&diff_query()).unwrap();
    assert!(answer.truncated);
    let kept: Vec<bool> = answer.files.iter().map(|f| f.patch.is_some()).collect();
    assert_eq!(kept, [true, true, false, false]);
    assert!(answer.files[3].hunks.is_none());
    assert_eq!(answer.files.len(), 4, "every file is still named");
    // a cap the names alone pass is `capped`
    session.answer_cap = names - 1;
    let capped = session.git_diff(&diff_query());
    assert!(
        matches!(&capped, Err(HostError::Failed(f)) if f.class == "capped"),
        "{capped:?}"
    );
}

#[test]
fn a_guest_log_past_the_answer_cap_is_truncated() {
    let dir = tempfile::tempdir().unwrap();
    let mut fake = FakeGit::repo(dir.path(), SHA, Vec::new());
    let commit = |n: u8| GitCommit {
        sha: specforge_effects::git::Sha::new(format!("{n:040x}")),
        parents: Vec::new(),
        author: person(1),
        committer: person(1),
        subject: "s".into(),
        body: String::new(),
        trailers: BTreeMap::new(),
    };
    fake.log = GitLog {
        count: Some(3),
        commits: (1..=3).map(commit).collect(),
    };
    let git = Arc::new(fake);
    let mut session = session(&git, dir.path());
    session.answer_cap = 2 * (40 + 2 * (1 + 14 + 25 + 8) + 1);
    let answer = session
        .git_log(&GitLogQuery {
            tips: vec![SHA.into()],
            ..GitLogQuery::default()
        })
        .unwrap();
    assert_eq!((answer.commits.len(), answer.truncated), (2, true));
    assert_eq!(answer.count, Some(3), "the count still counts every commit");
}

/// A collector's session of an extension granted git reads and, with `fs_read`, fs reads of the project, over `git`
/// and `fs`.
fn session_on_disk(git: &Arc<FakeGit>, fs: FakeFs, root: &Path, fs_read: bool) -> PortSession {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@t/a", "1.0.0"));
    c.capability(Capability::git("repo").why("read the history"));
    if fs_read {
        c.capability(
            Capability::fs("project")
                .ops(&[FsAccess::Read])
                .why("read the worktree"),
        );
    }
    let roots = Roots::new(root, root);
    let grant = Grant::decide(
        &c.declaration(),
        &Provenance::third_party(b"x"),
        &EffectsCeiling::default(),
        &roots,
        &MemoryConsent::approving_everything(),
    );
    let ports = Ports {
        git: Arc::clone(git) as _,
        fs: Arc::new(fs),
        ..testing::ports()
    };
    let effects = Effects::online(ports.clone(), EffectJournal::at(root));
    let call = PortCall::new(effects, &ports, roots, grant, Vec::new());
    PortSession::effectful("@t/a", Operation::Collect, Some(call), Limits::CEILING)
}

/// A worktree's text is the disk's: a guest reads it only where its fs grant reads the file, through every link on
/// the way. Its git grant alone gives the file's status and counts.
#[specforge_test(
    behavior = "grant_a_host_port_call",
    verify = "a guest diff never carries a worktree file's text its fs grant does not read"
)]
fn a_guest_worktree_diff_carries_the_text_its_fs_grant_reads() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let outside = root.parent().unwrap().join("outside");
    let mut fake = FakeGit::repo(&root, SHA, Vec::new());
    fake.diffs = vec![
        file("src/a.js", DiffStatus::Modified),
        file("d/b.js", DiffStatus::Modified),
    ];
    let git = Arc::new(fake);
    let disk = || {
        FakeFs::new()
            .with_file(root.join("src/a.js"), "b\nc\n")
            .with_file(outside.join("b.js"), "secret\n")
            .with_link(root.join("d"), &outside)
    };
    let worktree = GitDiffQuery {
        head: GitDiffHead::Worktree,
        ..diff_query()
    };
    let texts = |answer: &specforge_protocol_types::GitDiffAnswer| -> Vec<(bool, bool)> {
        answer
            .files
            .iter()
            .map(|f| (f.withheld, f.patch.is_some() || f.hunks.is_some()))
            .collect()
    };
    // git reads only: status and counts, no text
    let answer = session_on_disk(&git, disk(), &root, false)
        .git_diff(&worktree)
        .unwrap();
    assert_eq!(texts(&answer), [(true, false), (true, false)]);
    assert!(answer.files.iter().all(|f| (f.added, f.removed) == (2, 1)));
    // fs reads of the project: the file in it, never the one a linked directory leads out to
    let answer = session_on_disk(&git, disk(), &root, true)
        .git_diff(&worktree)
        .unwrap();
    assert_eq!(texts(&answer), [(false, true), (true, false)]);
    // a commit's text is the repository's: the git grant reads it
    let answer = session_on_disk(&git, disk(), &root, false)
        .git_diff(&diff_query())
        .unwrap();
    assert_eq!(texts(&answer), [(false, true), (false, true)]);
}
