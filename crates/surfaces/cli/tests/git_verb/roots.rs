//! `specforge git` where several repositories meet: a repository with no identity, and a machine root.

use super::{committed, git};

/// A new machine has no `user.name`: git makes a name up for a branch's reflog, and so must the verb.
#[test]
fn a_worktree_on_a_new_branch_needs_no_identity() {
    let (project, repo, sha) = committed();
    std::fs::write(repo.git_dir().join("config"), "[core]\n\tbare = false\n").unwrap();
    git(&project, &["branch", "create", "workspace/x"]).exits(0);
    let added = git(
        &project,
        &[
            "worktree",
            "add",
            "wt/deep/repo",
            "--branch",
            "workspace/y",
            "--create",
        ],
    )
    .exits(0)
    .answer();
    assert_eq!(added["branch"], "workspace/y");
    assert_eq!(added["sha"], sha);
}

/// `specforge git <args>` typed in `project`, as a block types it: no `--path`, no git program on `PATH`.
fn typed(project: &crate::support::TestProject, args: &[&str]) -> crate::support::Run {
    let empty = tempfile::tempdir().unwrap();
    let mut all = vec!["git"];
    all.extend_from_slice(args);
    crate::support::Cli::on(project)
        .env("PATH", empty.path())
        .bare(&all)
}

/// A root named at `~/gsd-workspaces` on `project`'s machine.
fn workspaces(project: &crate::support::TestProject) -> std::path::PathBuf {
    crate::support::Cli::on(project)
        .run(&[
            "grants",
            "approve",
            "--root",
            "workspaces",
            "--at",
            "~/gsd-workspaces",
        ])
        .exits(0);
    project
        .home()
        .canonicalize()
        .unwrap()
        .join("gsd-workspaces")
}

fn at(path: &std::path::Path) -> String {
    path.to_str().unwrap().to_string()
}

#[test]
fn in_and_worktree_add_and_remove_reach_a_machine_root() {
    let (project, _repo, sha) = committed();
    let top = at(&project.root().canonicalize().unwrap());
    let base = workspaces(&project);
    for name in ["ws1", "ws2"] {
        let linked = base.join(name).join("repo");
        let linked_at = at(&linked);
        let branch = format!("workspace/{name}");
        let added = typed(
            &project,
            &[
                "worktree", "add", &linked_at, "--branch", &branch, "--create",
            ],
        )
        .exits(0)
        .answer();
        assert_eq!(added["sha"], sha);
        assert!(linked.join("a.txt").is_file());
        assert_eq!(
            typed(
                &project,
                &["--in", &linked_at, "status", "--pick", ".clean"]
            )
            .exits(0)
            .stdout,
            "true\n"
        );
        assert_eq!(
            typed(&project, &["--in", &linked_at, "repo", "--pick", ".linked"])
                .exits(0)
                .stdout,
            "true\n"
        );
        assert_eq!(
            typed(
                &project,
                &[
                    "--in",
                    &linked_at,
                    "worktrees",
                    "--pick",
                    ".worktrees[main=true].path"
                ]
            )
            .exits(0)
            .stdout,
            format!("{top}\n")
        );
    }
    let one = base.join("ws1/repo");
    std::fs::write(one.join("new.txt"), "work\n").unwrap();
    assert_eq!(
        typed(&project, &["--in", &at(&one), "status", "--pick", ".clean"])
            .exits(0)
            .stdout,
        "false\n"
    );
    let dirty = typed(&project, &["worktree", "remove", &at(&one)]).exits(1);
    assert_eq!(dirty.json()["code"], "dirty_tree", "{}", dirty.stdout);
    std::fs::remove_file(one.join("new.txt")).unwrap();
    for name in ["ws1", "ws2"] {
        typed(
            &project,
            &["worktree", "remove", &at(&base.join(name).join("repo"))],
        )
        .exits(0);
    }
    assert!(!one.exists());
}

/// `--in` and a worktree's path are typed paths: the root holds them as it holds a verb's.
#[test]
fn nothing_leads_a_git_verb_out_of_a_machine_root() {
    let (project, _repo, _) = committed();
    let base = workspaces(&project);
    let home = project.home().canonicalize().unwrap();
    let outer = base.join("../outer-worktree");
    let run = typed(
        &project,
        &["worktree", "add", &at(&outer), "--branch", "o", "--create"],
    )
    .exits(1);
    assert_eq!(run.json()["code"], "outside_root", "{}", run.stdout);
    assert!(!home.join("outer-worktree").exists());
    typed(&project, &["--in", &at(&base.join("..")), "status"]).exits(1);
    typed(
        &project,
        &["worktree", "add", &at(&base), "--branch", "r", "--create"],
    );
    assert!(
        !base.join("a.txt").exists(),
        "the root itself takes no worktree"
    );
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&home, base.join("up")).unwrap();
        let through = base.join("up/made");
        typed(
            &project,
            &[
                "worktree",
                "add",
                &at(&through),
                "--branch",
                "u",
                "--create",
            ],
        )
        .exits(1);
        assert!(!home.join("made").exists());
        typed(&project, &["--in", &at(&base.join("up")), "status"]).exits(1);
    }
}

/// A repository that can't be read never answers as the one around it: the workspace guard reads it as not clean.
#[test]
fn a_broken_repository_in_a_machine_root_is_not_read_as_the_one_around_it() {
    let (project, _repo, _) = committed();
    let base = workspaces(&project);
    // The home is a clean repository of its own (a dotfiles repository).
    let home = project.home().canonicalize().unwrap();
    let dotfiles = specforge_git::testing::Repo::init(&home);
    dotfiles.write(".gitignore", "*\n");
    dotfiles.commit_all("first");
    let broken = base.join("ws/repo");
    std::fs::create_dir_all(&broken).unwrap();
    std::fs::write(broken.join(".git"), "gitdir: /nonexistent\n").unwrap();
    let run = typed(
        &project,
        &["--in", &at(&broken), "status", "--pick", ".clean"],
    );
    assert_ne!(run.code, 0, "{}", run.stdout);
    assert_eq!(run.stdout, "");
}

#[test]
fn clone_copies_a_local_repository_into_a_machine_root() {
    let (project, _repo, sha) = committed();
    let base = workspaces(&project);
    let into = base.join("ws/clone");
    let answer = typed(&project, &["clone", ".", &at(&into)])
        .exits(0)
        .answer();
    assert_eq!(answer["top"], at(&into));
    assert_eq!(answer["branch"], "main");
    assert_eq!(answer["sha"], sha);
    assert!(into.join("a.txt").is_file());
    assert_eq!(
        typed(
            &project,
            &["--in", &at(&into), "remotes", "--pick", ".remotes[0].name"]
        )
        .exits(0)
        .stdout,
        "origin\n"
    );
    assert_eq!(
        typed(
            &project,
            &["--in", &at(&into), "status", "--pick", ".clean"]
        )
        .exits(0)
        .stdout,
        "true\n"
    );
    // The new repository's own directory is the root's to remove, not a worktree of the source.
    crate::support::Cli::on(&project)
        .run(&["files", "remove", "-r", &at(&base.join("ws"))])
        .exits(0);
    assert!(!into.exists());
}

#[test]
fn clone_refuses_a_url_and_a_taken_directory() {
    let (project, _repo, _) = committed();
    let base = workspaces(&project);
    let into = at(&base.join("clone"));
    for remote in [
        "https://example.com/r.git",
        "ssh://git@example.com/r.git",
        "file:///tmp/r",
        "git@example.com:r.git",
        "example.com:r",
    ] {
        let run = typed(&project, &["clone", remote, &into]).exits(1);
        assert!(
            run.stdout.contains("is a remote"),
            "{remote}: {}",
            run.stdout
        );
    }
    std::fs::create_dir_all(base.join("taken")).unwrap();
    std::fs::write(base.join("taken/file"), "").unwrap();
    let run = typed(&project, &["clone", ".", &at(&base.join("taken"))]).exits(1);
    assert_eq!(run.json()["code"], "path_taken", "{}", run.stdout);
    let run = typed(&project, &["clone", ".", &at(&base.join("taken/file"))]).exits(1);
    assert_eq!(run.json()["code"], "path_taken", "{}", run.stdout);
    // An empty root is still a root.
    let empty = project.home().canonicalize().unwrap().join("gsd-empty");
    crate::support::Cli::on(&project)
        .run(&["grants", "approve", "--root", "empty", "--at", &at(&empty)])
        .exits(0);
    let run = typed(&project, &["clone", ".", &at(&empty)]).exits(1);
    assert_eq!(run.json()["code"], "path_taken", "{}", run.stdout);
    let run = typed(
        &project,
        &["worktree", "add", &at(&empty), "--branch", "b", "--create"],
    )
    .exits(1);
    assert_eq!(run.json()["code"], "path_taken", "{}", run.stdout);
    assert!(!base.join("clone").exists());
}

/// Each side of a clone is a typed path: neither end leaves the roots, and the source is held for a read.
#[test]
fn clone_holds_both_ends_to_the_roots() {
    let (project, _repo, _) = committed();
    let base = workspaces(&project);
    let home = project.home().canonicalize().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    specforge_git::testing::Repo::init(elsewhere.path());
    let run = typed(
        &project,
        &["clone", &at(elsewhere.path()), &at(&base.join("c1"))],
    )
    .exits(1);
    assert_eq!(run.json()["code"], "outside_root", "{}", run.stdout);
    assert!(run.stdout.contains("--at <dir>"), "{}", run.stdout);
    let run = typed(&project, &["clone", ".", &at(&home.join("c2"))]).exits(1);
    assert_eq!(run.json()["code"], "outside_root", "{}", run.stdout);
    assert!(!home.join("c2").exists());
    let run = typed(&project, &["clone", ".", &at(&base.join("../c3"))]).exits(1);
    assert_eq!(run.json()["code"], "outside_root", "{}", run.stdout);
    assert!(!home.join("c3").exists());
    let run = typed(&project, &["clone", ".", &at(&base.join("x/.specforge/y"))]).exits(1);
    assert_eq!(run.json()["code"], "permission_denied", "{}", run.stdout);
    let run = typed(&project, &["clone", &at(&base), &at(&base.join("c4"))]).exits(1);
    assert_eq!(run.json()["code"], "not_a_repository", "{}", run.stdout);
    // A ceiling that leaves git no write refuses it first.
    let read_only = crate::support::TestProject::new()
        .config(|config| config["effects"] = serde_json::json!({"git": {"ops": ["read"]}}));
    specforge_git::testing::Repo::init(read_only.root());
    let run = crate::support::Cli::on(&read_only)
        .at_home(project.home())
        .bare(&["git", "clone", ".", &at(&base.join("c5"))])
        .exits(1);
    assert_eq!(run.json()["code"], "E090", "{}", run.stdout);
}
