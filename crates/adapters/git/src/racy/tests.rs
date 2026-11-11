//! A fix-up that fails once its operation landed is said, and the operation's value answered.

#![allow(clippy::unwrap_used, reason = "a test's unwrap is its assertion")]

use std::fs::File;
use std::time::{Duration, UNIX_EPOCH};

use specforge_effects::git::{Located, ResetMode, Sha};

use crate::testing::Repo;

/// `rel` (the index when `None`) stamped `nanos` into one fixed second.
fn stamp(repo: &Repo, rel: Option<&str>, nanos: u64) {
    let path = rel.map_or_else(|| repo.git_dir().join("index"), |rel| repo.dir().join(rel));
    File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(UNIX_EPOCH + Duration::from_secs(1_700_000_000) + Duration::from_nanos(nanos))
        .unwrap();
}

#[test]
fn a_fix_up_that_fails_after_its_operation_is_said_and_the_value_kept() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.config("[core]\n\ttrustctime = false");
    repo.write("a.txt", "k\n");
    stamp(&repo, Some("a.txt"), 100);
    repo.commit_all("first");
    stamp(&repo, None, 500);
    // rewritten to the same size in the second it was staged: racy, so the fix-up has an entry to smudge
    repo.write("a.txt", "l\n");
    stamp(&repo, Some("a.txt"), 900);
    let Located::Repo(place) = crate::locate::locate(dir.path()) else {
        panic!("a repository");
    };
    let head = Sha::new(repo.sha("HEAD"));
    let mut ran = false;
    let (value, unkept) = super::kept_with(
        &place,
        || {
            ran = true;
            crate::merging::reset(&place, &head, ResetMode::Mixed).map(|()| 7)
        },
        |_| Err(git2::Error::from_str("the disk is full")),
    )
    .unwrap();
    assert!(ran);
    assert_eq!(value, 7, "the operation's value");
    let unkept = unkept.unwrap();
    assert!(
        unkept.starts_with("the operation completed") && unkept.contains("the disk is full"),
        "{unkept}"
    );
    // a fix-up that writes answers nothing to say
    let (_, said) = super::kept_with(&place, || Ok(()), |index| index.write()).unwrap();
    assert_eq!(said, None);
}
