//! The archive of superseded fact stores (ADR 0376 D4): a collect moves each store of a collector it supersedes to
//! `.specforge/facts-history/<seq>-<key>/<collector>.json` under a fresh sequence number, and keeps that collector's
//! newest `facts.retain`; a listing skips any directory that is no archive's.

use std::path::Path;

use specforge_project::facts::retain::{Archived, Retirement, archived, parse_name, retire};
use specforge_registry::PlainDisk;

use super::*;

const OLD: &str = "1111111111111111111111111111111111111111";
const OLDER: &str = "2222222222222222222222222222222222222222";
const NEW: &str = "3333333333333333333333333333333333333333";

/// `.specforge/facts/<key>/<collector>.json` holding `{}` under `root`.
fn live(root: &Path, collector: &str, key: &str) {
    write(root, &format!(".specforge/facts/{key}/{collector}.json"));
}

/// `.specforge/facts-history/<dir>/<collector>.json` holding `{}` under `root`.
fn archive(root: &Path, dir: &str, collector: &str) {
    write(
        root,
        &format!(".specforge/facts-history/{dir}/{collector}.json"),
    );
}

fn write(root: &Path, path: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "{}").unwrap();
}

fn history(dir: &str, collector: &str) -> String {
    format!(".specforge/facts-history/{dir}/{collector}.json")
}

fn facts(key: &str, collector: &str) -> String {
    format!(".specforge/facts/{key}/{collector}.json")
}

/// A root holding `c`'s live stores under [`OLDER`], [`OLD`] and [`NEW`], `c`'s archive `000006-<key>` and `d`'s
/// archive `000005-<key>`.
fn archiving() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    for key in [OLDER, OLD, NEW] {
        live(root, "c", key);
    }
    archive(root, &format!("000006-{OLDER}"), "c");
    archive(root, &format!("000005-{OLDER}"), "d");
    dir
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_retire_moves_each_superseded_store_under_a_fresh_seq_after_the_largest() {
    let dir = archiving();
    let retired = retire(&PlainDisk, dir.path(), "c", NEW, 8);
    assert_eq!(
        retired,
        Retirement {
            moves: vec![
                (facts(OLD, "c"), history(&format!("000007-{OLD}"), "c")),
                (facts(OLDER, "c"), history(&format!("000008-{OLDER}"), "c")),
            ],
            pruned: Vec::new(),
        }
    );
    // a retire only says what to do: nothing moved yet
    assert!(dir.path().join(facts(OLD, "c")).is_file());
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_retire_prunes_the_collectors_archives_beyond_retain_oldest_first() {
    let dir = archiving();
    let retired = retire(&PlainDisk, dir.path(), "c", NEW, 2);
    assert_eq!(retired.moves.len(), 2, "{retired:?}");
    assert_eq!(
        retired.pruned,
        [history(&format!("000006-{OLDER}"), "c")],
        "d's archive is not c's to prune"
    );
    // retain 1: the store the bound leaves no room for is removed where it is, never archived first
    let retired = retire(&PlainDisk, dir.path(), "c", NEW, 1);
    assert_eq!(
        retired,
        Retirement {
            moves: vec![(facts(OLDER, "c"), history(&format!("000008-{OLDER}"), "c"))],
            pruned: vec![history(&format!("000006-{OLDER}"), "c"), facts(OLD, "c")],
        }
    );
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_retire_at_the_bound_of_100_keeps_100() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    for seq in 1..=100 {
        archive(root, &format!("{seq:06}-{OLDER}"), "c");
    }
    live(root, "c", OLD);
    let retired = retire(&PlainDisk, root, "c", NEW, 100);
    assert_eq!(
        retired.moves,
        [(facts(OLD, "c"), history(&format!("000101-{OLD}"), "c"))]
    );
    assert_eq!(retired.pruned, [history(&format!("000001-{OLDER}"), "c")]);
    // with nothing superseded, 100 archives are within the bound
    let none = retire(&PlainDisk, root, "c", OLD, 100);
    assert_eq!(none, Retirement::default());
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_retire_with_nothing_superseded_or_a_name_that_is_no_stores_does_nothing() {
    let dir = archiving();
    let root = dir.path();
    assert_eq!(retire(&PlainDisk, root, "e", NEW, 8), Retirement::default());
    for (collector, key) in [("../c", NEW), (".c", NEW), ("c", "../x"), ("c", "")] {
        assert_eq!(
            retire(&PlainDisk, root, collector, key, 8),
            Retirement::default(),
            "{collector} {key}"
        );
    }
}

#[specforge_test(requirement = "FEATURE-142")]
fn an_archive_name_is_six_digits_and_a_store_key() {
    let dirty = format!("{OLD}-dirty");
    assert_eq!(parse_name(&format!("000007-{OLD}")), Some((7, OLD)));
    assert_eq!(
        parse_name(&format!("123456-{dirty}")),
        Some((123456, dirty.as_str()))
    );
    assert_eq!(parse_name("000000-worktree"), Some((0, "worktree")));
    let sha256 = "a".repeat(64);
    assert_eq!(
        parse_name(&format!("000010-{sha256}")),
        Some((10, sha256.as_str()))
    );
    for name in [
        format!("00007-{OLD}"),
        format!("0000007-{OLD}"),
        format!("00000a-{OLD}"),
        format!("+00007-{OLD}"),
        "000007-".to_string(),
        "000007-../x".to_string(),
        "000007-Worktree".to_string(),
        format!("000007-{}", &OLD[..39]),
        format!("000007_{OLD}"),
        "000007".to_string(),
    ] {
        assert_eq!(parse_name(&name), None, "{name}");
    }
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_listing_keeps_the_collectors_archives_newest_first_and_skips_the_rest() {
    let dir = archiving();
    let root = dir.path();
    archive(root, &format!("000011-{OLD}"), "c");
    archive(root, &format!("00007-{OLD}"), "c");
    archive(root, "000009-nothex", "c");
    archive(root, &format!("abcdef-{OLD}"), "c");
    // a directory a crash left empty, and one that holds another collector's store only
    fs::create_dir_all(root.join(format!(".specforge/facts-history/000012-{OLD}"))).unwrap();
    archive(root, &format!("000013-{OLD}"), "d");
    assert_eq!(
        archived(&PlainDisk, root, "c"),
        [
            Archived {
                seq: 11,
                key: OLD.to_string(),
                path: history(&format!("000011-{OLD}"), "c"),
            },
            Archived {
                seq: 6,
                key: OLDER.to_string(),
                path: history(&format!("000006-{OLDER}"), "c"),
            },
        ]
    );
    // the empty directory still counts for the next seq: a fresh one is never a directory already there
    let retired = retire(&PlainDisk, root, "c", NEW, 8);
    assert_eq!(retired.moves[0].1, history(&format!("000014-{OLD}"), "c"));
}
