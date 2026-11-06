//! `specforge grants`: the listing, `approve` and `revoke` over the user's consent store (the project's own HOME
//! here), and `collect` without a terminal running once approved. The approvals themselves are tested in process
//! (`specforge-ops`, `grants::tests`, `consent::tests`); these tests are the CLI's wiring.

use crate::support::{Cli, TestProject};
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

const EXTENSION: &str = "@specforge/cargo-test";

/// A Rust project collected by `@specforge/cargo-test`, with a fake `cargo` on `PATH` that writes a report and
/// leaves a marker, and a directory outside the project that `specforge.json` may name as a root.
struct Fixture {
    project: TestProject,
    tools: TempDir,
}

impl Fixture {
    fn new() -> Self {
        Self::with_roots("")
    }

    /// `roots` is the `"roots"` member of `specforge.json` with its leading comma, or nothing.
    fn with_roots(roots: &str) -> Self {
        let tools = TempDir::new().unwrap();
        let bin = tools.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let cargo = bin.join("cargo");
        std::fs::write(
            &cargo,
            r#"#!/bin/sh
touch "$(dirname "$0")/ran"
mkdir -p "$SPECFORGE_REPORT"
cat > "$SPECFORGE_REPORT/demo.json" <<'EOF'
{"entries":[{"entity_id":"login","test_name":"accepts","verify":"accepts valid credentials","status":"passed"}]}
EOF
exit 0
"#,
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let project = TestProject::new()
            .raw_config(format!(
                r#"{{"name":"demo","version":"0.1.0","extensions":["@specforge/software","@specforge/testing","@specforge/cargo-test"]{roots}}}"#
            ))
            .file(
                "app.spec",
                "behavior login \"Login\" {\n  verify unit \"accepts valid credentials\"\n}\n",
            )
            .file("Cargo.toml", "");
        Fixture { project, tools }
    }

    fn ran(&self) -> bool {
        self.tools.path().join("bin/ran").exists()
    }

    fn outside(&self) -> std::path::PathBuf {
        self.tools.path().canonicalize().unwrap()
    }

    /// `specforge <args>` with the fake `cargo` first on `PATH` and stdin closed: nobody to ask.
    fn cmd(&self, args: &[&str]) -> Command {
        let path = format!(
            "{}:{}",
            self.tools.path().join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let mut cmd = Cli::on(&self.project).env("PATH", path).command(args);
        cmd.write_stdin("");
        cmd
    }

    fn json(&self, args: &[&str]) -> serde_json::Value {
        let out = self
            .cmd(args)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        serde_json::from_slice(&out).unwrap()
    }
}

#[test]
fn grants_lists_a_collectors_command_as_in_use_and_not_approved() {
    let fx = Fixture::new();
    fx.cmd(&["grants"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("{EXTENSION} (builtin)")))
        .stdout(predicate::str::contains(
            "cargo-test  process cargo test --workspace --no-fail-fast: not approved, in use",
        ))
        .stderr(predicate::str::contains(format!(
            "warning[W170]: extension '{EXTENSION}': effects not approved on this machine"
        )))
        .stderr(predicate::str::contains(format!(
            "specforge grants approve {EXTENSION} cargo-test"
        )));
    let listing = fx.json(&["grants", "list", "--format", "json"]);
    let extension = listing["extensions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["extension"] == EXTENSION)
        .expect("cargo-test is listed");
    assert_eq!(extension["provenance"], "builtin");
    let row = &extension["capabilities"][0];
    assert_eq!(row["name"], "cargo-test");
    assert_eq!(row["port"], "process");
    assert_eq!(row["state"], "not-approved");
    assert_eq!(row["in_use"], true);
}

/// A store committed at `.specforge/consent.json` of a project whose `HOME` is the project itself (`WORKDIR=$HOME`, a
/// Heroku-style dyno) approves nothing: the collector does not run, and W186 says why.
#[cfg(unix)]
#[test]
fn a_store_committed_in_a_project_that_is_the_home_approves_nothing() {
    let fx = Fixture::new();
    fx.cmd(&["grants", "approve", EXTENSION]).assert().success();
    let committed = fx.project.root().join(".specforge");
    std::fs::create_dir_all(&committed).unwrap();
    std::fs::copy(
        fx.project.home().join(".specforge/consent.json"),
        committed.join("consent.json"),
    )
    .unwrap();

    let path = format!(
        "{}:{}",
        fx.tools.path().join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let mut collect = Cli::on(&fx.project)
        .env("PATH", path)
        .at_home(fx.project.root())
        .command(&["collect"]);
    collect
        .write_stdin("")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error[E059]"))
        .stderr(predicate::str::contains("inside the project"));
    assert!(!fx.ran(), "the repository approved its own command");
}

/// A root's target is the project's text and no extension's: a directory name carries an escape sequence to every
/// place that shows the approval it needs, and none of them passes it to the terminal or to an agent raw.
#[cfg(unix)]
#[test]
fn a_root_outside_the_project_is_shown_printable_wherever_it_is_listed() {
    let outside = TempDir::new().unwrap();
    let hostile = outside.path().join("v\u{1b}[2K\u{202e}");
    std::fs::create_dir(&hostile).unwrap();
    let fx = Fixture::with_roots(&format!(
        r#","roots":{{"vault":{}}}"#,
        serde_json::to_string(hostile.to_str().unwrap()).unwrap()
    ));
    let listed = |args: &[&str]| {
        let out = fx.cmd(args).output().unwrap();
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(text.contains("vault"), "{args:?}: {text}");
        for raw in ['\u{1b}', '\u{202e}'] {
            assert!(!text.contains(raw), "{args:?}: {raw:?} in {text:?}");
        }
        text
    };
    assert!(listed(&["grants"]).contains("\\u{1b}[2K"));
    assert!(listed(&["doctor"]).contains("\\u{1b}[2K"));
    // JSON would escape the ESC by itself, and never the bidirectional control.
    listed(&["grants", "--format", "json"]);
    listed(&["doctor", "--format", "json"]);
}

#[cfg(unix)]
#[test]
fn an_approved_collector_runs_without_a_terminal_until_revoked() {
    let fx = Fixture::new();
    fx.cmd(&["collect"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error[E059]"));
    assert!(!fx.ran());

    fx.cmd(&["grants", "approve", EXTENSION])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "Approved {EXTENSION} cargo-test"
        )));
    fx.cmd(&["grants"])
        .assert()
        .success()
        .stdout(predicate::str::contains(": approved 2"));
    fx.cmd(&["collect"]).assert().success();
    assert!(fx.ran(), "an approved command runs with nobody to ask");

    let store = fx.json(&["grants", "--store", "--format", "json"]);
    assert_eq!(store["capabilities"][0]["extension"], EXTENSION);
    assert_eq!(store["capabilities"][0]["scope"], "cargo-test");

    fx.cmd(&["grants", "revoke", EXTENSION, "cargo-test"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "Revoked {EXTENSION} cargo-test"
        )));
    fx.cmd(&["collect"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error[E059]"));
    fx.cmd(&["grants", "revoke", "--project"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Nothing to revoke."));
}

#[test]
fn a_root_outside_the_project_is_approved_and_revoked_by_name() {
    let outside = TempDir::new().unwrap();
    let target = outside.path().canonicalize().unwrap();
    let fx = Fixture::with_roots(&format!(
        r#","roots":{{"vault":{}}}"#,
        serde_json::to_string(&target.display().to_string()).unwrap()
    ));
    assert_ne!(fx.outside(), target);
    let roots = |fx: &Fixture| fx.json(&["grants", "--format", "json"])["roots"][0].clone();
    assert_eq!(roots(&fx)["name"], "vault");
    assert_eq!(roots(&fx)["inside"], false);
    assert_eq!(roots(&fx)["approved"], false);

    fx.cmd(&["grants", "approve", "--root", "vault"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Approved root vault"));
    assert_eq!(roots(&fx)["approved"], true);

    fx.cmd(&["grants", "revoke", "--root", "vault"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Revoked root vault"));
    assert_eq!(roots(&fx)["approved"], false);
}

#[test]
fn unknown_extensions_scopes_and_roots_are_refused() {
    let fx = Fixture::new();
    fx.cmd(&["grants", "approve", "@nobody/none"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "'@nobody/none' is not a loaded extension that declares capabilities",
        ));
    fx.cmd(&["grants", "approve", EXTENSION, "no-such-scope"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(format!(
            "{EXTENSION} declares no capability `no-such-scope`"
        )));
    fx.cmd(&["grants", "revoke", "--root", "nowhere"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "specforge.json names no root `nowhere`",
        ));
    let refused = fx
        .cmd(&["grants", "approve", "--root", "nowhere", "--format", "json"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let refused: serde_json::Value = serde_json::from_slice(&refused).unwrap();
    assert_eq!(refused["code"], "invalid_input");
}

#[test]
fn doctor_and_extensions_list_a_collectors_capability() {
    let fx = Fixture::new();
    fx.cmd(&["doctor"])
        .assert()
        .stdout(predicate::str::contains(
            "      may cargo-test  process cargo test --workspace --no-fail-fast: not approved, in use",
        ))
        .stdout(predicate::str::contains("Grants:"))
        .stdout(predicate::str::contains("[W170]"));
    fx.cmd(&["extensions"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "      may cargo-test: process cargo test --workspace --no-fail-fast",
        ));
    let doctor = fx.json(&["doctor", "--format", "json"]);
    let row = doctor["extensions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["name"] == EXTENSION)
        .expect("cargo-test is listed")["capabilities"][0]
        .clone();
    assert_eq!(row["state"], "not-approved");
}

/// `specforge <args>`, typed in `dir` (no `--path`), with `project`'s home.
fn typed_in(project: &TestProject, dir: &std::path::Path, args: &[&str]) -> crate::support::Run {
    Cli::on(project).in_dir(dir).bare(args)
}

const APPROVE: [&str; 5] = ["grants", "approve", "--root", "workspaces", "--at"];

#[test]
fn a_machine_root_is_approved_once_and_listed_from_any_project() {
    let fx = Fixture::new();
    let home = fx.project.home().canonicalize().unwrap();
    let target = home.join("gsd-workspaces");
    assert!(!target.exists());
    let approved = fx
        .cmd(&[&APPROVE[..], &["~/gsd-workspaces"]].concat())
        .assert()
        .success();
    let text = String::from_utf8_lossy(&approved.get_output().stdout).into_owned();
    assert!(text.contains("Approved root workspaces"), "{text}");
    assert!(target.is_dir(), "the directory is made");

    // Another project shares the home by its store; a directory with no specforge.json lists it too, and so does the
    // home itself, which no project owns.
    let loose = TempDir::new().unwrap();
    let elsewhere = fx.json(&["grants", "--format", "json"]);
    for listing in [
        elsewhere,
        typed_in(&fx.project, loose.path(), &["grants", "--format", "json"])
            .exits(0)
            .json(),
        typed_in(&fx.project, &home, &["grants", "--format", "json"])
            .exits(0)
            .json(),
    ] {
        let row = &listing["roots"][0];
        assert_eq!(row["name"], "workspaces", "{listing}");
        assert_eq!(row["target"], target.display().to_string());
        assert_eq!(row["inside"], false);
        assert_eq!(row["approved"], true);
        assert_eq!(row["machine"], true);
    }
    let store = fx.json(&["grants", "--store", "--format", "json"]);
    assert_eq!(store["machine_roots"][0]["root"], "workspaces", "{store}");
    assert_eq!(
        store["machine_roots"][0]["target"],
        target.display().to_string()
    );
    assert!(
        !store["machine_roots"][0]["approved_at"]
            .as_str()
            .unwrap()
            .is_empty()
    );

    // The home directory is a natural place to type it: the store in it is still the user's own.
    typed_in(
        &fx.project,
        &home,
        &[
            "grants",
            "approve",
            "--root",
            "second",
            "--at",
            "gsd-second",
        ],
    )
    .exits(0);
    assert!(home.join("gsd-second").is_dir());

    fx.cmd(&["grants"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "(machine), every project's verbs reach it",
        ));
}

#[test]
fn a_machine_root_that_is_or_holds_the_home_is_refused() {
    let fx = Fixture::new();
    let home = fx.project.home().canonicalize().unwrap();
    let outside = TempDir::new().unwrap();
    let file = outside.path().join("a-file");
    std::fs::write(&file, "").unwrap();
    let at_home = outside.path().join("at-home");
    let dangling = outside.path().join("dangling");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&home, &at_home).unwrap();
        std::os::unix::fs::symlink(outside.path().join("gone"), &dangling).unwrap();
    }
    let parent = home.parent().unwrap().display().to_string();
    for at in [
        "~",
        "~/",
        "~/.",
        "~/x/..",
        "~/x/../..",
        "/",
        parent.as_str(),
        "~other/x",
        "~other",
        "~/.specforge",
        "~/.specforge/roots",
        "~/.ssh",
        file.to_str().unwrap(),
        at_home.to_str().unwrap(),
        dangling.to_str().unwrap(),
    ] {
        let run = fx.cmd(&[&APPROVE[..], &[at]].concat()).assert().failure();
        let out = run.get_output();
        assert!(!out.stderr.is_empty(), "{at}: no refusal");
        let store = fx.json(&["grants", "--store", "--format", "json"]);
        assert_eq!(
            store["machine_roots"],
            serde_json::json!([]),
            "{at} was stored"
        );
    }
    assert!(!home.join("x").exists(), "a refusal makes nothing");
    assert!(!home.join(".ssh").exists(), "a refusal makes nothing");
    assert!(!outside.path().join("gone").exists());

    let target = outside.path().join("fine");
    for name in ["spec", "project", "temp", "Bad_Name", "../up", "a/b", ""] {
        fx.cmd(&[
            "grants",
            "approve",
            "--root",
            name,
            "--at",
            target.to_str().unwrap(),
        ])
        .assert()
        .failure();
    }
    assert!(!target.exists(), "a refused name makes nothing");

    // A flag out of place is a usage error, not a project root.
    fx.cmd(&["grants", "approve", "--at", target.to_str().unwrap()])
        .assert()
        .failure();
    fx.cmd(&["grants", "revoke", "--machine"])
        .assert()
        .failure();
    assert!(!target.exists());
}

/// A machine root reaches every project's verbs, so one at a credential directory would print its tokens: `files read
/// ~/.config/gh/hosts.yml`. Judged after links, before anything is made.
#[test]
fn a_machine_root_is_no_hidden_directory_of_the_home_and_no_system_directory() {
    let fx = Fixture::new();
    let home = fx.project.home().canonicalize().unwrap();
    std::fs::create_dir_all(home.join(".config/gh")).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(home.join(".config/gh"), home.join("shortcut")).unwrap();
    let mut refused = vec![
        "~/.config",
        "~/.config/gh",
        "~/.claude",
        "~/.gnupg",
        "~/.aws",
        "~/.aws/deep/er",
        "~/Library",
        "~/Library/Keychains",
        "~/AppData/Roaming",
    ];
    #[cfg(unix)]
    refused.extend([
        "~/shortcut",
        "~/shortcut/sub",
        "/etc",
        "/etc/specforge-refused",
        "/usr",
        "/usr/local/specforge-refused",
        "/bin",
        "/sbin",
        "/var",
        "/var/specforge-refused",
        "/private",
        "/private/specforge-refused",
        "/System",
        "/Library",
        "/Library/specforge-refused",
        "/opt",
        "/opt/specforge-refused",
        "/proc",
        "/sys",
        "/dev",
    ]);
    #[cfg(windows)]
    refused.extend([
        "C:\\Windows",
        "C:\\Windows\\System32",
        "C:\\Program Files",
        "C:\\Program Files (x86)\\x",
        "C:\\ProgramData",
    ]);
    for at in refused {
        let run = fx.cmd(&[&APPROVE[..], &[at]].concat()).assert().failure();
        let stderr = String::from_utf8_lossy(&run.get_output().stderr).into_owned();
        assert!(stderr.contains("directory"), "{at}: {stderr}");
        let store = fx.json(&["grants", "--store", "--format", "json"]);
        assert_eq!(store["machine_roots"], serde_json::json!([]), "{at} stored");
    }
    assert!(!home.join(".claude").exists(), "a refusal makes nothing");
    assert!(!home.join(".config/gh/sub").exists());

    // What only looks like one, and a home that itself lies where the system's directories are.
    for (at, made) in [
        ("~/work.d/.cache", "work.d/.cache"),
        ("~/etc", "etc"),
        ("~/gsd-workspaces", "gsd-workspaces"),
    ] {
        fx.cmd(&[&APPROVE[..], &[at]].concat()).assert().success();
        assert!(home.join(made).is_dir(), "{at}");
    }
}

/// A root can't be made over the store that holds it: a block could then write its own approvals through a verb.
#[test]
fn a_machine_root_never_reaches_the_consent_store() {
    let fx = Fixture::new();
    let dir_path = fx
        .project
        .home()
        .canonicalize()
        .unwrap()
        .join("beside-store");
    std::fs::create_dir(&dir_path).unwrap();
    let store = dir_path.join("store/consent.json");
    let approve = |at: std::path::PathBuf| {
        Cli::on(&fx.project)
            .env("SPECFORGE_CONSENT_FILE", &store)
            .run(&[&APPROVE[..], &[at.to_str().unwrap()]].concat())
    };
    for at in [
        dir_path.join("store"),
        dir_path.clone(),
        dir_path.join("store/inside"),
    ] {
        let run = approve(at.clone());
        assert_ne!(run.code, 0, "{}", at.display());
        assert!(run.stderr.contains("consent store"), "{}", run.stderr);
    }
    approve(dir_path.join("beside")).exits(0);
}

#[test]
fn a_machine_root_is_revoked_by_name() {
    let fx = Fixture::new();
    fx.cmd(&["grants", "revoke", "--root", "workspaces", "--machine"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "this machine names no root `workspaces`",
        ));
    fx.cmd(&[&APPROVE[..], &["~/gsd-workspaces"]].concat())
        .assert()
        .success();
    // Revoking the project's approvals keeps it: no project holds it.
    fx.cmd(&["grants", "revoke", "--project"])
        .assert()
        .success();
    assert_eq!(
        fx.json(&["grants", "--format", "json"])["roots"][0]["machine"],
        true
    );
    // `--root` alone is the project's root, whose name this is not.
    fx.cmd(&["grants", "revoke", "--root", "workspaces"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("specforge.json names no root"));
    fx.cmd(&["grants", "revoke", "--root", "workspaces", "--machine"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Revoked root workspaces"));
    assert_eq!(
        fx.json(&["grants", "--format", "json"])["roots"],
        serde_json::json!([])
    );
    assert!(
        fx.project.home().join("gsd-workspaces").is_dir(),
        "revoking leaves the directory"
    );
}
