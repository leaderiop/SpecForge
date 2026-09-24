// `specforge watch` — end-to-end smoke test against the real binary.

use std::fs;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use assert_cmd::cargo::CommandCargoExt;
use tempfile::TempDir;

#[allow(deprecated)]
fn specforge_cmd() -> Command {
    Command::cargo_bin("specforge").unwrap()
}

/// Spawn `specforge watch --json` on a fresh project and return the line
/// receiver plus the child handle.
fn spawn_watch(project: &TempDir) -> (mpsc::Receiver<String>, std::process::Child) {
    let mut child = specforge_cmd()
        .args([
            "watch",
            "--path",
            project.path().to_str().unwrap(),
            "--json",
        ])
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
