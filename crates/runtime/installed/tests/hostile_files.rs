//! A lock, a module and a .wasm file entry are read only when each is a regular file of at most 64 MiB
//! (ADR 0264): a named pipe, a device or a larger file is refused at once.
#![cfg(unix)]

use std::fs::{File, OpenOptions};
use std::path::Path;
use std::time::Duration;

use specforge_installed::testing::install;
use specforge_installed::{Installed, LoadProblem};
use specforge_package::PackageName;
use specforge_wasm::testing::InProcessRuntime;
use tempfile::TempDir;

use crate::load::{entries, extension, load_with};

fn pipe(path: &Path) {
    let made = std::process::Command::new("mkfifo").arg(path).status();
    assert!(made.unwrap().success());
}

fn device_link(path: &Path) {
    std::os::unix::fs::symlink("/dev/null", path).unwrap();
}

fn over_ceiling(path: &Path) {
    File::create(path)
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
}

/// `f`'s result, or `None` when it did not return within 2 s; a reader still waiting at `pipe` is released.
fn within<T: Send + 'static>(
    pipe: Option<&Path>,
    f: impl FnOnce() -> T + Send + 'static,
) -> Option<T> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    let got = rx.recv_timeout(Duration::from_secs(2)).ok();
    if let Some(pipe) = pipe {
        let pipe = pipe.to_path_buf();
        std::thread::spawn(move || {
            let _ = OpenOptions::new().write(true).open(pipe);
        });
    }
    got
}

fn lock_problem(root: &Path) -> Option<String> {
    Installed::at(root)
        .lock()
        .problem()
        .map(|problem| format!("{} {}", problem.code, problem.message))
}

/// The load of `entry` at `root`: whether its first failure is `ModuleUnreadable`, and its message.
fn load_failure(root: &Path, entry: &str, runtime: InProcessRuntime) -> (bool, String) {
    let loaded = load_with(root, &entries(&[entry]), &runtime);
    let failure = loaded.enabled[0].failure.as_ref().unwrap();
    (
        matches!(failure.problem, LoadProblem::ModuleUnreadable { .. }),
        failure.diagnostic.message.clone(),
    )
}

#[specforge_test_macros::test(
    behavior = "read_lock_file",
    verify = "a specforge.lock that is not a regular file, or is over 64 MiB, is E033 at once, never waited on or read whole"
)]
fn a_lock_that_is_a_named_pipe_is_e033_at_once() {
    let dir = TempDir::new().unwrap();
    let lock = dir.path().join("specforge.lock");
    pipe(&lock);
    let root = dir.path().to_path_buf();
    let problem = within(Some(&lock), move || lock_problem(&root));
    let problem = problem.expect("the read waited for a writer").unwrap();
    assert!(
        problem.contains("E033") && problem.contains("named pipe"),
        "{problem}"
    );
}

#[specforge_test_macros::test(
    behavior = "read_lock_file",
    verify = "a specforge.lock that is not a regular file, or is over 64 MiB, is E033 at once, never waited on or read whole"
)]
fn a_lock_over_the_ceiling_is_e033_unread() {
    let dir = TempDir::new().unwrap();
    over_ceiling(&dir.path().join("specforge.lock"));
    let problem = lock_problem(dir.path()).unwrap();
    assert!(
        problem.contains("E033") && problem.contains("larger than 64 MiB"),
        "{problem}"
    );
}

#[specforge_test_macros::test(
    behavior = "load_extension_manifests",
    verify = "a .wasm file entry or an installed module that is not a regular file, or is over 64 MiB, is E028 at once, never waited on or read whole"
)]
fn a_file_entry_that_is_a_named_pipe_is_e028_at_once() {
    let dir = TempDir::new().unwrap();
    let wasm = dir.path().join("x.wasm");
    pipe(&wasm);
    let root = dir.path().to_path_buf();
    let got = within(Some(&wasm), move || {
        load_failure(&root, "x.wasm", InProcessRuntime::new())
    });
    let (unreadable, message) = got.expect("the read waited for a writer");
    assert!(unreadable && message.contains("named pipe"), "{message}");
}

#[specforge_test_macros::test(
    behavior = "load_extension_manifests",
    verify = "a .wasm file entry or an installed module that is not a regular file, or is over 64 MiB, is E028 at once, never waited on or read whole"
)]
fn an_installed_module_linked_to_a_device_is_unreadable_not_changed() {
    let dir = TempDir::new().unwrap();
    let runtime = || InProcessRuntime::new().with(extension("@acme/a"));
    install(dir.path(), &runtime(), &["@acme/a"]);
    let module = Installed::at(dir.path()).module_path(&PackageName::parse("@acme/a").unwrap());
    std::fs::remove_file(&module).unwrap();
    device_link(&module);

    let (unreadable, message) = load_failure(dir.path(), "@acme/a", runtime());

    assert!(
        unreadable && message.contains("character device"),
        "{message}"
    );
}
