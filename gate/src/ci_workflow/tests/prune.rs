//! Which cache entries `cache-prune` deletes (ADR 0375 D2): its shipped delete step, run by bash against a `gh` that
//! lists a fixture through the job's own `STALE_CACHES` jq program and records each delete. Compiled only under
//! `cfg(unix)`: it runs bash and jq, and the gate runs on Linux, where the runner has both. A machine without jq fails
//! the test, naming jq: CONTRIBUTING.md lists it beside z3.
//!
//! gh evaluates `--jq` with its embedded gojq (Go's RE2 regexes), and this test with jq (Oniguruma). `STALE_CACHES`
//! uses only what both read the same: `group_by`, `sub` with a character-class regex, `sort_by`, a slice and
//! `flatten`. A program that needs an Oniguruma-only feature (look-around, `\h`) would pass here and fail on the runner.

use super::*;

const PRUNE: &str = "cache-prune";
const DELETE: &str = "Delete the cache entries a newer one of their group supersedes";

/// A `gh` as a bash function, defined ahead of the step's script, so nothing runs from the temp dir (a `/tmp` mounted
/// `noexec` cannot fail the test). It answers `cache list` with `$GH_FIXTURE/caches.json`, filtered by the `--jq`
/// program it is given, records the `--ref` it was asked for in `$GH_FIXTURE/ref`, and appends each
/// `cache delete <id>` to `$GH_FIXTURE/deleted`.
const FAKE_GH: &str = r#"gh() {
  case "$1 $2" in
    "cache list")
      shift 2
      local program=""
      while [ $# -gt 0 ]; do
        case "$1" in
          --jq) program=$2; shift 2 ;;
          --ref) printf '%s' "$2" > "$GH_FIXTURE/ref"; shift 2 ;;
          *) shift ;;
        esac
      done
      jq -r "$program" "$GH_FIXTURE/caches.json"
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

/// Two generations of one key on Linux, one of them listed first; one generation of that key on Windows; perf's
/// unprefixed key in two generations; a key with no hashes, alone. Only each group's older entries go: 1 and 4.
#[test]
fn the_prune_step_deletes_only_what_a_newer_entry_of_its_group_supersedes() {
    let jq = std::process::Command::new("jq").arg("--version").output();
    assert!(
        jq.is_ok_and(|out| out.status.success()),
        "the prune test runs jq: install jq (`apt-get install jq`, `brew install jq`), as CONTRIBUTING.md lists"
    );
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
    let run = bash(
        &format!("{FAKE_GH}{delete}"),
        &[
            ("GH_FIXTURE", dir.path().to_str().unwrap()),
            ("STALE_CACHES", program),
        ],
    );
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
