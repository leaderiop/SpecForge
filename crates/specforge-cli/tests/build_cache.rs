//! `specforge check --cache`: the build cache (`specforge-cache.json`) is
//! written only on request, only by a check that passes, and always the
//! same bytes for the same sources.

use specforge_test_macros::test as specforge_test;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

const CACHE: &str = "specforge-cache.json";

fn project(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (path, content) in files {
        fs::write(dir.path().join(path), content).unwrap();
    }
    dir
}

fn check(dir: &Path, args: &[&str]) -> assert_cmd::assert::Assert {
    assert_cmd::cargo_bin_cmd!("specforge")
        .arg("check")
        .arg(dir)
        .args(args)
        .assert()
}

const SOURCES: &[(&str, &str)] = &[
    (
        "b.spec",
        "feature zeta \"Z\" {\n  status in_progress\n}\n\nbehavior beta \"B\" {\n  contract \"no status\"\n}\n",
    ),
    (
        "a.spec",
        "feature alpha \"A\" {\n  status done\n}\n\nmilestone mid \"M\" {\n  status \"planned\"\n}\n",
    ),
];

#[specforge_test(
    behavior = "write_build_cache",
    verify = "check --cache records each entity's kind and status"
)]
fn check_cache_records_kinds_and_statuses() {
    let dir = project(SOURCES);

    check(dir.path(), &["--cache"]).success();

    let written = fs::read_to_string(dir.path().join(CACHE)).unwrap();
    assert_eq!(
        written,
        r#"{
  "format": 1,
  "statuses": {
    "alpha": {
      "kind": "feature",
      "status": "done"
    },
    "mid": {
      "kind": "milestone",
      "status": "planned"
    },
    "zeta": {
      "kind": "feature",
      "status": "in_progress"
    }
  }
}
"#
    );
}

#[specforge_test(
    behavior = "write_build_cache",
    verify = "the cache file is deterministic"
)]
fn the_cache_file_is_deterministic() {
    let dir = project(SOURCES);

    check(dir.path(), &["--cache"]).success();
    let first = fs::read(dir.path().join(CACHE)).unwrap();
    for _ in 0..3 {
        check(dir.path(), &["--cache"]).success();
        assert_eq!(fs::read(dir.path().join(CACHE)).unwrap(), first);
    }
    // No staging file is left beside it.
    let names: Vec<String> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(names.iter().all(|n| !n.ends_with(".tmp")), "{names:?}");
}

#[specforge_test(
    behavior = "write_build_cache",
    verify = "check without --cache never writes the cache"
)]
fn check_without_cache_never_writes_it() {
    let dir = project(SOURCES);

    check(dir.path(), &[]).success();
    assert!(!dir.path().join(CACHE).exists());

    // An existing cache is read, never rewritten, by a plain check.
    fs::write(dir.path().join(CACHE), "sentinel").unwrap();
    check(dir.path(), &[]).success();
    assert_eq!(
        fs::read_to_string(dir.path().join(CACHE)).unwrap(),
        "sentinel"
    );
}

#[specforge_test(
    behavior = "write_build_cache",
    verify = "check --cache with errors leaves the cache untouched"
)]
fn check_cache_with_errors_leaves_the_cache_untouched() {
    let dir = project(&[(
        "a.spec",
        "feature alpha \"A\" {\n  status done\n  behaviors [missing]\n}\n",
    )]);
    let previous = "{\"format\": 1, \"statuses\": {}}\n";
    fs::write(dir.path().join(CACHE), previous).unwrap();

    check(dir.path(), &["--cache"])
        .code(1)
        .stderr(predicates::str::contains(
            "specforge-cache.json not written",
        ));
    assert_eq!(
        fs::read_to_string(dir.path().join(CACHE)).unwrap(),
        previous
    );

    // Nor is a first cache written by a failing check.
    fs::remove_file(dir.path().join(CACHE)).unwrap();
    check(dir.path(), &["--cache"]).code(1);
    assert!(!dir.path().join(CACHE).exists());
}
