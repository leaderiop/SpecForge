//! The plain disk reads a project file only when it is a regular file of at most 64 MiB (ADR 0264).
#![cfg(unix)]

use std::fs::{File, OpenOptions};
use std::io::ErrorKind;
use std::path::Path;
use std::time::Duration;

use specforge_registry::{Disk, PlainDisk};
use tempfile::TempDir;

fn pipe(path: &Path) {
    let made = std::process::Command::new("mkfifo").arg(path).status();
    assert!(made.unwrap().success());
}

fn over_ceiling(path: &Path) {
    File::create(path)
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
}

/// `f`'s result, or `None` when it did not return within 2 s.
fn within<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(Duration::from_secs(2)).ok()
}

#[test]
#[ignore = "flipped by T4"]
fn a_named_pipe_is_refused_without_waiting_for_a_writer() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("p");
    pipe(&path);
    let read_path = path.clone();
    let read = within(move || {
        PlainDisk
            .read(&read_path)
            .map_err(|e| (e.kind(), e.to_string()))
    });
    // Release a reader that is still waiting, so the process ends.
    std::thread::spawn(move || {
        let _ = OpenOptions::new().write(true).open(path);
    });
    let (kind, text) = read.expect("the read waited for a writer").unwrap_err();
    assert_ne!(kind, ErrorKind::InvalidInput);
    assert!(text.contains("named pipe"), "{text}");
}

#[test]
#[ignore = "flipped by T4"]
fn a_link_to_a_device_is_refused() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("dev");
    std::os::unix::fs::symlink("/dev/null", &path).unwrap();
    let error = PlainDisk.read(&path).unwrap_err();
    assert!(error.to_string().contains("character device"), "{error}");
}

#[test]
#[ignore = "flipped by T4"]
fn a_file_over_the_ceiling_is_refused_unread() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("big");
    over_ceiling(&path);
    let error = PlainDisk.read(&path).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::FileTooLarge);
    assert!(error.to_string().contains("64 MiB"), "{error}");
}

#[test]
fn a_link_to_a_regular_file_is_read_wherever_it_points() {
    let (a, b) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    std::fs::write(b.path().join("real"), "THERE").unwrap();
    let link = a.path().join("link");
    std::os::unix::fs::symlink(b.path().join("real"), &link).unwrap();
    assert_eq!(PlainDisk.read(&link).unwrap(), b"THERE");
}

#[test]
fn a_directory_reads_as_is_a_directory() {
    let dir = TempDir::new().unwrap();
    let error = PlainDisk.read(dir.path()).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::IsADirectory);
}
