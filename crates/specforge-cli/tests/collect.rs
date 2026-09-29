use assert_cmd::Command;
use predicates::prelude::*;
use specforge_test_macros::test as specforge_test;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[allow(deprecated)]
fn specforge_cmd() -> Command {
    Command::cargo_bin("specforge").unwrap()
}

/// A Rust project collected by `@specforge/cargo-test`, a fake `cargo` on
/// `PATH` that writes a report the way `specforge-test` does (and leaves a
/// marker so tests can tell whether it ran), and a private consent store.
struct Fixture {
    dir: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("proj");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("specforge.json"),
            r#"{"name":"demo","version":"0.1.0","extensions":["@specforge/software","@specforge/testing","@specforge/cargo-test"]}"#,
        )
        .unwrap();
        std::fs::write(
            root.join("app.spec"),
            "behavior login \"Login\" {\n  verify unit \"accepts valid credentials\"\n}\n",
        )
        .unwrap();
        std::fs::write(root.join("Cargo.toml"), "").unwrap();

        let bin = dir.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let cargo = bin.join("cargo");
        std::fs::write(
            &cargo,
            r#"#!/bin/sh
touch "$(dirname "$0")/ran"
[ -n "$NO_REPORT" ] && exit 101
mkdir -p "$SPECFORGE_REPORT"
cat > "$SPECFORGE_REPORT/demo.json" <<'EOF'
{"entries":[{"entity_id":"login","test_name":"accepts","verify":"accepts valid credentials","status":"pass"}]}
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
        Fixture { dir }
    }

    fn root(&self) -> PathBuf {
        self.dir.path().join("proj")
    }

    fn ran(&self) -> bool {
        self.dir.path().join("bin/ran").exists()
    }

    /// `specforge <args>` in the fixture's environment, with stdin closed.
    fn cmd(&self, args: &[&str]) -> Command {
        let path = format!(
            "{}:{}",
            self.dir.path().join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let mut cmd = specforge_cmd();
        cmd.args(args)
            .arg("--path")
            .arg(self.root())
            .env("PATH", path)
            .env(
                "SPECFORGE_CONSENT_FILE",
                self.dir.path().join("consent.json"),
            )
            .write_stdin("");
        cmd
    }

    fn report(&self) -> serde_json::Value {
        let raw = std::fs::read_to_string(self.root().join("specforge-report.json")).unwrap();
        serde_json::from_str(&raw).unwrap()
    }
}

fn write_report(root: &Path, entries: &str) {
    let dir = root.join("target/specforge");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("old.json"), format!(r#"{{"entries":{entries}}}"#)).unwrap();
}

#[test]
fn test_collect_help_exits_0() {
    specforge_cmd()
        .args(["collect", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--no-run"))
        .stdout(predicate::str::contains("--yes"));
}

#[test]
fn test_collect_in_empty_dir_exits_1() {
    let dir = TempDir::new().unwrap();
    specforge_cmd()
        .args(["collect", "--path", dir.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no specforge project found"));
}

#[test]
fn test_collect_json_format_in_empty_dir() {
    let dir = TempDir::new().unwrap();
    specforge_cmd()
        .args([
            "collect",
            "--path",
            dir.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"error\""));
}

#[test]
fn collect_without_a_runner_extension_is_e058() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"test","version":"0.1.0"}"#,
    )
    .unwrap();
    specforge_cmd()
        .args(["collect", "--path", dir.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error[E058]"))
        .stderr(predicate::str::contains(
            "specforge add @specforge/cargo-test",
        ));
}

#[specforge_test(
    behavior = "auto_detect_collector",
    verify = "no match emits E058 with available collectors"
)]
fn collect_without_detection_files_is_e058() {
    let fx = Fixture::new();
    std::fs::write(
        fx.root().join("specforge.json"),
        r#"{"name":"demo","version":"0.1.0","extensions":["@specforge/software","@specforge/testing","@specforge/cargo-test","@specforge/vitest"]}"#,
    )
    .unwrap();
    std::fs::remove_file(fx.root().join("Cargo.toml")).unwrap();
    fx.cmd(&["collect", "--yes"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no test runner detected"))
        .stderr(predicate::str::contains("available: cargo-test, vitest"));
    assert!(!fx.ran());
}

#[specforge_test(
    behavior = "auto_detect_collector",
    verify = "a single enabled collector is used without detection"
)]
fn a_single_enabled_runner_needs_no_detection() {
    let fx = Fixture::new();
    std::fs::remove_file(fx.root().join("Cargo.toml")).unwrap();
    fx.cmd(&["collect", "--yes"]).assert().success();
    assert!(fx.ran());
}

#[specforge_test(
    behavior = "approve_collector_command",
    verify = "unapproved command without a terminal fails with E059 and runs nothing"
)]
fn unapproved_command_without_a_terminal_is_refused() {
    let fx = Fixture::new();
    fx.cmd(&["collect"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error[E059]"))
        .stderr(predicate::str::contains(
            "cargo test --workspace --no-fail-fast",
        ));
    assert!(!fx.ran(), "the command must not run without approval");
    assert!(!fx.root().join("specforge-report.json").exists());
}

#[specforge_test(
    behavior = "approve_collector_command",
    verify = "--yes runs the declared command"
)]
fn yes_runs_the_declared_command() {
    let fx = Fixture::new();
    // A stale report from an earlier run must not survive the new one.
    write_report(
        &fx.root(),
        r#"[{"entity_id":"login","test_name":"old","status":"fail"}]"#,
    );
    fx.cmd(&["collect", "--yes"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "running cargo-test (@specforge/cargo-test): cargo test --workspace --no-fail-fast",
        ))
        .stdout(predicate::str::contains(
            "cargo-test: 1 entities, 1 passed, 0 failed, 0 skipped",
        ));
    assert!(fx.ran());
    let tests = &fx.report()["results"]["login"]["tests"];
    assert_eq!(tests.as_array().unwrap().len(), 1);
    assert_eq!(tests[0]["name"], "accepts");
    assert_eq!(tests[0]["verify"], "accepts valid credentials");
    assert_eq!(tests[0]["runner"], "cargo-test");
}

#[test]
fn runner_that_writes_no_report_is_e045() {
    let fx = Fixture::new();
    fx.cmd(&["collect", "--yes"])
        .env("NO_REPORT", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error[E045]"))
        .stderr(predicate::str::contains("produced no report"));
}

#[test]
fn no_run_parses_the_existing_report_without_running() {
    let fx = Fixture::new();
    write_report(
        &fx.root(),
        r#"[{"entity_id":"login","test_name":"a","status":"pass"},{"entity_id":"ghost","test_name":"b","status":"pass"}]"#,
    );
    let out = fx
        .cmd(&["collect", "--no-run", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert!(!fx.ran());
    let json: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(json["runners"][0]["ran"], false);
    assert_eq!(json["runners"][0]["passed"], 1);
    assert_eq!(json["diagnostics"][0]["code"], "W115");

    // Without a report, --no-run says how to get one.
    std::fs::remove_dir_all(fx.root().join("target")).unwrap();
    fx.cmd(&["collect", "--no-run"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("run without --no-run"));
}

#[test]
fn explicit_report_files_are_parsed() {
    let fx = Fixture::new();
    let report = fx.dir.path().join("elsewhere.json");
    std::fs::write(
        &report,
        r#"{"entries":[{"entity_id":"login","test_name":"a","status":"fail"}]}"#,
    )
    .unwrap();
    fx.cmd(&[
        "collect",
        "--runner",
        "cargo-test",
        &format!("--report={}", report.display()),
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("0 passed, 1 failed"));
    assert!(!fx.ran());
}

#[specforge_test(
    behavior = "ingest_collector_report",
    verify = "collect then analyze scores the recorded tests"
)]
fn collect_then_analyze_scores_the_recorded_tests() {
    let fx = Fixture::new();
    fx.cmd(&["collect", "--yes"]).assert().success();
    let out = specforge_cmd()
        .args(["analyze", "coverage", "--json", "--path"])
        .arg(fx.root())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(
        text.contains(r#""entities_proven": 1"#) || text.contains(r#""entities_proven":1"#),
        "analyze should read specforge-report.json by default: {text}"
    );
}

#[specforge_test(
    behavior = "vt_declare_vitest_collector",
    verify = "collect runs vitest with the report path and maps linked tests"
)]
fn vitest_runs_with_the_report_path_and_maps_linked_tests() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().join("web");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("specforge.json"),
        r#"{"name":"web","version":"0.1.0","extensions":["@specforge/software","@specforge/testing","@specforge/vitest"]}"#,
    )
    .unwrap();
    std::fs::write(
        root.join("app.spec"),
        "behavior login \"Login\" {\n  verify unit \"accepts valid credentials\"\n}\n",
    )
    .unwrap();
    std::fs::write(root.join("vitest.config.ts"), "export default {}\n").unwrap();

    // A fake `npx` that records its arguments and writes a vitest JSON
    // report to the `--outputFile.json=` path the collector declared.
    let bin = dir.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let npx = bin.join("npx");
    std::fs::write(
        &npx,
        r#"#!/bin/sh
echo "$@" > "$(dirname "$0")/args"
for arg in "$@"; do
  case "$arg" in --outputFile.json=*) out="${arg#--outputFile.json=}" ;; esac
done
mkdir -p "$(dirname "$out")"
cat > "$out" <<'REPORT'
{"testResults":[{"assertionResults":[
 {"fullName":"login accepts valid credentials","status":"passed","duration":3,
  "meta":{"specforge":{"behavior":"login","verify":"accepts valid credentials"}}},
 {"fullName":"unlinked","status":"passed","meta":{}}]}]}
REPORT
exit 0
"#,
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&npx, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    specforge_cmd()
        .args(["collect", "--yes", "--path"])
        .arg(&root)
        .env("PATH", path)
        .env("SPECFORGE_CONSENT_FILE", dir.path().join("consent.json"))
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "vitest: 1 entities, 1 passed, 0 failed, 0 skipped",
        ));

    let args = std::fs::read_to_string(bin.join("args")).unwrap();
    assert!(
        args.starts_with("--no vitest run"),
        "never downloads vitest: {args}"
    );
    assert!(
        args.contains(".specforge/reports/vitest.json"),
        "the report path is expanded: {args}"
    );
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("specforge-report.json")).unwrap())
            .unwrap();
    let test = &report["results"]["login"]["tests"][0];
    assert_eq!(test["verify"], "accepts valid credentials");
    assert_eq!(test["runner"], "vitest");
}
