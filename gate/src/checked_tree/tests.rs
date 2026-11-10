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
