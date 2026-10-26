//! A project holding a named pipe where it expects a file is refused at once, on `check` and over MCP
//! (ADR 0264); no process waits for a writer that never comes.
#![cfg(unix)]

use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::path::Path;
use std::process::Child;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::support::mcp_process::{initialize, request};
use crate::support::{Cli, TestProject};
use specforge_test_macros::test as specforge_test;

fn pipe(path: &Path) {
    let made = std::process::Command::new("mkfifo").arg(path).status();
    assert!(made.unwrap().success());
}

/// Kills and reaps the child when the test ends, so none outlives it.
struct Reaped(Child);

impl Drop for Reaped {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[specforge_test(
    behavior = "load_extension_manifests",
    verify = "check reports a .wasm file entry that is a named pipe as E028 at once"
)]
fn check_reports_a_wasm_file_entry_that_is_a_named_pipe_as_e028_at_once() {
    let project = TestProject::new().raw_config(
        r#"{"name":"t","version":"0.1.0","extensions":["@specforge/software","x.wasm"]}"#,
    );
    pipe(&project.root().join("x.wasm"));
    let mut child = Reaped(Cli::on(&project).start(&["check"]));
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break Some(status);
        }
        if Instant::now() > deadline {
            break None;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let status = status.expect("check waited for a writer on the named pipe");
    let mut text = String::new();
    child
        .0
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    child
        .0
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    assert_eq!(status.code(), Some(1), "{text}");
    assert!(
        text.contains("E028") && text.contains("named pipe"),
        "{text}"
    );
}

#[specforge_test(
    behavior = "mcp_initialize",
    verify = "specforge mcp answers initialize over a project whose specforge.lock is a named pipe, and doctor reports the lock"
)]
fn mcp_answers_initialize_over_a_project_whose_lock_is_a_named_pipe() {
    let project = TestProject::new()
        .raw_config(r#"{"name":"t","version":"0.1.0","extensions":["@specforge/software"]}"#);
    pipe(&project.root().join("specforge.lock"));
    let root = project.root().to_str().unwrap().to_string();
    let mut child = Reaped(Cli::on(&project).start(&["mcp", &root]));
    let mut stdin = child.0.stdin.take().unwrap();
    let stdout = child.0.stdout.take().unwrap();
    let (sender, lines) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if sender.send(line).is_err() {
                break;
            }
        }
    });
    let doctor = request(
        1,
        "tools/call",
        serde_json::json!({"name": "specforge.doctor", "arguments": {}}),
    );
    writeln!(stdin, "{}\n{doctor}", initialize(0)).unwrap();
    let initialized = lines
        .recv_timeout(Duration::from_secs(10))
        .expect("initialize was answered within 10 s");
    assert!(initialized.contains("capabilities"), "{initialized}");
    let report = lines
        .recv_timeout(Duration::from_secs(30))
        .expect("doctor answered");
    assert!(
        report.contains("lock_unreadable") && report.contains("named pipe"),
        "{report}"
    );
}
