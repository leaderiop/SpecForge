//! Which cache entries `cache-prune` deletes (ADR 0375 D2): its shipped delete step, run by bash against a `gh` that
//! lists a fixture and records each delete, the step reading the list with the job's own `STALE_CACHES` jq program.
//! Compiled only under `cfg(unix)`: it runs bash and jq, and the gate runs on Linux, where the runner has both. A
//! machine without jq fails the test, naming jq: CONTRIBUTING.md lists it beside z3. The step runs `STALE_CACHES` with
//! the runner's jq, not gh's embedded gojq, so the test runs the program with the implementation CI runs.
//!
//! The step runs with an empty directory as its `PATH`, so the fake is the only `gh` it can reach: a `command gh`,
//! `env gh`, `xargs gh` or `exec gh` finds no program and fails the test, and never runs a real `gh` with the
//! machine's credentials. jq is reached the same way, through a function that calls it by the path the test found.

use super::*;

const PRUNE: &str = "cache-prune";
const DELETE: &str = "Delete the cache entries a newer one of their group supersedes";

/// A `gh` and a `jq` as bash functions, defined ahead of the step's script, so nothing runs from the temp dir (a `/tmp`
/// mounted `noexec` cannot fail the test). `gh` answers `cache list` with the first `--limit` entries of
/// `$GH_FIXTURE/caches.json`, records the `--ref` and the `--limit` it was asked for in `$GH_FIXTURE/ref` and
/// `$GH_FIXTURE/limit`, and appends each `cache delete <id>` to `$GH_FIXTURE/deleted`. `jq` is `$JQ`, an absolute
/// path: the step's `PATH` holds nothing.
const FAKES: &str = r#"jq() { "$JQ" "$@"; }
gh() {
  case "$1 $2" in
    "cache list")
      shift 2
      local limit=30
      while [ $# -gt 0 ]; do
        case "$1" in
          --ref) printf '%s' "$2" > "$GH_FIXTURE/ref"; shift 2 ;;
          --limit) limit=$2; printf '%s' "$2" > "$GH_FIXTURE/limit"; shift 2 ;;
          --json) shift 2 ;;
          *) echo "unexpected gh cache list $1" >&2; return 2 ;;
        esac
      done
      jq ".[:$limit]" "$GH_FIXTURE/caches.json"
      ;;
    "cache delete") echo "$3" >> "$GH_FIXTURE/deleted" ;;
    *) echo "unexpected gh $*" >&2; return 2 ;;
  esac
}
"#;

/// One `gh cache list --json id,key,createdAt` entry.
fn entry(id: u32, key: &str, created: &str) -> String {
    format!(r#"{{"id":{id},"key":"{key}","createdAt":"{created}"}}"#)
}

/// What one run of the shipped delete step did: its output, the `--ref` and `--limit` it listed with, and the ids it
/// deleted, sorted.
struct Pruned {
    run: std::process::Output,
    listed_ref: String,
    limit: String,
    deleted: Vec<String>,
}

/// Runs the shipped delete step over six entries, its listing limited to `limit` (the job's own when `None`): two
/// generations of one key on Linux, the older listed second; one generation of that key on Windows; perf's unprefixed
/// key in two generations; a key with no hashes, alone. Only each group's older entries are stale: 1 and 4.
fn prune(limit: Option<&str>) -> Pruned {
    let jq = on_path("jq").expect(
        "the prune test runs jq: install jq (`apt-get install jq`, `brew install jq`), as CONTRIBUTING.md lists",
    );
    let workflow = read(CI);
    let jobs = jobs(&workflow);
    let job = jobs.iter().find(|job| job.id == PRUNE).unwrap();
    let program = job
        .env("STALE_CACHES")
        .expect("the prune job has no STALE_CACHES");
    let own_limit = job
        .env("CACHE_LIST_LIMIT")
        .expect("the prune job has no CACHE_LIST_LIMIT");
    let delete = script(CI, PRUNE, DELETE);

    let dir = tempfile::tempdir().unwrap();
    let no_programs = tempfile::tempdir().unwrap();
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
    let run = bash(
        &format!("{FAKES}{delete}"),
        &[
            ("PATH", no_programs.path().to_str().unwrap()),
            ("JQ", jq.to_str().unwrap()),
            ("GH_FIXTURE", dir.path().to_str().unwrap()),
            ("STALE_CACHES", &program),
            ("CACHE_LIST_LIMIT", limit.unwrap_or(&own_limit)),
        ],
    );
    let read = |name: &str| std::fs::read_to_string(dir.path().join(name)).unwrap_or_default();
    let mut deleted: Vec<String> = read("deleted").lines().map(str::to_string).collect();
    deleted.sort();
    Pruned {
        listed_ref: read("ref"),
        limit: read("limit"),
        deleted,
        run,
    }
}

fn output(run: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    )
}

/// Main's entries, listed under the job's own limit, lose each group's older entries and nothing else.
#[test]
fn the_prune_step_deletes_only_what_a_newer_entry_of_its_group_supersedes() {
    let pruned = prune(None);
    assert!(pruned.run.status.success(), "{}", output(&pruned.run));
    assert_eq!(pruned.listed_ref, "refs/heads/main");
    let workflow = read(CI);
    let jobs = jobs(&workflow);
    let job = jobs.iter().find(|job| job.id == PRUNE).unwrap();
    assert_eq!(
        Some(pruned.limit.as_str()),
        job.env("CACHE_LIST_LIMIT").as_deref()
    );
    assert_eq!(pruned.deleted, ["1", "4"]);
}

/// A listing that reaches its limit may have stopped short of main's entries: the step deletes what it read
/// supersedes, then fails the job, so a cut list is never taken for the whole.
#[test]
fn a_listing_that_reaches_its_limit_fails_the_prune_step() {
    let pruned = prune(Some("5"));
    assert!(!pruned.run.status.success(), "{}", output(&pruned.run));
    assert_eq!(pruned.limit, "5");
    assert!(
        String::from_utf8_lossy(&pruned.run.stdout)
            .contains("::error title=cache-prune::the listing read 5 entries, its limit"),
        "{}",
        output(&pruned.run)
    );
    assert_eq!(pruned.deleted, ["1", "4"]);
}
