//! Which cache entries `cache-prune` deletes (ADR 0375 D2): its shipped delete step, run by bash against a `gh` that
//! lists a fixture through the job's own `STALE_CACHES` jq program and records each delete. Compiled only under
//! `cfg(unix)`: it runs bash and jq, and the gate runs on Linux, where the runner has both.

use super::*;
use std::os::unix::fs::PermissionsExt;

const PRUNE: &str = "cache-prune";
const DELETE: &str = "Delete the cache entries a newer one of their group supersedes";

/// A `gh` that answers `cache list` with `caches.json` in its own directory, filtered by the `--jq` program it is
/// given, and appends each `cache delete <id>` to `deleted`. It records the `--ref` it was asked for in `ref`.
const FAKE_GH: &str = r#"#!/usr/bin/env bash
set -eo pipefail
here=$(dirname "$0")
case "$1 $2" in
  "cache list")
    shift 2
    while [ $# -gt 0 ]; do
      case "$1" in
        --jq) program=$2; shift 2 ;;
        --ref) printf '%s' "$2" > "$here/ref"; shift 2 ;;
        *) shift ;;
      esac
    done
    jq -r "$program" "$here/caches.json"
    ;;
  "cache delete") echo "$3" >> "$here/deleted" ;;
  *) echo "unexpected gh $*" >&2; exit 2 ;;
esac
"#;

/// One `gh cache list --json id,key,createdAt` entry.
fn entry(id: u32, key: &str, created: &str) -> String {
    format!(r#"{{"id":{id},"key":"{key}","createdAt":"{created}"}}"#)
}

/// Two generations of one key on Linux, one of them listed first; one generation of that key on Windows; perf's
/// unprefixed key in two generations; a key with no hashes, alone. Only each group's older entries go: 1 and 4.
#[test]
fn the_prune_step_deletes_only_what_a_newer_entry_of_its_group_supersedes() {
    let workflow = read(CI);
    let jobs = jobs(&workflow);
    let prune = jobs.iter().find(|job| job.id == PRUNE).unwrap();
    let program = prune
        .env("STALE_CACHES")
        .expect("the prune job has no STALE_CACHES");
    let delete = script(CI, PRUNE, DELETE);

    let dir = tempfile::tempdir().unwrap();
    let caches = [
        entry(
            2,
            "v1-rust-build-Linux-x64-bbbbbbbb-22222222",
            "2026-11-02T00:00:00Z",
        ),
        entry(
            1,
            "v1-rust-build-Linux-x64-aaaaaaaa-11111111",
            "2026-11-01T00:00:00Z",
        ),
        entry(
            3,
            "v1-rust-build-Windows-x64-aaaaaaaa-11111111",
            "2026-11-01T00:00:00Z",
        ),
        entry(
            4,
            "v0-rust-perf-Linux-x64-aaaaaaaa-11111111",
            "2026-10-01T00:00:00Z",
        ),
        entry(
            5,
            "v0-rust-perf-Linux-x64-aaaaaaaa-33333333",
            "2026-10-02T00:00:00Z",
        ),
        entry(6, "some-other-cache", "2026-09-01T00:00:00Z"),
    ];
    std::fs::write(
        dir.path().join("caches.json"),
        format!("[{}]", caches.join(",")),
    )
    .unwrap();
    let gh = dir.path().join("gh");
    std::fs::write(&gh, FAKE_GH).unwrap();
    std::fs::set_permissions(&gh, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        dir.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let run = bash(&delete, &[("PATH", &path), ("STALE_CACHES", program)]);
    assert!(
        run.status.success(),
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("ref")).unwrap(),
        "refs/heads/main"
    );
    let mut deleted: Vec<String> = std::fs::read_to_string(dir.path().join("deleted"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect();
    deleted.sort();
    assert_eq!(deleted, ["1", "4"]);
}
