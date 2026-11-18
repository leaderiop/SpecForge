//! absorb's target paths end to end: a stop leaves a directory it did not make as it was, a ledger it cannot write or
//! a symbolic link it would write through stops the run before any write, and a target path longer than 260
//! characters converts. A module of the `absorb_e2e` binary (`absorb_e2e.rs` declares it), so the test binary stays
//! one.

use std::collections::BTreeMap;
use std::path::Path;

use super::{OLD, absorb, path};

/// A symbolic link to the file `target` at `link`; it never returns without one (ADR 0375 D4). The bin's own helper
/// (`absorb/tests.rs`) is out of an integration test's reach, so this is its twin.
#[cfg(unix)]
fn link_file(target: &Path, link: &Path) {
    std::os::unix::fs::symlink(target, link)
        .unwrap_or_else(|e| panic!("a link {} -> {}: {e}", link.display(), target.display()));
}

/// A symbolic link to the file `target` at `link`; it never returns without one (ADR 0375 D4): a file has no junction,
/// so a link it cannot make fails the test.
#[cfg(windows)]
fn link_file(target: &Path, link: &Path) {
    std::os::windows::fs::symlink_file(target, link).unwrap_or_else(|e| {
        panic!(
            "a file link {} -> {}: {e}; a file link needs the privilege to create symbolic links or Developer Mode",
            link.display(),
            target.display()
        )
    });
}

/// The version the old product builtin accepts and the id grammar refuses: its milestone would be `v1.0.0+build.1`.
const REFUSED_VERSION: &str = "1.0.0+build.1";

/// A source at `old` whose one release has [`REFUSED_VERSION`]: it converts to a version milestone whose id the target
/// refuses (`+` is no id character), so the run stops after init wrote, before the edit plan applies.
fn refused_version_source(old: &Path) {
    std::fs::create_dir_all(old.join("spec")).unwrap();
    std::fs::write(
        old.join("specforge.json"),
        r#"{"name": "refused", "spec_root": "spec", "extensions": ["@specforge/product"]}"#,
    )
    .unwrap();
    std::fs::write(
        old.join("spec/releases.spec"),
        format!(
            "release build \"Build\" {{\n  version     \"{REFUSED_VERSION}\"\n  description \"A build.\"\n  status      \
             planned\n}}\n"
        ),
    )
    .unwrap();
}

/// Every entry under `root` (symbolic links not followed), by its path from `root`, with a file's bytes.
fn tree_bytes(root: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
    walkdir::WalkDir::new(root)
        .min_depth(1)
        .sort_by_file_name()
        .into_iter()
        .filter_map(Result::ok)
        .map(|e| {
            let key = e.path().strip_prefix(root).unwrap().display().to_string();
            let bytes = e
                .file_type()
                .is_file()
                .then(|| std::fs::read(e.path()).unwrap());
            (key, bytes)
        })
        .collect()
}

/// A stop in a directory the run did not make leaves it byte for byte as it was: the refused version stops the run
/// after init appended to its `.gitignore` and `.gitattributes`, and the roll back gives them back their bytes.
#[test]
fn a_stop_leaves_a_directory_it_did_not_make_byte_for_byte() {
    let dir = tempfile::tempdir().unwrap();
    let (old, to) = (dir.path().join("old"), dir.path().join("tree"));
    refused_version_source(&old);
    std::fs::create_dir_all(to.join("src")).unwrap();
    std::fs::write(to.join(".gitignore"), "/target").unwrap();
    std::fs::write(to.join(".gitattributes"), "*.rs text\n").unwrap();
    std::fs::write(to.join("Cargo.toml"), "[workspace]\n").unwrap();
    std::fs::write(to.join("src/lib.rs"), "").unwrap();
    let before = tree_bytes(&to);
    let run = absorb(&["--from", path(&old), "--to", path(&to)]);
    assert_eq!(run.code, 2, "{}", run.stderr);
    assert!(
        run.stderr.contains("1 id(s) the target refuses"),
        "{}",
        run.stderr
    );
    assert_eq!(tree_bytes(&to), before);
}

/// A symbolic link at the ledger path, the default one in `--to` or `--ledger`'s, is refused before any write: the file
/// it points to, outside `--to`, is never written through, and `--to` is left as it was. It runs on both OSes, its
/// links made by [`link_file`] (ADR 0375 D4).
#[test]
fn a_symbolic_link_at_the_ledger_path_is_refused_before_any_write() {
    let dir = tempfile::tempdir().unwrap();
    let outside = dir.path().join("outside.json");
    std::fs::write(&outside, "mine\n").unwrap();
    let to = dir.path().join("tree");
    std::fs::create_dir_all(&to).unwrap();
    link_file(&outside, &to.join("absorb-ledger.json"));
    let before = tree_bytes(&to);
    let run = absorb(&["--from", OLD, "--to", path(&to)]);
    assert_eq!(run.code, 2, "{}", run.stderr);
    assert!(
        run.stderr
            .contains("is a symbolic link: absorb never writes through one"),
        "{}",
        run.stderr
    );
    assert_eq!(tree_bytes(&to), before);
    let given = dir.path().join("given.json");
    link_file(&outside, &given);
    let fresh = dir.path().join("fresh");
    let run = absorb(&[
        "--from",
        OLD,
        "--to",
        path(&fresh),
        "--ledger",
        path(&given),
    ]);
    assert_eq!(run.code, 2, "{}", run.stderr);
    assert!(!fresh.exists(), "a --to it would make is not made");
    assert_eq!(
        std::fs::read_to_string(&outside).unwrap(),
        "mine\n",
        "never written through"
    );
}

/// A ledger that cannot be written is a stop before any write: the ledger is written before the edit plan applies, so
/// the record of what does not convert is never lost behind a written target, and the tree is left as it was.
#[test]
fn a_ledger_that_cannot_be_written_stops_before_any_write() {
    let dir = tempfile::tempdir().unwrap();
    let to = dir.path().join("tree");
    std::fs::create_dir_all(&to).unwrap();
    std::fs::write(to.join("README.md"), "notes\n").unwrap();
    let not_a_dir = dir.path().join("a-file");
    std::fs::write(&not_a_dir, "").unwrap();
    let ledger = not_a_dir.join("ledger.json");
    let before = tree_bytes(&to);
    let run = absorb(&["--from", OLD, "--to", path(&to), "--ledger", path(&ledger)]);
    assert_eq!(run.code, 2, "{}", run.stderr);
    assert!(
        run.stderr.contains("the ledger: ") && run.stderr.contains("nothing was written"),
        "{}",
        run.stderr
    );
    assert_eq!(tree_bytes(&to), before);
}

/// A milestone id the target refuses, from a release's version, stops the run before any write, naming it; the scaffold
/// is removed (ADR 0372 D3: every id goes through the target's id rule).
#[test]
fn a_version_the_target_refuses_as_an_id_stops_the_run_before_any_write() {
    let dir = tempfile::tempdir().unwrap();
    let (old, to) = (dir.path().join("old"), dir.path().join("new"));
    refused_version_source(&old);
    let run = absorb(&["--from", path(&old), "--to", path(&to)]);
    assert_eq!(run.code, 2, "{}", run.stderr);
    assert!(
        run.stderr.contains("1 id(s) the target refuses")
            && run.stderr.contains(&format!(
                "milestone 'v{REFUSED_VERSION}': invalid entity ID"
            )),
        "{}",
        run.stderr
    );
    assert!(!to.exists(), "the scaffold is removed");
}

/// A `--to` past Windows' old 260-character limit converts and checks clean: the Windows leg runs this, where
/// `core.longpaths` and std's long-path handling are what it tests (ADR 0375 D4).
#[test]
fn a_target_longer_than_260_characters_converts_and_checks_clean() {
    let dir = tempfile::tempdir().unwrap();
    let mut to = dir.path().to_path_buf();
    while to.as_os_str().len() <= 260 {
        to.push("t".repeat(60));
    }
    assert!(to.as_os_str().len() > 260, "{} is not long", to.display());
    let run = absorb(&["--from", OLD, "--to", path(&to)]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.summary["target"]["errors"], 0, "{}", run.summary);
    assert!(to.join("specforge.json").is_file());
}
