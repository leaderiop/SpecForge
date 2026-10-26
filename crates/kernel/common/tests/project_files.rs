//! A project file is read only when it is a regular file of at most 64 MiB (ADR 0264).
#![cfg(unix)]
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use std::fs::{File, OpenOptions};
use std::path::Path;
use std::time::Duration;

use specforge_common::{ConfigProblem, read_project_config};
use specforge_test_macros::test as specforge_test;
use tempfile::TempDir;

fn pipe(path: &Path) {
    let made = std::process::Command::new("mkfifo").arg(path).status();
    assert!(made.unwrap().success());
}

/// `f`'s result, or `None` when it did not return within 2 s.
fn within<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(Duration::from_secs(2)).ok()
}

/// What `read_project_config` says unreadable about `root`.
fn unreadable(root: &Path) -> String {
    let read = read_project_config(root);
    match read.problems.as_slice() {
        [ConfigProblem::Unreadable { error, .. }] => error.clone(),
        other => panic!("{other:?}"),
    }
}

#[specforge_test(
    behavior = "load_extension_manifests",
    verify = "a specforge.json that is not a regular file, or is over 64 MiB, is E069 at once, never waited on or read whole"
)]
#[ignore = "flipped by T4"]
fn a_config_that_is_a_named_pipe_is_unreadable_at_once() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("specforge.json");
    pipe(&path);
    let root = dir.path().to_path_buf();
    let read = within(move || unreadable(&root));
    // Release a reader that is still waiting, so the process ends.
    std::thread::spawn(move || {
        let _ = OpenOptions::new().write(true).open(path);
    });
    let text = read.expect("the read waited for a writer");
    assert!(text.contains("named pipe"), "{text}");
}

#[specforge_test(
    behavior = "load_extension_manifests",
    verify = "a specforge.json that is not a regular file, or is over 64 MiB, is E069 at once, never waited on or read whole"
)]
#[ignore = "flipped by T4"]
fn a_config_over_the_ceiling_is_unreadable_unread() {
    let dir = TempDir::new().unwrap();
    File::create(dir.path().join("specforge.json"))
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
    let text = unreadable(dir.path());
    assert!(text.contains("larger than 64 MiB"), "{text}");
}
