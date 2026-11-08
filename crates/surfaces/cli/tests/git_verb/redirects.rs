//! A planted `.git` reaches nothing outside: a `.git` file or link, a `commondir` and an alternate are redirects, each
//! honoured only within reach, or as a real linked worktree or submodule that names this `.git` back. A walk upward to the
//! repository around a project is no redirect.

use super::{committed, git};
use crate::support::{Cli, TestProject, at};
use specforge_git::testing::Repo;

/// A repository out of every root, with a commit holding `s.txt`.
fn outside() -> (tempfile::TempDir, Repo) {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("s.txt", "OUTSIDE=hunter3\n");
    repo.commit_all("secret");
    (dir, repo)
}

/// Every way a `fake` directory of `project` can be led at `outside`, and each verb stays out.
fn refused_through_fake(project: &TestProject, repo: &Repo) {
    let fake = at(&project.root().join("fake"));
    let show = git(
        project,
        &["--in", &fake, "show", "HEAD:s.txt", "--pick", ".text"],
    );
    assert_ne!(show.code, 0, "{}", show.stdout);
    assert!(!show.stdout.contains("hunter3"), "{}", show.stdout);
    assert!(!show.stderr.contains("hunter3"), "{}", show.stderr);
    let tag = git(project, &["--in", &fake, "tag", "planted"]);
    assert_ne!(tag.code, 0, "{}", tag.stdout);
    assert!(
        !repo.git_dir().join("refs/tags/planted").exists(),
        "a tag was written outside"
    );
    let into = project.root().join("copied");
    let clone = git(project, &["clone", "fake", &at(&into)]);
    assert_ne!(clone.code, 0, "{}", clone.stdout);
    assert!(!into.join("s.txt").exists(), "the outside was copied");
}

#[test]
fn a_planted_git_file_reaches_nothing_outside() {
    let (project, _repo, _) = committed();
    let (_dir, secret) = outside();
    let fake = project.root().join("fake");
    std::fs::create_dir(&fake).unwrap();
    std::fs::write(
        fake.join(".git"),
        format!("gitdir: {}\n", secret.git_dir().display()),
    )
    .unwrap();
    refused_through_fake(&project, &secret);
}

#[cfg(unix)]
#[test]
fn a_planted_git_link_reaches_nothing_outside() {
    let (project, _repo, _) = committed();
    let (_dir, secret) = outside();
    let fake = project.root().join("fake");
    std::fs::create_dir(&fake).unwrap();
    std::os::unix::fs::symlink(secret.git_dir(), fake.join(".git")).unwrap();
    refused_through_fake(&project, &secret);
}

/// A `.git` that is a real directory and hands its refs to another: git's own spelling of a linked worktree's.
#[test]
fn a_planted_commondir_reaches_nothing_outside() {
    let (project, _repo, _) = committed();
    let (_dir, secret) = outside();
    let fake = project.root().join("fake");
    std::fs::create_dir_all(fake.join(".git")).unwrap();
    std::fs::write(fake.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(
        fake.join(".git/commondir"),
        format!("{}\n", secret.git_dir().display()),
    )
    .unwrap();
    // Forged to name itself back: only an admin directory of the common one is a worktree's.
    std::fs::write(
        fake.join(".git/gitdir"),
        format!("{}\n", fake.join(".git").display()),
    )
    .unwrap();
    refused_through_fake(&project, &secret);
}

/// `core.worktree` leads a repository's worktree reads out as a `.git` file leads its object reads.
#[test]
fn a_planted_core_worktree_reaches_nothing_outside() {
    let (project, _repo, _) = committed();
    let (dir, _secret) = outside();
    let fake = project.root().join("fake");
    std::fs::create_dir(&fake).unwrap();
    let planted = Repo::init(&fake);
    planted.write("s.txt", "plain\n");
    planted.commit_all("first");
    let config = fake.join(".git/config");
    let mut text = std::fs::read_to_string(&config).unwrap();
    text.push_str(&format!("[core]\n\tworktree = {}\n", dir.path().display()));
    std::fs::write(&config, text).unwrap();
    let run = git(
        &project,
        &["--in", &at(&fake), "diff", "--worktree", "--patch"],
    );
    assert_ne!(run.code, 0, "{}", run.stdout);
    assert!(!run.stdout.contains("hunter3"), "{}", run.stdout);
}

#[test]
fn an_alternate_outside_reach_refuses_and_one_within_it_does_not() {
    let (project, repo, _) = committed();
    let (_dir, secret) = outside();
    let alternates = repo.git_dir().join("objects/info/alternates");
    std::fs::create_dir_all(alternates.parent().unwrap()).unwrap();
    std::fs::write(
        &alternates,
        format!(
            "# a comment\n{}\n",
            secret.git_dir().join("objects").display()
        ),
    )
    .unwrap();
    let run = git(&project, &["status"]);
    assert_ne!(run.code, 0, "{}", run.stdout);
    assert!(
        run.stdout.contains("alternate") || run.stdout.contains(".git"),
        "{}",
        run.stdout
    );

    // One inside the project is the project's own, and is honoured.
    let near = project.root().join("near-objects");
    std::fs::create_dir_all(near.join("info")).unwrap();
    std::fs::write(&alternates, format!("{}\n", near.display())).unwrap();
    git(&project, &["status"]).exits(0);

    // One within reach that names another out of it leads out all the same.
    std::fs::write(
        near.join("info/alternates"),
        format!("{}\n", secret.git_dir().join("objects").display()),
    )
    .unwrap();
    assert_ne!(git(&project, &["status"]).code, 0);
}

/// A project nested in a repository finds it by a walk, a redirect of nothing, whichever roots hold it. Yet a clone opens
/// exactly what it is named: a nested project cannot copy the repository around it.
#[test]
fn a_nested_project_walks_to_its_repository_and_cannot_clone_it() {
    let dir = tempfile::tempdir().unwrap();
    let monorepo = Repo::init(dir.path());
    monorepo.write("top.txt", "OUTER=hunter3\n");
    let project = TestProject::new();
    let nested = dir.path().join("apps/one");
    std::fs::create_dir_all(nested.parent().unwrap()).unwrap();
    std::fs::rename(project.root(), &nested).unwrap();
    monorepo.commit_all("first");
    let in_nested = |args: &[&str]| {
        let mut all = vec!["git"];
        all.extend_from_slice(args);
        Cli::on(&project).in_dir(&nested).bare(&all)
    };
    let top = in_nested(&["repo", "--pick", ".top"]).exits(0);
    assert_eq!(
        top.stdout.trim(),
        dir.path().canonicalize().unwrap().to_str().unwrap()
    );
    in_nested(&["status"]).exits(0);
    let clone = in_nested(&["clone", ".", "copied"]);
    assert_ne!(clone.code, 0, "{}", clone.stdout);
    assert!(!nested.join("copied/top.txt").exists());
}

/// A real linked worktree names its `.git` back, so its repository is honoured wherever it lies. A `.git` that points at
/// an admin directory naming another worktree is a plant.
#[test]
fn a_linked_worktree_is_honoured_and_a_forged_one_is_not() {
    let (project, _repo, _) = committed();
    let (_dir, secret) = outside();
    secret.write("t.txt", "more\n");
    let sha = secret.commit_all("more");
    let held = project.root().join("wt");
    secret.linked_worktree(&held, "work", &sha);
    let status = git(
        &project,
        &["--in", &at(&held), "status", "--pick", ".clean"],
    )
    .exits(0);
    assert_eq!(status.stdout, "true\n");

    let fake = project.root().join("fake");
    std::fs::create_dir(&fake).unwrap();
    let admin = secret.git_dir().join("worktrees").join("wt");
    assert!(admin.is_dir(), "{}", admin.display());
    std::fs::write(fake.join(".git"), format!("gitdir: {}\n", admin.display())).unwrap();
    let run = git(&project, &["--in", &at(&fake), "show", "HEAD:s.txt"]);
    assert_ne!(run.code, 0, "{}", run.stdout);
    assert!(!run.stdout.contains("hunter3"), "{}", run.stdout);
}

/// A submodule's git directory lies in its superproject's, which a project at the submodule's top does not hold: its
/// `core.worktree` names it back. A `.git` pointing at a module that names another worktree is a plant.
#[test]
fn a_submodule_is_honoured_and_a_forged_one_is_not() {
    let project = TestProject::new();
    let dir = tempfile::tempdir().unwrap();
    let modules = dir.path().join("super/.git/modules");
    std::fs::create_dir_all(&modules).unwrap();
    let inner = Repo::init(project.root());
    inner.write("b.txt", "b\n");
    inner.commit_all("second");
    let module = modules.join("sub");
    std::fs::rename(project.root().join(".git"), &module).unwrap();
    std::fs::write(
        project.root().join(".git"),
        format!("gitdir: {}\n", module.display()),
    )
    .unwrap();
    let config = module.join("config");
    let mut text = std::fs::read_to_string(&config).unwrap();
    text.push_str(&format!(
        "[core]\n\tworktree = {}\n",
        project.root().canonicalize().unwrap().display()
    ));
    std::fs::write(&config, text).unwrap();
    git(&project, &["status"]).exits(0);

    let fake = project.root().join("fake");
    std::fs::create_dir(&fake).unwrap();
    std::fs::write(fake.join(".git"), format!("gitdir: {}\n", module.display())).unwrap();
    let run = git(&project, &["--in", &at(&fake), "show", "HEAD:b.txt"]);
    assert_ne!(run.code, 0, "{}", run.stdout);
}

/// A `.git` file whose directory lies within reach (`git init --separate-git-dir` inside the project) is honoured.
#[test]
fn a_git_file_within_reach_is_honoured() {
    let (project, repo, _) = committed();
    let moved = project.root().join("kept-git");
    std::fs::rename(repo.git_dir(), &moved).unwrap();
    std::fs::write(
        project.root().join(".git"),
        format!("gitdir: {}\n", moved.display()),
    )
    .unwrap();
    git(&project, &["status"]).exits(0);
}
