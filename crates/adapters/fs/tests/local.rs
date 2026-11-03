//! The `Fs` port's adapter over a real directory (ADR 0335): the contract its fake keeps, containment, the read cap
//! and the walk's skip list and links.
#![allow(
    clippy::unwrap_used,
    reason = "a test fails on the first unexpected value"
)]

use specforge_effects::{Fs, FsOp, FsOutcome, FsRequest, OutcomeClass, Roots};
use specforge_fs::LocalFs;

/// A project with `a.txt` (`a\n`) and a `.env`, and its roots.
fn project() -> (tempfile::TempDir, Roots) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    std::fs::write(root.join("a.txt"), "a\n").unwrap();
    std::fs::write(root.join(".env"), "TOKEN=x\n").unwrap();
    std::fs::create_dir(root.join("spec")).unwrap();
    let roots = Roots::new(&root, root.join("spec"));
    (dir, roots)
}

fn run(roots: &Roots, path: &str, op: FsOp) -> FsOutcome {
    LocalFs.run(
        roots,
        &FsRequest {
            root: "project".into(),
            path: path.into(),
            op,
        },
    )
}

fn walk(roots: &Roots, glob: Option<&str>) -> Vec<String> {
    let walked = run(
        roots,
        "",
        FsOp::Walk {
            glob: glob.map(str::to_string),
            max_entries: 100,
        },
    );
    assert_eq!(walked.class, OutcomeClass::Ok, "{walked:?}");
    walked.entries
}

#[test]
fn the_local_fs_keeps_the_fs_contract() {
    let (_dir, roots) = project();
    specforge_effects::testing::fs_contract(&LocalFs, &roots);
}

#[test]
fn the_fake_fs_keeps_the_same_contract() {
    let roots = Roots::new("/p", "/p/spec");
    let fake = specforge_effects::testing::FakeFs::new()
        .with_file("/p/a.txt", "a\n")
        .with_file("/p/.env", "TOKEN=x\n");
    specforge_effects::testing::fs_contract(&fake, &roots);
}

#[test]
fn a_read_past_its_max_bytes_is_cut_there_with_the_files_size() {
    let (_dir, roots) = project();
    std::fs::write(roots.project().join("long.txt"), "0123456789").unwrap();
    let read = run(&roots, "long.txt", FsOp::Read { max_bytes: 4 });
    assert_eq!(read.class, OutcomeClass::Ok);
    assert_eq!(read.bytes, b"0123");
    assert!(read.truncated);
    assert_eq!(read.stat.unwrap().size, 10);
}

#[cfg(unix)]
#[test]
fn a_link_out_of_the_root_is_outside() {
    let (_dir, roots) = project();
    let elsewhere = tempfile::tempdir().unwrap();
    std::fs::write(elsewhere.path().join("secret.txt"), "s").unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), roots.project().join("out")).unwrap();
    let read = run(&roots, "out/secret.txt", FsOp::Read { max_bytes: 10 });
    assert_eq!(read.class, OutcomeClass::Outside, "{read:?}");
}

#[cfg(unix)]
#[test]
fn a_link_to_a_withheld_file_is_not_read() {
    let (_dir, roots) = project();
    std::os::unix::fs::symlink(".env", roots.project().join("notes.txt")).unwrap();
    let read = FsOp::Read { max_bytes: 100 };
    assert_eq!(run(&roots, "notes.txt", read).class, OutcomeClass::Denied);
    assert_eq!(run(&roots, "notes.txt", FsOp::Stat).class, OutcomeClass::Ok);
}

#[test]
fn a_walk_skips_dependency_directories_the_glob_does_not_name() {
    let (_dir, roots) = project();
    let root = roots.project();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/main.ts"), "").unwrap();
    // Every directory ADR 0293 D4 names.
    for skipped in specforge_effects::fs::SKIPPED {
        std::fs::create_dir_all(root.join(skipped).join("pkg")).unwrap();
        std::fs::write(root.join(skipped).join("pkg/index.ts"), "").unwrap();
    }
    assert_eq!(walk(&roots, Some("**/*.ts")), ["src/main.ts"]);
    assert_eq!(
        walk(&roots, Some("node_modules/**/*.ts")),
        ["node_modules/pkg/index.ts"]
    );
}

#[test]
fn a_walk_stops_at_its_entry_budget() {
    let (_dir, roots) = project();
    let walked = run(
        &roots,
        "",
        FsOp::Walk {
            glob: None,
            max_entries: 1,
        },
    );
    assert!(walked.truncated);
    assert_eq!(walked.entries.len(), 1);
}

#[cfg(unix)]
#[test]
fn a_walk_does_not_descend_through_a_directory_link() {
    let (_dir, roots) = project();
    let root = roots.project();
    std::fs::create_dir(root.join("real")).unwrap();
    std::fs::write(root.join("real/x.md"), "").unwrap();
    std::os::unix::fs::symlink(root.join("real"), root.join("linked")).unwrap();
    assert_eq!(walk(&roots, Some("**/*.md")), ["real/x.md"]);
}
