// `specforge watch` — end-to-end smoke test against the real binary.

use std::fs;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use assert_cmd::cargo::CommandCargoExt;
use specforge_test::prelude::*;
use tempfile::TempDir;

#[allow(deprecated)]
fn specforge_cmd() -> Command {
    Command::cargo_bin("specforge").unwrap()
}

/// Spawn `specforge watch --json` on a fresh project and return the line
/// receiver plus the child handle.
fn spawn_watch(project: &TempDir) -> (mpsc::Receiver<String>, std::process::Child) {
    spawn_watch_with(project, &[])
}

fn spawn_watch_with(
    project: &TempDir,
    extra: &[&str],
) -> (mpsc::Receiver<String>, std::process::Child) {
    let mut child = specforge_cmd()
        .args([
            "watch",
            "--path",
            project.path().to_str().unwrap(),
            "--json",
        ])
        .args(extra)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn specforge watch");

    let stdout = child.stdout.take().expect("stdout piped");
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                return;
            }
        }
    });
    (rx, child)
}

/// Wait for a line matching `needle` within `timeout`.
fn wait_for_line(rx: &mpsc::Receiver<String>, needle: &str, timeout: Duration) -> Option<String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(line) => {
                if line.contains(needle) {
                    return Some(line);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => return None,
        }
    }
    None
}

#[test]
fn watch_rebuilds_on_file_change() {
    let project = TempDir::new().unwrap();
    fs::write(project.path().join("specforge.json"), "{}").unwrap();
    fs::write(
        project.path().join("main.spec"),
        "entity one { title \"One\" }\n",
    )
    .unwrap();

    let (rx, mut child) = spawn_watch(&project);

    let ready = wait_for_line(&rx, "\"event\":\"ready\"", Duration::from_secs(60));
    assert!(ready.is_some(), "watch never reported ready");

    // Give the watcher a moment to settle, then grow the spec file.
    std::thread::sleep(Duration::from_millis(300));
    fs::write(
        project.path().join("main.spec"),
        "entity one { title \"One\" }\nentity two { title \"Two\" }\n",
    )
    .unwrap();

    let rebuilt = wait_for_line(&rx, "\"event\":\"rebuilt\"", Duration::from_secs(60));
    let _ = child.kill();
    let _ = child.wait();

    let line = rebuilt.expect("no rebuild event after file change");
    assert!(
        line.contains("\"added_nodes\":1"),
        "expected exactly one added node, got: {line}"
    );
    assert!(
        line.contains("\"errors\":0"),
        "clean edit must stay diagnostic-free: {line}"
    );
}

#[test]
fn watch_reports_diagnostics_on_broken_edit() {
    let project = TempDir::new().unwrap();
    fs::write(project.path().join("specforge.json"), "{}").unwrap();
    fs::write(
        project.path().join("main.spec"),
        "entity one { title \"One\" }\n",
    )
    .unwrap();

    let (rx, mut child) = spawn_watch(&project);
    let ready = wait_for_line(&rx, "\"event\":\"ready\"", Duration::from_secs(60));
    assert!(ready.is_some(), "watch never reported ready");

    std::thread::sleep(Duration::from_millis(300));
    // Reference a nonexistent entity -> E003 unresolved reference.
    fs::write(
        project.path().join("main.spec"),
        "entity one { title \"One\"\n  uses [ghost]\n}\n",
    )
    .unwrap();

    let rebuilt = wait_for_line(&rx, "\"event\":\"rebuilt\"", Duration::from_secs(60));
    let _ = child.kill();
    let _ = child.wait();

    let line = rebuilt.expect("no rebuild event after file change");
    assert!(
        line.contains("\"errors\":1"),
        "broken edit must surface one error: {line}"
    );
    assert!(
        line.contains("main.spec"),
        "changed diagnostic files must include the edited file: {line}"
    );
}

#[test]
fn watch_verify_incremental_checks_each_rebuild_against_a_cold_one() {
    let project = TempDir::new().unwrap();
    fs::write(project.path().join("specforge.json"), "{}").unwrap();
    fs::write(
        project.path().join("main.spec"),
        "entity one { title \"One\" }\n",
    )
    .unwrap();

    let (rx, mut child) = spawn_watch_with(&project, &["--verify-incremental"]);
    assert!(
        wait_for_line(&rx, "\"event\":\"ready\"", Duration::from_secs(60)).is_some(),
        "watch never reported ready"
    );
    std::thread::sleep(Duration::from_millis(300));
    fs::write(
        project.path().join("main.spec"),
        "entity one { title \"One\" }\nentity two { title \"Two\" }\n",
    )
    .unwrap();

    let rebuilt = wait_for_line(&rx, "\"event\":\"rebuilt\"", Duration::from_secs(60));
    let _ = child.kill();
    let _ = child.wait();

    let line = rebuilt.expect("no rebuild event after file change");
    let event: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(event["verification"], "passed", "{line}");
}

// The debug build of the compiler checks every rebuild; a release build only
// with --verify-incremental. The binary is built in the test's profile, so
// this test exists only in debug builds.
#[cfg(debug_assertions)]
#[specforge_test(
    behavior = "validate_delta_correctness",
    verify = "a debug build checks each rebuild without the flag"
)]
fn watch_debug_build_verifies_without_the_flag() {
    let project = TempDir::new().unwrap();
    fs::write(project.path().join("specforge.json"), "{}").unwrap();
    fs::write(
        project.path().join("main.spec"),
        "entity one { title \"One\" }\n",
    )
    .unwrap();

    let (rx, mut child) = spawn_watch(&project);
    assert!(
        wait_for_line(&rx, "\"event\":\"ready\"", Duration::from_secs(60)).is_some(),
        "watch never reported ready"
    );
    std::thread::sleep(Duration::from_millis(300));
    fs::write(
        project.path().join("main.spec"),
        "entity one { title \"One\" }\nentity two { title \"Two\" }\n",
    )
    .unwrap();

    let rebuilt = wait_for_line(&rx, "\"event\":\"rebuilt\"", Duration::from_secs(60));
    let _ = child.kill();
    let _ = child.wait();

    let line = rebuilt.expect("no rebuild event after file change");
    let event: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(event["verification"], "passed", "{line}");
}

#[test]
fn watch_rebuild_reports_what_check_reports() {
    let project = TempDir::new().unwrap();
    fs::write(
        project.path().join("specforge.json"),
        r#"{"name":"w","version":"0.1.0","extensions":["@specforge/software","@specforge/testing"]}"#,
    )
    .unwrap();
    fs::write(
        project.path().join("main.spec"),
        "behavior login \"L\" {\n  category command\n  contract \"c\"\n  verify unit \"v\"\n}\n",
    )
    .unwrap();

    let (rx, mut child) = spawn_watch(&project);
    assert!(
        wait_for_line(&rx, "\"event\":\"ready\"", Duration::from_secs(60)).is_some(),
        "watch never reported ready"
    );
    std::thread::sleep(Duration::from_millis(300));
    // Drop the required contract: software's rule reports E006.
    let broken = "behavior login \"L\" {\n  category command\n  verify unit \"v\"\n}\n";
    fs::write(project.path().join("main.spec"), broken).unwrap();

    let rebuilt = wait_for_line(&rx, "\"event\":\"rebuilt\"", Duration::from_secs(60));
    let _ = child.kill();
    let _ = child.wait();

    let check = specforge_cmd()
        .args(["check", "--format", "json"])
        .arg(project.path())
        .output()
        .unwrap();
    let check: serde_json::Value = serde_json::from_slice(&check.stdout).unwrap();
    let check_errors = check
        .as_array()
        .unwrap_or_else(|| panic!("{check}"))
        .iter()
        .filter(|d| {
            d["severity"]
                .as_str()
                .is_some_and(|s| s.eq_ignore_ascii_case("error"))
        })
        .count();
    assert!(
        check_errors >= 1,
        "check sees the missing contract: {check}"
    );

    let event: serde_json::Value = serde_json::from_str(&rebuilt.expect("no rebuild")).unwrap();
    assert_eq!(
        event["errors"], check_errors,
        "watch: {event}\ncheck: {check}"
    );
}

/// A diagnostic list (check's JSON, or a watch event's `diagnostics`) as a
/// multiset of compact JSON: order-independent, nothing dropped.
fn diagnostic_set(list: &serde_json::Value) -> std::collections::BTreeMap<String, usize> {
    let mut set = std::collections::BTreeMap::new();
    for d in list
        .as_array()
        .unwrap_or_else(|| panic!("not a list: {list}"))
    {
        *set.entry(d.to_string()).or_default() += 1;
    }
    set
}

/// Copy a parity fixture, run watch on it until `ready`, rewrite `main.spec`
/// unchanged plus a newline, and return (ready, rebuilt, check's list).
fn watch_and_check(fixture: &str) -> (serde_json::Value, serde_json::Value, serde_json::Value) {
    let project = TempDir::new().unwrap();
    let from = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/parity")
        .join(fixture);
    for entry in fs::read_dir(&from).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), project.path().join(entry.file_name())).unwrap();
    }

    let (rx, mut child) = spawn_watch(&project);
    let ready = wait_for_line(&rx, "\"event\":\"ready\"", Duration::from_secs(60))
        .expect("watch never reported ready");
    std::thread::sleep(Duration::from_millis(300));
    let main = project.path().join("main.spec");
    let text = fs::read_to_string(&main).unwrap();
    fs::write(&main, format!("{text}\n")).unwrap();
    let rebuilt = wait_for_line(&rx, "\"event\":\"rebuilt\"", Duration::from_secs(60))
        .expect("no rebuild event");
    let _ = child.kill();
    let _ = child.wait();

    let check = specforge_cmd()
        .args(["check", "--format", "json"])
        .arg(project.path())
        .output()
        .unwrap();
    (
        serde_json::from_str(&ready).unwrap(),
        serde_json::from_str(&rebuilt).unwrap(),
        serde_json::from_slice(&check.stdout).unwrap(),
    )
}

/// The resolver's E025 reaches watch at startup and after a rebuild, with
/// everything else `check` reports (plan 01, D1).
#[specforge_test(
    behavior = "emit_incremental_diagnostics",
    verify = "total diagnostic set matches full rebuild"
)]
fn watch_reports_what_check_reports_for_a_missing_import() {
    let (ready, rebuilt, check) = watch_and_check("missing_import");
    assert!(
        diagnostic_set(&check)
            .keys()
            .any(|d| d.contains("\"E025\"")),
        "{check}"
    );
    assert_eq!(
        diagnostic_set(&ready["diagnostics"]),
        diagnostic_set(&check)
    );
    assert_eq!(
        diagnostic_set(&rebuilt["diagnostics"]),
        diagnostic_set(&check)
    );
    assert_eq!(rebuilt["verification"], "passed", "{rebuilt}");
}

/// An extension that fails to load (E028) is reported by watch as by
/// `check`, before and after a rebuild (plan 01, D2).
#[specforge_test(
    behavior = "rebuild_affected_subgraph",
    verify = "incremental rebuild equals cold rebuild"
)]
fn watch_reports_what_check_reports_for_an_extension_that_fails_to_load() {
    let (ready, rebuilt, check) = watch_and_check("unknown_extension");
    assert!(
        diagnostic_set(&check)
            .keys()
            .any(|d| d.contains("\"E028\"")),
        "{check}"
    );
    assert_eq!(
        diagnostic_set(&ready["diagnostics"]),
        diagnostic_set(&check)
    );
    assert_eq!(
        diagnostic_set(&rebuilt["diagnostics"]),
        diagnostic_set(&check)
    );
    assert_eq!(rebuilt["verification"], "passed", "{rebuilt}");
}

/// Watch writes the freshness marker in the project root as soon as it is
/// ready, not only after a rebuild, so a running MCP server sees it (D9).
#[test]
fn watch_writes_the_freshness_marker_at_startup() {
    let project = TempDir::new().unwrap();
    fs::write(project.path().join("specforge.json"), "{}").unwrap();
    fs::write(
        project.path().join("main.spec"),
        "entity one { title \"One\" }\n",
    )
    .unwrap();

    let (rx, mut child) = spawn_watch(&project);
    let ready = wait_for_line(&rx, "\"event\":\"ready\"", Duration::from_secs(60));
    let _ = child.kill();
    let _ = child.wait();

    assert!(ready.is_some(), "watch never reported ready");
    let marker = project.path().join(".specforge/graph.json");
    let marker: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&marker).expect("no marker at startup")).unwrap();
    assert_eq!(marker["nodes"], 1, "{marker}");
}
