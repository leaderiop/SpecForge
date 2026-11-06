//! A machine root (ADR 0335's S1 amendment): named once on this machine, reached by every project's verbs under its
//! ceiling, and it holds a verb exactly as a root of the project does: no link, `..` or second name leads out of it.

use crate::support::{Cli, TestProject};

/// A project, and a `workspaces` root at `~/gsd-workspaces` on its machine.
fn approved() -> (TestProject, std::path::PathBuf) {
    let project = TestProject::new().files(&[("a.txt", "a\n")]);
    Cli::on(&project)
        .run(&[
            "grants",
            "approve",
            "--root",
            "workspaces",
            "--at",
            "~/gsd-workspaces",
        ])
        .exits(0);
    let target = project
        .home()
        .canonicalize()
        .unwrap()
        .join("gsd-workspaces");
    (project, target)
}

fn at(path: &std::path::Path) -> String {
    path.to_str().unwrap().to_string()
}

#[test]
fn a_machine_root_is_reached_from_any_project_under_its_ceiling() {
    let project = TestProject::new().files(&[("a.txt", "a\n")]);
    let target = project
        .home()
        .canonicalize()
        .unwrap()
        .join("gsd-workspaces");
    std::fs::create_dir_all(&target).unwrap();
    let path = at(&target);

    // Before the approval it is in no root.
    let before = Cli::on(&project).run(&["files", "stat", &path]).exits(1);
    assert_eq!(before.json()["code"], "outside_root", "{}", before.stdout);
    assert!(before.stdout.contains("--at <dir>"), "{}", before.stdout);

    Cli::on(&project)
        .run(&["grants", "approve", "--root", "workspaces", "--at", &path])
        .exits(0);
    let made = at(&target.join("ws/sub"));
    Cli::on(&project).run(&["files", "stat", &path]).exits(0);
    Cli::on(&project).run(&["files", "mkdir", &made]).exits(0);
    assert!(target.join("ws/sub").is_dir());
    Cli::on(&project)
        .run(&["files", "remove", "-r", &at(&target.join("ws"))])
        .exits(0);
    assert!(!target.join("ws").exists());

    // From a directory with no specforge.json, on the same machine.
    let loose = tempfile::tempdir().unwrap();
    let run = Cli::on(&project)
        .in_dir(loose.path())
        .bare(&["files", "stat", &path, "--pick", ".entries[0].kind"])
        .exits(0);
    assert_eq!(run.stdout, "dir\n");

    // A ceiling that lists only the project's root, or switches the port off, refuses it.
    for effects in [r#"{"fs":{"roots":["project"]}}"#, r#"{"fs":"off"}"#] {
        let narrowed = TestProject::new().config(|config| {
            config["effects"] = serde_json::from_str(effects).unwrap();
        });
        let run = Cli::on(&narrowed)
            .at_home(project.home())
            .run(&["files", "stat", &path])
            .exits(1);
        assert_eq!(run.json()["code"], "E090", "{effects}: {}", run.stdout);
    }
    // One that lists it keeps it.
    let listed = TestProject::new().config(|config| {
        config["effects"] = serde_json::json!({"fs": {"roots": ["project", "workspaces"]}});
    });
    Cli::on(&listed)
        .at_home(project.home())
        .run(&["files", "stat", &path])
        .exits(0);

    // Revoked, it is gone everywhere.
    Cli::on(&project)
        .run(&["grants", "revoke", "--root", "workspaces", "--machine"])
        .exits(0);
    Cli::on(&project).run(&["files", "stat", &path]).exits(1);
}

#[test]
fn a_machine_root_whose_target_became_a_link_is_not_reached() {
    let (project, target) = approved();
    let elsewhere = tempfile::tempdir().unwrap();
    std::fs::write(elsewhere.path().join("secret"), "s").unwrap();
    std::fs::remove_dir(&target).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(elsewhere.path(), &target).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(elsewhere.path(), &target).unwrap();
    let run = Cli::on(&project)
        .run(&["files", "read", &at(&target.join("secret"))])
        .exits(1);
    assert_eq!(run.json()["code"], "outside_root", "{}", run.stdout);
    let listed = Cli::on(&project)
        .run(&["grants", "--format", "json"])
        .exits(0)
        .json();
    assert_eq!(listed["roots"][0]["approved"], false, "{listed}");
}

/// What the root must not let a typed path do: leave it by `..`, a link or a second name, or reach what is withheld.
#[test]
fn nothing_leads_out_of_a_machine_root() {
    let (project, target) = approved();
    let home = project.home().canonicalize().unwrap();
    std::fs::write(home.join("secret"), "TOKEN=hunter2\n").unwrap();
    std::fs::create_dir_all(target.join("ws")).unwrap();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(home.join("secret"), target.join("ws/link")).unwrap();
        std::os::unix::fs::symlink(&home, target.join("ws/dir")).unwrap();
    }
    std::fs::hard_link(home.join("secret"), target.join("ws/second")).unwrap();
    std::fs::create_dir_all(target.join("ws/.specforge")).unwrap();
    std::fs::write(target.join("ws/.specforge/state"), "TOKEN=hunter2\n").unwrap();
    std::fs::write(target.join("ws/.env"), "TOKEN=hunter2\n").unwrap();
    let read = |path: std::path::PathBuf| {
        let run = Cli::on(&project).run(&["files", "read", &at(&path)]);
        assert_ne!(run.code, 0, "{} was read", path.display());
        assert!(!run.stdout.contains("hunter2"), "{}", run.stdout);
    };
    read(target.join("../secret"));
    read(target.join("ws/../../secret"));
    read(home.join("secret"));
    read(target.join("ws/link"));
    read(target.join("ws/dir/secret"));
    read(target.join("ws/second"));
    read(target.join("ws/.specforge/state"));
    read(target.join("ws/.env"));
    // Nor is the root itself changed, nor a write taken through a link.
    Cli::on(&project)
        .run(&["files", "remove", "-r", &at(&target)])
        .exits(1);
    assert!(target.is_dir());
    #[cfg(unix)]
    {
        Cli::on(&project)
            .run(&["files", "mkdir", &at(&target.join("ws/dir/made"))])
            .exits(1);
        assert!(!home.join("made").exists());
    }
    // A name the store never held reaches nothing.
    let run = Cli::on(&project).run(&[
        "files",
        "read",
        &at(&target.parent().unwrap().join("other")),
    ]);
    assert_ne!(run.code, 0);
}

/// A root of the project's own takes the name: the machine root is not reached there, and the listing says so.
#[test]
fn a_projects_own_root_keeps_the_name() {
    let (project, target) = approved();
    let own = TestProject::new()
        .files(&[("mine/a.txt", "a\n")])
        .config(|config| config["roots"] = serde_json::json!({"workspaces": "mine"}));
    let run = Cli::on(&own)
        .at_home(project.home())
        .run(&["files", "stat", &at(&target)])
        .exits(1);
    assert_eq!(run.json()["code"], "outside_root", "{}", run.stdout);
    let listed = Cli::on(&own)
        .at_home(project.home())
        .run(&["grants", "--format", "json"])
        .exits(0)
        .json();
    let machine: Vec<_> = listed["roots"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["machine"] == true)
        .collect();
    assert_eq!(machine[0]["approved"], false, "{listed}");
}
