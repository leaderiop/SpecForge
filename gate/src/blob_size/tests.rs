use super::*;

use std::path::Path;
use std::process::{Command, Stdio};

fn sizes(entries: &[(&str, u64)]) -> Vec<(String, u64)> {
    entries.iter().map(|&(p, n)| (p.to_string(), n)).collect()
}

#[test]
fn a_blob_over_the_cap_fails_naming_its_path_and_size() {
    let problems = verdict(&sizes(&[
        ("extensions/a/wasm/a.wasm", 10),
        ("extensions/b/wasm/b.wasm", CAP + 1),
    ]));
    assert_eq!(
        problems,
        ["extensions/b/wasm/b.wasm is 2500001 B, over the 2500000 B cap"]
    );
}

#[test]
fn a_blob_at_the_cap_passes() {
    assert!(verdict(&sizes(&[("extensions/a/wasm/a.wasm", CAP)])).is_empty());
}

#[test]
fn measure_reads_each_blob_and_leaves_out_a_file_that_is_gone() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.wasm"), [0u8; 3]).unwrap();
    std::fs::write(dir.path().join("a.rs"), [0u8; 9]).unwrap();
    let paths = ["a.wasm", "a.rs", "gone.wasm"].map(String::from);
    assert_eq!(
        measure(dir.path(), &paths, Source::Working),
        sizes(&[("a.wasm", 3)])
    );
}

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

#[test]
fn the_hook_measures_the_staged_blob_not_the_working_copy() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git(root, &["init", "-q"]);
    let paths = ["big.wasm", "small.wasm"].map(String::from);
    std::fs::write(root.join("big.wasm"), [0u8; 7]).unwrap();
    std::fs::write(root.join("small.wasm"), [0u8; 1]).unwrap();
    git(root, &["add", "big.wasm", "small.wasm"]);
    // Shrunk and grown after staging: the commit still holds what was staged.
    std::fs::write(root.join("big.wasm"), [0u8; 2]).unwrap();
    std::fs::write(root.join("small.wasm"), [0u8; 5]).unwrap();
    assert_eq!(
        measure(root, &paths, Source::Staged),
        sizes(&[("big.wasm", 7), ("small.wasm", 1)])
    );
    assert_eq!(
        measure(root, &paths, Source::Working),
        sizes(&[("big.wasm", 2), ("small.wasm", 5)])
    );
}
