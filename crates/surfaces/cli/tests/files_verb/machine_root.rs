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

/// The store, as a hand or another program left it: its entries are not trusted for having been written.
#[cfg(unix)]
fn forge_store(home: &std::path::Path, machine_roots: serde_json::Value) {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = home.join(".specforge");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    let store = dir.join("consent.json");
    std::fs::write(
        &store,
        serde_json::json!({"version": 2, "machine_roots": machine_roots}).to_string(),
    )
    .unwrap();
    std::fs::set_permissions(&store, std::fs::Permissions::from_mode(0o600)).unwrap();
}

/// Every machine root is judged again when it is read, by the check `approve` uses: a forged entry reaches nothing, the
/// listing says W186 and `approved` agrees with what the verbs do.
#[cfg(unix)]
#[test]
fn a_forged_machine_root_is_ignored_when_the_store_is_read() {
    let project = TestProject::new().files(&[("a.txt", "a\n")]);
    let home = project.home().canonicalize().unwrap();
    let gh = home.join(".config/gh");
    std::fs::create_dir_all(&gh).unwrap();
    std::fs::write(gh.join("hosts.yml"), "oauth_token: hunter2\n").unwrap();
    std::fs::write(home.join("note"), "hunter2\n").unwrap();
    let fine = home.join("fine");
    std::fs::create_dir_all(&fine).unwrap();
    let good = home.join("good");
    std::fs::create_dir_all(&good).unwrap();
    forge_store(
        &home,
        serde_json::json!([
            {"root": "everything", "target": "/"},
            {"root": "home", "target": at(&home)},
            {"root": "state", "target": at(&home.join(".specforge"))},
            {"root": "gh", "target": at(&gh)},
            {"root": "system", "target": "/etc"},
            {"root": "spec", "target": at(&good)},
            {"root": "Bad_Name", "target": at(&good)},
            {"root": "fine", "target": at(&fine)},
        ]),
    );
    for path in [
        gh.join("hosts.yml"),
        home.join("note"),
        home.join(".specforge/consent.json"),
        good.clone(),
    ] {
        let run = Cli::on(&project).run(&["files", "read", &at(&path)]);
        assert_ne!(run.code, 0, "{} was reached", path.display());
        assert!(!run.stdout.contains("hunter2"), "{}", run.stdout);
    }
    Cli::on(&project)
        .run(&["files", "stat", &at(&fine)])
        .exits(0);

    let listed = Cli::on(&project)
        .run(&["grants", "--format", "json"])
        .exits(0)
        .json();
    let rows = listed["roots"].as_array().unwrap();
    assert_eq!(rows.len(), 8, "{listed}");
    for row in rows {
        let reached = row["name"] == "fine";
        assert_eq!(row["approved"], reached, "{row}");
    }
    let warned = listed["diagnostics"].to_string();
    for name in [
        "everything",
        "home",
        "state",
        "gh",
        "system",
        "spec",
        "Bad_Name",
    ] {
        assert!(warned.contains(&format!("`{name}`")), "{name}: {warned}");
    }
    assert!(warned.contains("W186"), "{warned}");
    assert!(!warned.contains("`fine`"), "{warned}");
}

/// A store that is a link could be a repository's file: in every mode it approves nothing, a loose directory included,
/// and neither does one reached through a `.specforge` that is a link.
#[cfg(unix)]
#[test]
fn a_consent_store_that_is_a_link_approves_nothing() {
    let (project, target) = approved();
    let home = project.home().canonicalize().unwrap();
    let dir = home.join(".specforge");
    let store = dir.join("consent.json");
    let loose = tempfile::tempdir().unwrap();
    let real = loose.path().join("committed.json");
    std::fs::rename(&store, &real).unwrap();
    std::os::unix::fs::symlink(&real, &store).unwrap();
    let path = at(&target);
    let refused = |run: crate::support::Run| {
        assert_eq!(run.exits(1).json()["code"], "outside_root");
    };
    refused(Cli::on(&project).run(&["files", "stat", &path]));
    refused(
        Cli::on(&project)
            .in_dir(loose.path())
            .bare(&["files", "stat", &path]),
    );
    let listed = Cli::on(&project)
        .run(&["grants", "--format", "json"])
        .exits(0)
        .json();
    assert!(
        listed["diagnostics"].to_string().contains("symbolic link"),
        "{listed}"
    );
    let approve = Cli::on(&project).run(&["grants", "revoke", "--root", "workspaces", "--machine"]);
    assert_ne!(approve.code, 0, "{}", approve.stdout);
    assert!(store.is_symlink(), "a refusal writes nothing over the link");

    std::fs::remove_file(&store).unwrap();
    std::fs::rename(&dir, loose.path().join("state")).unwrap();
    std::fs::rename(&real, loose.path().join("state/consent.json")).unwrap();
    std::os::unix::fs::symlink(loose.path().join("state"), &dir).unwrap();
    refused(Cli::on(&project).run(&["files", "stat", &path]));
}

/// A machine root is for `files` and `git`: no program runs in it through `--in`, and `http` sends no file out of it.
#[test]
fn only_files_and_git_reach_a_machine_root() {
    let (project, target) = approved();
    let dir = target.join("ws");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("body"), "TOKEN=hunter2\n").unwrap();
    let ran = dir.join("ran");
    let run = Cli::on(&project).run(&[
        "run",
        "--in",
        &at(&dir),
        "--",
        "/bin/sh",
        "-c",
        &format!("echo x > {}", at(&ran)),
    ]);
    assert_ne!(run.code, 0, "{}", run.stdout);
    assert!(run.stderr.contains("outside_root"), "{}", run.stderr);
    assert!(run.stderr.contains("specforge files"), "{}", run.stderr);
    assert!(!ran.exists(), "a program ran in the machine root");
    let sent = Cli::on(&project).run(&[
        "http",
        "post",
        "http://127.0.0.1:9/",
        "--body-file",
        &at(&dir.join("body")),
    ]);
    assert_ne!(sent.code, 0, "{}", sent.stdout);
    let said = format!("{}{}", sent.stdout, sent.stderr);
    assert!(said.contains("outside_root"), "{said}");
    assert!(!said.contains("hunter2"), "{said}");

    // The project's own directory is as it was, and so is a machine root the project also holds.
    Cli::on(&project)
        .run(&["run", "--in", ".", "--", "/bin/sh", "-c", "true"])
        .exits(0);
    let inside = TestProject::new().files(&[("mine/a.txt", "a\n")]);
    let moved = inside.home().canonicalize().unwrap().join("proj");
    std::fs::rename(inside.root(), &moved).unwrap();
    let typed = |args: &[&str]| Cli::on(&inside).in_dir(&moved).bare(args);
    typed(&["grants", "approve", "--root", "mine", "--at", "mine"]).exits(0);
    typed(&["run", "--in", "mine", "--", "/bin/sh", "-c", "true"]).exits(0);
}
