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
