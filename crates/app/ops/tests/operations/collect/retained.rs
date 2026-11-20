//! A collect archives the fact stores it supersedes (ADR 0376 D4): moved to `.specforge/facts-history/<seq>-<key>/`
//! in the change that writes the new store, journaled, each collector keeping its newest `facts.retain`; a failed
//! change puts every store back where it was and removes no directory.

use std::sync::Arc;

use serde_json::json;
use specforge_effects::SharedGit;
use specforge_effects::testing::FakeGit;
use specforge_project::facts::Store;
use specforge_project::facts::retain::{self, Archived};
use specforge_registry::PlainDisk;
use specforge_test_macros::test as specforge_test;

use super::facts::{SHA, collecting, nothing, project, serving, two_tests};

const NEXT: &str = "fedcba9876543210fedcba9876543210fedcba98";
const THIRD: &str = "00112233445566778899aabbccddeeff00112233";

/// The git a collect at `sha` asks.
fn at(p: &specforge_project::testing::TestProject, sha: &str) -> SharedGit {
    Arc::new(FakeGit::repo(p.root(), sha, Vec::new()))
}

/// `node`'s archives under the project root.
fn archived(p: &specforge_project::testing::TestProject) -> Vec<Archived> {
    retain::archived(&PlainDisk, p.root(), "node")
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_collect_archives_the_store_it_supersedes() {
    let p = project();
    super::facts::collect(
        &p,
        &at(&p, SHA),
        serving(two_tests(), nothing()),
        Some("node"),
    );
    let first = std::fs::read(p.root().join(Store::path("node", SHA))).unwrap();

    super::facts::collect(
        &p,
        &at(&p, NEXT),
        serving(json!([two_tests()[0]]), nothing()),
        Some("node"),
    );

    assert!(p.root().join(Store::path("node", NEXT)).is_file());
    assert!(
        !p.root().join(format!(".specforge/facts/{SHA}")).exists(),
        "the live key's directory the move emptied is gone"
    );
    let [only] = archived(&p).try_into().unwrap_or_else(|a| panic!("{a:?}"));
    assert_eq!((only.seq, only.key.as_str()), (1, SHA));
    assert_eq!(
        only.path,
        format!(".specforge/facts-history/000001-{SHA}/node.json")
    );
    assert_eq!(
        std::fs::read(p.root().join(&only.path)).unwrap(),
        first,
        "byte for byte"
    );
    // the journal went with the change
    assert!(!p.root().join(".specforge/journal").exists());
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_collect_prunes_the_collectors_archives_beyond_facts_retain() {
    let p = project();
    p.config_edit(|c| c["facts"] = json!({"retain": 1}));
    for (sha, facts) in [
        (SHA, two_tests()),
        (NEXT, json!([two_tests()[0]])),
        (THIRD, two_tests()),
    ] {
        super::facts::collect(&p, &at(&p, sha), serving(facts, nothing()), Some("node"));
    }
    let [only] = archived(&p).try_into().unwrap_or_else(|a| panic!("{a:?}"));
    assert_eq!((only.seq, only.key.as_str()), (2, NEXT));
    assert!(
        !p.root()
            .join(format!(".specforge/facts-history/000001-{SHA}"))
            .exists(),
        "the directory the prune emptied is gone"
    );
    assert!(p.root().join(Store::path("node", THIRD)).is_file());
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_failed_collect_puts_every_store_back_and_removes_no_directory() {
    let p = project();
    super::facts::collect(
        &p,
        &at(&p, SHA),
        serving(two_tests(), nothing()),
        Some("node"),
    );
    let live = Store::path("node", SHA);
    let before = std::fs::read(p.root().join(&live)).unwrap();
    // the archive's directory is a file: the move cannot be written, and the change fails
    std::fs::write(p.root().join(".specforge/facts-history"), "in the way").unwrap();

    let (_, stored) = collecting(
        &p,
        &at(&p, NEXT),
        serving(json!([two_tests()[0]]), nothing()),
        Some("node"),
    );

    assert!(stored.is_err(), "{stored:?}");
    assert_eq!(
        std::fs::read(p.root().join(&live)).unwrap(),
        before,
        "byte for byte"
    );
    assert!(
        !p.root().join(Store::path("node", NEXT)).exists(),
        "the new store is not left"
    );
    assert!(p.root().join(format!(".specforge/facts/{SHA}")).is_dir());
    assert_eq!(
        std::fs::read_to_string(p.root().join(".specforge/facts-history")).unwrap(),
        "in the way"
    );
    assert!(
        super::facts::ledger(&p).contains(&format!("node facts:{SHA} ")),
        "the ledger still records the first store"
    );
}

#[cfg(unix)]
#[specforge_test(requirement = "FEATURE-142")]
fn a_collect_over_an_archive_it_cannot_list_archives_nothing_and_overwrites_nothing() {
    use std::os::unix::fs::PermissionsExt;
    let p = project();
    super::facts::collect(
        &p,
        &at(&p, SHA),
        serving(two_tests(), nothing()),
        Some("node"),
    );
    let live = Store::path("node", SHA);
    let before = std::fs::read(p.root().join(&live)).unwrap();
    // an earlier store of the same key, archived first: listed, the next seq is 2
    let kept = format!(".specforge/facts-history/000001-{SHA}/node.json");
    std::fs::create_dir_all(p.root().join(&kept).parent().unwrap()).unwrap();
    std::fs::write(p.root().join(&kept), "archived first").unwrap();
    // the archive may be entered, not listed
    let history = p.root().join(".specforge/facts-history");
    let mode = |m: u32| std::fs::set_permissions(&history, std::fs::Permissions::from_mode(m));
    mode(0o300).unwrap();
    if std::fs::read_dir(&history).is_ok() {
        mode(0o755).unwrap();
        eprintln!("skipped: this user lists a directory it may not read (root)");
        return;
    }

    let (_, stored) = collecting(
        &p,
        &at(&p, NEXT),
        serving(json!([two_tests()[0]]), nothing()),
        Some("node"),
    );
    mode(0o755).unwrap();

    let error = stored.expect_err("an archive it cannot list is refused");
    assert_eq!(error.code, specforge_ops::FILE_UNREADABLE, "{error:?}");
    assert!(
        error.message.contains(".specforge/facts-history"),
        "{error:?}"
    );
    assert_eq!(
        std::fs::read_to_string(p.root().join(&kept)).unwrap(),
        "archived first",
        "the archived store is not overwritten"
    );
    assert_eq!(std::fs::read(p.root().join(&live)).unwrap(), before);
    assert!(!p.root().join(Store::path("node", NEXT)).exists());
}
