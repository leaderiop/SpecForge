//! A planted `.git` leads a guest nowhere outside its call's roots: a `.git` file, a `commondir` and an object
//! alternate are redirects, each refused as a person's verb refuses it (`within_roots`), and one within the roots is
//! honoured.

use std::path::Path;
use std::sync::Arc;

use specforge_effects::testing::{self, MemoryConsent};
use specforge_effects::{EffectJournal, Effects, EffectsCeiling, Grant, Ports, Provenance, Roots};
use specforge_extension_sdk::prelude::{Capability, ContributionsBuilder, ExtensionMeta};
use specforge_git::Gix;
use specforge_git::testing::Repo;
use specforge_protocol_types::{Denial, GitLogQuery, HostError, Operation};

use crate::ports::{PortCall, PortSession};
use crate::sandbox::Limits;

/// A collector's session of an extension granted git reads, over the real port, its roots `project` alone.
fn session(project: &Path) -> PortSession {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@t/a", "1.0.0"));
    c.capability(Capability::git("repo").why("read the history"));
    let roots = Roots::new(project, project);
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
    let effects = Effects::online(ports.clone(), EffectJournal::at(project));
    let call = PortCall::new(effects, &ports, roots, grant, Vec::new());
    PortSession::effectful("@t/a", Operation::Collect, Some(call), Limits::CEILING)
}

/// A repository beside the project, out of its roots, with one commit: the commit's sha.
fn secret(dir: &Path) -> (Repo, String) {
    let repo = Repo::init(&dir.join("secret"));
    repo.write("key.txt", "SECRET=hunter3\n");
    let sha = repo.commit_all("the secret commit");
    (repo, sha)
}

/// `answer` is the refusal of a repository that leads out of the call's roots, naming no secret.
fn refused<T: std::fmt::Debug>(answer: Result<T, HostError>) {
    assert!(
        matches!(&answer, Err(HostError::Denied(Denial::Scope(why))) if why.contains("in no root")),
        "{answer:?}"
    );
    assert!(!format!("{answer:?}").contains("hunter3"));
}

/// What a guest's log of `sha` answers in `project`: the subjects it read, or the error.
fn logged(project: &Path, sha: &str) -> Result<Vec<String>, HostError> {
    session(project)
        .git_log(&GitLogQuery {
            tips: vec![sha.into()],
            ..GitLogQuery::default()
        })
        .map(|answer| answer.commits.into_iter().map(|c| c.subject).collect())
}

#[test]
fn a_planted_git_file_leads_a_guest_nowhere() {
    let dir = tempfile::tempdir().unwrap();
    let (secret, sha) = secret(dir.path());
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(
        project.join(".git"),
        format!("gitdir: {}\n", secret.git_dir().display()),
    )
    .unwrap();
    refused(logged(&project, &sha));
    refused(session(&project).git_locate());
}

/// A `.git` that is a real directory and hands its refs to another: git's own spelling of a linked worktree's, forged.
#[test]
fn a_planted_commondir_leads_a_guest_nowhere() {
    let dir = tempfile::tempdir().unwrap();
    let (secret, sha) = secret(dir.path());
    let project = dir.path().join("project");
    std::fs::create_dir_all(project.join(".git")).unwrap();
    std::fs::write(project.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(
        project.join(".git/commondir"),
        format!("{}\n", secret.git_dir().display()),
    )
    .unwrap();
    std::fs::write(
        project.join(".git/gitdir"),
        format!("{}\n", project.join(".git").display()),
    )
    .unwrap();
    let answer = session(&project).git_commit("HEAD");
    assert!(!format!("{answer:?}").contains(&sha), "{answer:?}");
    refused(answer);
}

#[test]
fn a_planted_alternate_leads_a_guest_nowhere_and_one_within_the_roots_is_read() {
    let dir = tempfile::tempdir().unwrap();
    let (secret, sha) = secret(dir.path());
    let project = dir.path().join("project");
    let repo = Repo::init(&project);
    repo.write("a.txt", "a\n");
    repo.commit_all("project commit");
    std::fs::write(
        repo.git_dir().join("objects/info/alternates"),
        format!("{}\n", secret.git_dir().join("objects").display()),
    )
    .unwrap();
    refused(logged(&project, &sha));
    // the same objects moved into the project are its own, and read
    let near = project.join("near-objects");
    copy_dir(&secret.git_dir().join("objects"), &near);
    std::fs::write(
        repo.git_dir().join("objects/info/alternates"),
        format!("{}\n", near.display()),
    )
    .unwrap();
    assert_eq!(logged(&project, &sha).unwrap(), ["the secret commit"]);
}

/// A repository whose git directory lies in the project, beside its worktree (`--separate-git-dir`), is within reach.
#[test]
fn a_git_file_naming_a_directory_within_the_roots_is_read() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("project");
    let repo = Repo::init_separate(&project, &project.join(".store/project.git"));
    repo.write("a.txt", "a\n");
    let sha = repo.commit_all("held commit");
    assert_eq!(logged(&project, &sha).unwrap(), ["held commit"]);
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}
