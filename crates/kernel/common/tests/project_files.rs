//! A project file is read only when it is a regular file of at most 64 MiB (ADR 0264).
#![cfg(unix)]
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use std::fs::{File, OpenOptions};
use std::path::Path;
use std::time::Duration;

use specforge_common::regular;
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
fn a_config_over_the_ceiling_is_unreadable_unread() {
    let dir = TempDir::new().unwrap();
    File::create(dir.path().join("specforge.json"))
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
    let text = unreadable(dir.path());
    assert!(text.contains("larger than 64 MiB"), "{text}");
}

fn within_pipe_release(path: &Path) {
    // Release a reader that is still waiting, so the process ends.
    let path = path.to_path_buf();
    std::thread::spawn(move || {
        let _ = OpenOptions::new().write(true).open(path);
    });
}

#[test]
fn the_reader_refuses_a_named_pipe_without_opening_it() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("p");
    pipe(&path);
    let read_path = path.clone();
    let read = within(move || regular::open(&read_path).map(|_| ()));
    within_pipe_release(&path);
    let error = read.expect("the open waited for a writer").unwrap_err();
    assert!(error.to_string().contains("named pipe"), "{error}");
}

#[test]
fn the_reader_names_a_character_device() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("dev");
    std::os::unix::fs::symlink("/dev/null", &path).unwrap();
    let error = regular::read(&path).unwrap_err();
    assert!(error.to_string().contains("character device"), "{error}");
}

#[test]
fn the_reader_refuses_a_stated_length_over_the_ceiling_before_reading() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("big");
    File::create(&path)
        .unwrap()
        .set_len(regular::MAX_FILE_BYTES + 1)
        .unwrap();
    let error = regular::open(&path).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::FileTooLarge);
}

#[test]
fn the_reader_reads_a_file_of_exactly_the_ceiling() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("exact");
    File::create(&path)
        .unwrap()
        .set_len(regular::MAX_FILE_BYTES)
        .unwrap();
    assert_eq!(
        regular::read(&path).unwrap().len() as u64,
        regular::MAX_FILE_BYTES
    );
}

#[test]
fn the_reader_follows_a_link_to_a_regular_file() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("real"), "HERE").unwrap();
    std::os::unix::fs::symlink(dir.path().join("real"), dir.path().join("link")).unwrap();
    assert_eq!(
        regular::read_to_string(&dir.path().join("link")).unwrap(),
        "HERE"
    );
}

#[test]
fn a_dangling_link_reads_as_not_found() {
    let dir = TempDir::new().unwrap();
    std::os::unix::fs::symlink(dir.path().join("gone"), dir.path().join("link")).unwrap();
    let error = regular::read(&dir.path().join("link")).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn a_directory_reads_as_is_a_directory() {
    let dir = TempDir::new().unwrap();
    let error = regular::read(dir.path()).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::IsADirectory);
}

#[test]
fn no_refusal_is_invalid_input() {
    let dir = TempDir::new().unwrap();
    let device = dir.path().join("dev");
    std::os::unix::fs::symlink("/dev/null", &device).unwrap();
    let big = dir.path().join("big");
    File::create(&big)
        .unwrap()
        .set_len(regular::MAX_FILE_BYTES + 1)
        .unwrap();
    for path in [device.as_path(), big.as_path(), dir.path()] {
        let kind = regular::read(path).unwrap_err().kind();
        assert_ne!(kind, std::io::ErrorKind::InvalidInput, "{path:?}");
    }
}
