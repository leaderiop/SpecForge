//! A planted `.git` leads a guest nowhere outside its call's roots: a `.git` file, a `commondir` and an object
//! alternate are redirects, each judged as a person's verb judges it (`Git::leads_to` against the roots).

use std::path::Path;
use std::sync::Arc;

use specforge_effects::testing::{self, MemoryConsent};
use specforge_effects::{EffectJournal, Effects, EffectsCeiling, Grant, Ports, Provenance, Roots};
use specforge_extension_sdk::prelude::{Capability, ContributionsBuilder, ExtensionMeta};
use specforge_git::Gix;
use specforge_git::testing::Repo;
use specforge_protocol_types::{GitLogQuery, HostError, Operation};

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
fn a_planted_git_file_leads_a_guest_to_the_repository_it_names() {
    let dir = tempfile::tempdir().unwrap();
    let (secret, sha) = secret(dir.path());
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(
        project.join(".git"),
        format!("gitdir: {}\n", secret.git_dir().display()),
    )
    .unwrap();
    assert_eq!(
        logged(&project, &sha).unwrap(),
        ["the secret commit"],
        "pinned: the guest reads the repository outside its roots"
    );
    assert!(session(&project).git_locate().unwrap().is_some());
}

/// A `.git` that is a real directory and hands its refs to another: git's own spelling of a linked worktree's, forged.
#[test]
fn a_planted_commondir_leads_a_guest_to_the_refs_it_names() {
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
    assert_eq!(
        session(&project).git_commit("HEAD").unwrap().as_deref(),
        Some(sha.as_str()),
        "pinned: the guest resolves HEAD through the planted commondir"
    );
}

#[test]
fn a_planted_alternate_leads_a_guest_to_the_objects_it_names() {
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
    assert_eq!(
        logged(&project, &sha).unwrap(),
        ["the secret commit"],
        "pinned: the guest reads the secret commit through the alternate"
    );
}
