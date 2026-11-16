use super::*;

#[test]
fn a_stray_fact_store_and_ledger_are_not_in_the_checked_tree() {
    let repo = tempfile::tempdir().unwrap();
    let root = repo.path();
    std::fs::create_dir_all(root.join("spec")).unwrap();
    std::fs::write(root.join("specforge.json"), "{}").unwrap();
    std::fs::write(root.join("spec/a.spec"), "behavior a \"A\" {\n}\n").unwrap();
    // What a local collect leaves: a store, and the ledger line recording it; neither is tracked.
    std::fs::create_dir_all(root.join(".specforge/facts/worktree")).unwrap();
    std::fs::write(
        root.join(".specforge/facts/worktree/rust-source.json"),
        "{}",
    )
    .unwrap();
    std::fs::write(
        root.join("specforge-harness"),
        "rust-source facts:worktree sha256:00\n",
    )
    .unwrap();
    let tracked = ["specforge.json", "spec/a.spec", "spec/deleted.spec"].map(String::from);
    let scratch = tempfile::tempdir().unwrap();
    let into = scratch.path().join("tree");
    // A copy left by an earlier run is replaced.
    std::fs::create_dir_all(into.join(".specforge")).unwrap();

    copy_tracked(root, &tracked, &into).unwrap();

    assert_eq!(
        std::fs::read_to_string(into.join("spec/a.spec")).unwrap(),
        "behavior a \"A\" {\n}\n"
    );
    assert!(into.join("specforge.json").is_file());
    assert!(!into.join(".specforge").exists());
    assert!(!into.join("specforge-harness").exists());
    assert!(!into.join("spec/deleted.spec").exists());
}

#[test]
fn a_copy_keeps_each_file_s_modification_time() {
    let repo = tempfile::tempdir().unwrap();
    let root = repo.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/lib.rs"), "pub fn a() {}\n").unwrap();
    let an_hour_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    std::fs::File::options()
        .write(true)
        .open(root.join("src/lib.rs"))
        .unwrap()
        .set_modified(an_hour_ago)
        .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(root.join("run.sh"), "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(root.join("run.sh"), std::fs::Permissions::from_mode(0o755))
            .unwrap();
    }
    let scratch = tempfile::tempdir().unwrap();
    let into = scratch.path().join("tree");

    copy_tracked(root, &["src/lib.rs", "run.sh"].map(String::from), &into).unwrap();

    let copied = std::fs::metadata(into.join("src/lib.rs")).unwrap();
    assert_eq!(copied.modified().unwrap(), an_hour_ago);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(into.join("run.sh"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755, "a tracked script stays executable");
    }
}

#[test]
fn a_read_only_tracked_file_is_copied_with_its_time_and_mode_and_copied_over() {
    let repo = tempfile::tempdir().unwrap();
    let root = repo.path();
    let file = root.join("locked.txt");
    std::fs::write(&file, "kept\n").unwrap();
    let an_hour_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    std::fs::File::options()
        .write(true)
        .open(&file)
        .unwrap()
        .set_modified(an_hour_ago)
        .unwrap();
    let mut read_only = std::fs::metadata(&file).unwrap().permissions();
    read_only.set_readonly(true);
    std::fs::set_permissions(&file, read_only).unwrap();
    let tracked = ["locked.txt".to_string()];
    let scratch = tempfile::tempdir().unwrap();
    let into = scratch.path().join("tree");
    // Twice: the second run empties a copy that holds a read-only file.
    for _ in 0..2 {
        copy_tracked(root, &tracked, &into).unwrap();
        let copy = std::fs::metadata(into.join("locked.txt")).unwrap();
        assert!(copy.permissions().readonly());
        assert_eq!(copy.modified().unwrap(), an_hour_ago);
        assert_eq!(
            std::fs::read_to_string(into.join("locked.txt")).unwrap(),
            "kept\n"
        );
    }
    // The temporary directories are removed by their owner: give the original its write bit back first.
    let mut writable = std::fs::metadata(&file).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false, reason = "a test's own file")]
    writable.set_readonly(false);
    std::fs::set_permissions(&file, writable).unwrap();
}
