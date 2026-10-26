//! A held file is read only when it is a regular file of at most 64 MiB (ADR 0264).
#![cfg(unix)]
use super::*;
use std::io::ErrorKind;
use std::time::Duration;

fn scene() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::TempDir::new().unwrap();
    let real = std::fs::canonicalize(dir.path()).unwrap();
    (dir, real)
}

fn over_ceiling(path: &Path) {
    std::fs::File::create(path)
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
}

#[test]
fn a_bounded_read_refuses_a_named_pipe_at_once() {
    let (_dir, root) = scene();
    let pipe = root.join("p");
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&pipe)
            .status()
            .unwrap()
            .success()
    );
    let (tx, rx) = std::sync::mpsc::channel();
    let read_pipe = pipe.clone();
    let read_root = root.clone();
    std::thread::spawn(move || {
        let read = Containment::within(&read_root).read(&read_pipe);
        let _ = tx.send(matches!(
            read,
            Err(ReadFailure::Io { error, .. }) if error.kind() != ErrorKind::InvalidInput
        ));
    });
    let refused = rx.recv_timeout(Duration::from_secs(2));
    // Release a reader that is still waiting, so the process ends.
    std::thread::spawn(move || {
        let _ = std::fs::OpenOptions::new().write(true).open(pipe);
    });
    assert_eq!(refused, Ok(true));
}

#[test]
fn an_unbounded_read_refuses_a_link_to_a_device() {
    let (_dir, root) = scene();
    let link = root.join("dev");
    std::os::unix::fs::symlink("/dev/null", &link).unwrap();
    let read = Containment::unbounded().read(&link);
    assert!(
        matches!(&read, Err(ReadFailure::Io { error, .. }) if error.to_string().contains("character device")),
        "{read:?}"
    );
}

#[test]
fn every_held_read_refuses_a_file_over_the_ceiling() {
    let (_dir, root) = scene();
    let big = root.join("big");
    over_ceiling(&big);
    let held = Containment::unbounded().resolve(&big).unwrap();
    assert_eq!(held.read().unwrap_err().kind(), ErrorKind::FileTooLarge);
    assert_eq!(
        held.read_if_there().unwrap_err().kind(),
        ErrorKind::FileTooLarge
    );
    assert_eq!(
        held.read_to_string().unwrap_err().kind(),
        ErrorKind::FileTooLarge
    );
}
