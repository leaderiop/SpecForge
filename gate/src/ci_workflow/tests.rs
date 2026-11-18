use super::*;
use specforge_repo_root::repo_root;

mod parse;
#[cfg(unix)]
mod prune;

const CI: &str = ".github/workflows/ci.yml";
const BOOTSTRAP: &str = ".github/workflows/bootstrap.yml";

fn read(file: &str) -> String {
    std::fs::read_to_string(repo_root().join(file)).unwrap_or_else(|e| panic!("{file}: {e}"))
}

/// The script of the step `name` in job `id` of `file`, as the runner gets it.
fn script(file: &str, id: &str, name: &str) -> String {
    let workflow = read(file);
    let jobs = jobs(&workflow);
    let job = jobs
        .iter()
        .find(|job| job.id == id)
        .unwrap_or_else(|| panic!("{file}: no job `{id}`"));
    job.step(name)
        .unwrap_or_else(|| panic!("{file}: job `{id}` has no step `{name}`"))
        .script()
        .unwrap_or_else(|| {
            panic!("{file}: the step `{name}` of job `{id}` runs no `run: |` script")
        })
}

/// A one-job workflow: its timeout and budget as given, the clock first and the check last.
fn synthetic(timeout: &str, budget: &str) -> String {
    format!(
        "name: X\n\non:\n  push:\n\njobs:\n  only:\n    runs-on: ubuntu-latest\n    timeout-minutes: {timeout}\n    \
         env:\n      BUDGET_SECONDS: {budget}\n    steps:\n      - name: {CLOCK}\n        shell: bash\n        \
         run: |\n          date\n      - run: true\n      - name: {CHECK}\n        shell: bash\n        run: |\n          \
         true\n"
    )
}

/// Runs `script` as Actions runs a `shell: bash` step, with only `PATH` and `vars` in its environment.
#[cfg(unix)]
fn bash(script: &str, vars: &[(&str, &str)]) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("step.sh");
    std::fs::write(&file, script).unwrap();
    std::process::Command::new("bash")
        .args(["--noprofile", "--norc", "-eo", "pipefail"])
        .arg(&file)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .envs(vars.iter().copied())
        .output()
        .unwrap()
}

#[cfg(unix)]
fn seconds_ago(seconds: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    (now - seconds).to_string()
}

/// The shipped check, run by bash: a job that started 100 s ago fails a 10 s budget with an `::error`, and passes a
/// 1,000 s one, writing what it took to the step summary. Under `cfg(unix)`: it runs bash, and the gate runs on Linux.
#[cfg(unix)]
#[test]
fn the_budget_check_fails_a_job_past_its_budget() {
    let check = script(CI, "build", CHECK);
    let dir = tempfile::tempdir().unwrap();
    let summary = dir.path().join("summary.md");
    let summary = summary.to_str().unwrap();
    let t0 = seconds_ago(100);

    let past = bash(
        &check,
        &[
            ("BUDGET_T0", &t0),
            ("BUDGET_SECONDS", "10"),
            ("GITHUB_STEP_SUMMARY", summary),
        ],
    );
    let stdout = String::from_utf8_lossy(&past.stdout);
    assert!(
        !past.status.success(),
        "a job past its budget passed: {stdout}"
    );
    assert!(
        stdout.contains("::error title=Build-time budget::this job took")
            && stdout.contains("of its 10 s build-time budget (ADR 0375 D3)"),
        "{stdout}"
    );

    let within = bash(
        &check,
        &[
            ("BUDGET_T0", &t0),
            ("BUDGET_SECONDS", "1000"),
            ("GITHUB_STEP_SUMMARY", summary),
        ],
    );
    assert!(
        within.status.success(),
        "a job within its budget failed: {}{}",
        String::from_utf8_lossy(&within.stdout),
        String::from_utf8_lossy(&within.stderr)
    );
    let written = std::fs::read_to_string(summary).unwrap();
    assert!(
        written.contains("of its 1000 s build-time budget"),
        "{written}"
    );
}

/// With no clock, or a budget that is not a whole number, the check fails rather than passing on nothing.
#[cfg(unix)]
#[test]
fn the_budget_check_fails_a_job_with_no_clock_or_no_budget() {
    let check = script(CI, "build", CHECK);
    let dir = tempfile::tempdir().unwrap();
    let summary = dir.path().join("summary.md");
    let summary = summary.to_str().unwrap();
    let t0 = seconds_ago(1);
    let cases: [(&[(&str, &str)], &str); 2] = [
        (
            &[("BUDGET_SECONDS", "1000"), ("GITHUB_STEP_SUMMARY", summary)],
            "no clock",
        ),
        (
            &[
                ("BUDGET_T0", &t0),
                ("BUDGET_SECONDS", "ten"),
                ("GITHUB_STEP_SUMMARY", summary),
            ],
            "no budget",
        ),
    ];
    for (vars, says) in cases {
        let out = bash(&check, vars);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(!out.status.success(), "{says}: passed: {stdout}");
        assert!(
            stdout.contains("::error title=Build-time budget::") && stdout.contains(says),
            "{says}: {stdout}"
        );
    }
}

/// The clock step writes one line `BUDGET_T0=<seconds>` to `GITHUB_ENV`, which the later steps see.
#[cfg(unix)]
#[test]
fn the_clock_records_where_the_job_started() {
    let clock = script(CI, "build", CLOCK);
    let dir = tempfile::tempdir().unwrap();
    let env = dir.path().join("github_env");
    let out = bash(&clock, &[("GITHUB_ENV", env.to_str().unwrap())]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let written = std::fs::read_to_string(&env).unwrap();
    let lines: Vec<&str> = written.lines().collect();
    assert_eq!(lines.len(), 1, "{written}");
    let seconds = lines[0]
        .strip_prefix("BUDGET_T0=")
        .unwrap_or_else(|| panic!("{written}"));
    assert!(
        !seconds.is_empty() && seconds.bytes().all(|b| b.is_ascii_digit()),
        "{written}"
    );
}

/// A budget over `timeout-minutes * 60 - 300` is a finding naming the job and its cap; one at the cap is none.
#[test]
fn a_budget_over_its_cap_is_a_finding() {
    let over = findings("x.yml", &synthetic("10", "301"));
    assert_eq!(over.len(), 1, "{over:?}");
    assert!(
        over[0].contains("job `only`") && over[0].contains("cap of 300 s"),
        "{over:?}"
    );
    assert_eq!(
        findings("x.yml", &synthetic("10", "300")),
        Vec::<String>::new()
    );
}

/// Each part of a budget a job lacks is a finding: no budget, no timeout, a matrix entry with no budget, a clock that is
/// not the first step, a check that is not the last.
#[test]
fn each_missing_part_of_a_budget_is_a_finding() {
    let whole = synthetic("10", "300");
    let clock =
        format!("      - name: {CLOCK}\n        shell: bash\n        run: |\n          date\n");
    let cases = [
        (
            whole.replace("    env:\n      BUDGET_SECONDS: 300\n", ""),
            "has no BUDGET_SECONDS",
        ),
        (
            whole.replace("    timeout-minutes: 10\n", ""),
            "has no timeout-minutes",
        ),
        (
            whole
                .replace("300", "${{ matrix.budget }}")
                .replace(
                    "    env:",
                    "    strategy:\n      matrix:\n        include:\n          - os: a\n            budget: 60\n          \
                     - os: b\n    env:",
                ),
            "has a matrix entry `os: b` with no budget",
        ),
        (
            whole.replace(&format!("{clock}      - run: true\n"), &format!("      - run: true\n{clock}")),
            "does not start with the step",
        ),
        (whole.clone() + "      - run: false\n", "does not end with the step"),
    ];
    for (workflow, says) in cases {
        let found = findings("x.yml", &workflow);
        assert!(
            found.len() == 1 && found[0].contains("job `only`") && found[0].contains(says),
            "{says}: {found:?}\n{workflow}"
        );
    }
}

/// Every job of `ci.yml` and `bootstrap.yml` has a budget under its timeout, starts with the clock and ends with the
/// check, and each runs the same two scripts the tests above run on the `build` job (ADR 0375 D3).
#[test]
fn every_job_has_a_budget_under_its_timeout() {
    let (clock, check) = (script(CI, "build", CLOCK), script(CI, "build", CHECK));
    let mut ids = Vec::new();
    for file in [CI, BOOTSTRAP] {
        let workflow = read(file);
        assert_eq!(findings(file, &workflow), Vec::<String>::new());
        for job in jobs(&workflow) {
            for (name, want) in [(CLOCK, &clock), (CHECK, &check)] {
                assert_eq!(
                    job.step(name).and_then(|step| step.script()).as_ref(),
                    Some(want),
                    "{file}: job `{}` runs another `{name}`",
                    job.id
                );
            }
            ids.push(job.id.to_string());
        }
    }
    assert_eq!(
        ids,
        [
            "ci",
            "build",
            "tests",
            "effect-plane",
            "windows-subset",
            "dogfood",
            "cache-prune",
            "bootstrap"
        ]
    );
}

/// The timeouts are the decided ones: a job fits its budget by getting faster, never by a longer timeout.
#[test]
fn the_timeouts_are_the_decided_ones() {
    let decided = [
        ("ci", 40),
        ("build", 40),
        ("tests", 30),
        ("effect-plane", 30),
        ("windows-subset", 30),
        ("dogfood", 20),
        ("cache-prune", 10),
        ("bootstrap", 45),
    ];
    let mut found = Vec::new();
    for file in [CI, BOOTSTRAP] {
        let workflow = read(file);
        for job in jobs(&workflow) {
            found.push((job.id.to_string(), job.timeout_minutes()));
        }
    }
    let decided: Vec<(String, Option<u32>)> = decided
        .iter()
        .map(|(id, minutes)| (id.to_string(), Some(*minutes)))
        .collect();
    assert_eq!(found, decided);
}

/// The Windows job runs the named subset (ADR 0375 D4): the three filters as written, and line endings and long
/// paths set before the checkout.
#[test]
fn the_windows_leg_runs_the_named_subset() {
    let workflow = read(CI);
    let jobs = jobs(&workflow);
    let windows: Vec<&Job<'_>> = jobs
        .iter()
        .filter(|job| job.key("runs-on") == Some("windows-latest"))
        .collect();
    assert_eq!(windows.len(), 1, "one job runs on windows-latest");
    let steps = windows[0].steps();
    let runs: Vec<&str> = steps.iter().filter_map(|step| step.key("run")).collect();
    for filter in [
        "cargo nextest run --no-fail-fast -p gate -E 'test(/^checked_tree::/)'",
        "cargo nextest run --no-fail-fast -p specforge-component --lib -E 'test(/user_compile_cache/)'",
        "cargo nextest run --no-fail-fast -p xtask --bin absorb --test absorb_e2e",
    ] {
        assert!(runs.contains(&filter), "{filter} is not run: {runs:?}");
    }
    let checkout = steps
        .iter()
        .position(|step| step.key("uses") == Some("actions/checkout@v4"))
        .expect("the Windows job checks out");
    let config = steps.iter().position(|step| {
        step.key("shell") == Some("bash")
            && step.script().is_some_and(|script| {
                script.contains("git config --global core.autocrlf false\n")
                    && script.contains("git config --global core.longpaths true\n")
            })
    });
    assert!(
        config.is_some_and(|config| config < checkout),
        "no bash step sets core.autocrlf false and core.longpaths true before the checkout"
    );
}

/// The `tests` jobs restore no cache: only `cargo gate` compiles there, and the tests come from the archive (ADR 0375
/// D1).
#[test]
fn the_tests_jobs_restore_no_cache() {
    let workflow = read(CI);
    let jobs = jobs(&workflow);
    let tests = jobs.iter().find(|job| job.id == "tests").unwrap();
    assert!(!tests.steps().is_empty());
    assert!(
        !tests.steps().iter().any(Step::is_rust_cache),
        "the tests jobs restore a cache"
    );
}

/// Only `ci.yml` and the nightly `perf.yml` carry a cache, and `ci.yml` saves only on main: a pull request never saves
/// a cache main would restore, coverage's instrumented or release's tag-scoped caches never evict CI's, and no other
/// workflow keeps a restore-only step whose key nothing saves (ADR 0375 D1).
#[test]
fn only_ci_and_perf_save_a_cache() {
    let main = "${{ github.ref == 'refs/heads/main' }}";
    let dir = repo_root().join(".github/workflows");
    let mut files: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".yml") || name.ends_with(".yaml"))
        .collect();
    files.sort();
    assert!(
        files.contains(&"coverage.yml".to_string()) && files.contains(&"release.yml".to_string())
    );
    let mut ci_savers = 0;
    for name in &files {
        let file = format!(".github/workflows/{name}");
        let workflow = read(&file);
        let jobs = jobs(&workflow);
        // A workflow read as no job, or a job as no step, would pass every check below on nothing.
        assert!(!jobs.is_empty(), "{file}: the gate reads no job");
        for job in &jobs {
            assert!(
                !job.steps().is_empty(),
                "{file}: the gate reads no step of job `{}`",
                job.id
            );
        }
        if name == "perf.yml" {
            continue;
        }
        for job in &jobs {
            for step in job.steps().iter().filter(|step| step.is_rust_cache()) {
                assert_eq!(
                    name, "ci.yml",
                    "{file}: job `{}` restores a cache no workflow saves",
                    job.id
                );
                let save_if = step.with("save-if");
                if save_if == Some(main) {
                    ci_savers += 1;
                } else {
                    assert_eq!(
                        save_if,
                        Some("false"),
                        "{file}: a rust-cache step of job `{}` saves outside main",
                        job.id
                    );
                }
            }
        }
    }
    assert!(ci_savers > 0, "ci.yml saves no cache on main");
}

const PREFIXED: &str = "prefix-key: ${{ env.CACHE_PREFIX }}";
const ON_MAIN: &str = "save-if: ${{ github.ref == 'refs/heads/main' }}";
const RESTORE_ONLY: &str = "save-if: false";
const PREFIX_ENV: &str = "  CACHE_PREFIX: v1-rust\n";

/// A workflow whose env holds `env` and whose jobs each run one rust-cache step: a job is its id, the lines of its
/// `strategy:` (none when empty) and the step's `with:` inputs.
fn cached(env: &str, jobs: &[(&str, &str, &[&str])]) -> String {
    let mut text = format!("name: X\n\non:\n  push:\n\nenv:\n{env}\njobs:\n");
    for (id, strategy, with) in jobs {
        text.push_str(&format!("  {id}:\n    runs-on: ubuntu-latest\n"));
        if !strategy.is_empty() {
            text.push_str(&format!("    strategy:\n{strategy}"));
        }
        text.push_str("    steps:\n      - uses: Swatinem/rust-cache@v2\n        with:\n");
        for line in *with {
            text.push_str(&format!("          {line}\n"));
        }
    }
    text
}

/// A matrix whose entries each name `os` and, when given, `cache`.
fn matrix(entries: &[(&str, Option<&str>)]) -> String {
    let mut text = "      matrix:\n        include:\n".to_string();
    for (os, cache) in entries {
        text.push_str(&format!("          - os: {os}\n"));
        if let Some(cache) = cache {
            text.push_str(&format!("            cache: {cache}\n"));
        }
    }
    text
}

/// The `ci.yml` cache keys: one prefix in the workflow's env, read by every rust-cache step; every step keyed by a
/// `shared-key` alone; six savers, each its own key, saved only on main; `dogfood` restores the specs shard's key and
/// saves nothing (ADR 0375 D1).
#[test]
fn every_cache_step_is_keyed_once_under_the_workflow_prefix() {
    let workflow = read(CI);
    assert_eq!(key_findings(CI, &workflow), Vec::<String>::new());
    assert_eq!(workflow_env(&workflow, PREFIX), Some("v1-rust"));
    let (keys, _) = cache_keys(CI, &workflow);
    let savers: Vec<&str> = keys
        .iter()
        .filter(|key| key.saves)
        .map(|key| key.key.as_str())
        .collect();
    assert!(savers.len() >= 5, "{savers:?}");
    assert_eq!(
        savers,
        [
            "lint",
            "gsd-crates",
            "specs",
            "build",
            "effect-plane",
            "windows-subset"
        ]
    );
    let restored: Vec<(&str, &str)> = keys
        .iter()
        .filter(|key| !key.saves)
        .map(|key| (key.at.as_str(), key.key.as_str()))
        .collect();
    assert_eq!(restored, [("job `dogfood`", "specs")]);
}

/// A workflow keyed once under its prefix has no finding: a literal key over a matrix of runners is one key (rust-cache
/// adds the OS), a matrix key is one per entry, and a restore-only step may name any saver's key.
#[test]
fn a_workflow_keyed_once_under_its_prefix_has_no_finding() {
    let two = matrix(&[("a", Some("one")), ("b", Some("two"))]);
    let runners = matrix(&[("windows-latest", None), ("macos-latest", None)]);
    let workflow = cached(
        PREFIX_ENV,
        &[
            (
                "shards",
                &two,
                &[PREFIXED, "shared-key: ${{ matrix.cache }}", ON_MAIN],
            ),
            ("plane", &runners, &[PREFIXED, "shared-key: plane", ON_MAIN]),
            ("reader", "", &[PREFIXED, "shared-key: two", RESTORE_ONLY]),
        ],
    );
    assert_eq!(key_findings("x.yml", &workflow), Vec::<String>::new());
}

/// Two steps saving one key is a finding naming both, whether they are two jobs or two entries of one matrix.
#[test]
fn two_savers_of_one_key_are_a_finding_naming_both() {
    let jobs = cached(
        PREFIX_ENV,
        &[
            ("a", "", &[PREFIXED, "shared-key: specs", ON_MAIN]),
            ("b", "", &[PREFIXED, "shared-key: specs", ON_MAIN]),
        ],
    );
    let found = key_findings("x.yml", &jobs);
    assert!(
        found.len() == 1 && found[0].contains("`specs` is saved by job `a` and job `b`"),
        "{found:?}"
    );
    let entries = matrix(&[("a", Some("same")), ("b", Some("same"))]);
    let one_job = cached(
        PREFIX_ENV,
        &[(
            "m",
            &entries,
            &[PREFIXED, "shared-key: ${{ matrix.cache }}", ON_MAIN],
        )],
    );
    let found = key_findings("x.yml", &one_job);
    assert!(
        found.len() == 1
            && found[0]
                .contains("job `m` (matrix entry `os: a`) and job `m` (matrix entry `os: b`)"),
        "{found:?}"
    );
}

/// A restore-only step whose key no step saves is a finding: it would restore nothing, ever.
#[test]
fn a_restore_only_key_no_saver_holds_is_a_finding() {
    let workflow = cached(
        PREFIX_ENV,
        &[
            ("saver", "", &[PREFIXED, "shared-key: specs", ON_MAIN]),
            ("reader", "", &[PREFIXED, "shared-key: spec", RESTORE_ONLY]),
        ],
    );
    let found = key_findings("x.yml", &workflow);
    assert!(
        found.len() == 1
            && found[0].contains("job `reader` restores the cache key `spec`, which no step saves"),
        "{found:?}"
    );
}

/// A cache step off the workflow's prefix, naming a `key`, naming no `shared-key`, or a matrix key an entry lacks, is
/// a finding naming the job.
#[test]
fn a_cache_step_off_the_prefix_or_with_a_key_is_a_finding() {
    let lacking = matrix(&[("a", Some("one")), ("b", None)]);
    let cases: [(&str, &[&str], &str); 5] = [
        (
            "",
            &["shared-key: x", ON_MAIN],
            "does not read its prefix-key",
        ),
        (
            "",
            &["prefix-key: v1-rust", "shared-key: x", ON_MAIN],
            "does not read its prefix-key",
        ),
        (
            "",
            &[PREFIXED, "key: x", "shared-key: x", ON_MAIN],
            "names a `key`",
        ),
        ("", &[PREFIXED, ON_MAIN], "names no `shared-key`"),
        (
            &lacking,
            &[PREFIXED, "shared-key: ${{ matrix.cache }}", ON_MAIN],
            "has a matrix entry `os: b` with no cache",
        ),
    ];
    for (strategy, with, says) in cases {
        let found = key_findings("x.yml", &cached(PREFIX_ENV, &[("only", strategy, with)]));
        assert!(
            found.len() == 1 && found[0].contains("job `only`") && found[0].contains(says),
            "{says}: {found:?}"
        );
    }
}

/// A workflow with no `CACHE_PREFIX` in its env is a finding, though every step reads it.
#[test]
fn a_workflow_without_a_cache_prefix_is_a_finding() {
    let workflow = cached(
        "  CARGO_TERM_COLOR: always\n",
        &[("only", "", &[PREFIXED, "shared-key: x", ON_MAIN])],
    );
    let found = key_findings("x.yml", &workflow);
    assert!(
        found.len() == 1 && found[0].contains("the workflow's env has no CACHE_PREFIX"),
        "{found:?}"
    );
}

/// The one job that deletes caches runs only on a push (to main, the only branch `ci.yml` runs pushes for), after
/// every job that saves a cache, with `actions: write` and `contents: read` alone, and lists only main's entries
/// (ADR 0375 D2).
#[test]
fn the_prune_job_runs_only_on_mains_pushes_with_actions_write_alone() {
    let workflow = read(CI);
    assert!(
        workflow.contains("on:\n  push:\n    branches: [main]\n"),
        "ci.yml runs pushes for another branch than main"
    );
    let jobs = jobs(&workflow);
    let prune = jobs.iter().find(|job| job.id == "cache-prune").unwrap();
    assert_eq!(
        prune.key("if"),
        Some("github.event_name == 'push' && !cancelled()")
    );
    let permissions: Vec<&str> = prune
        .block("permissions")
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    assert_eq!(permissions, ["actions: write", "contents: read"]);
    let savers: Vec<&str> = jobs
        .iter()
        .filter(|job| {
            job.steps()
                .iter()
                .any(|step| step.is_rust_cache() && step.with("save-if") != Some("false"))
        })
        .map(|job| job.id)
        .collect();
    assert_eq!(
        prune.key("needs"),
        Some(format!("[{}]", savers.join(", ")).as_str())
    );
    let steps = prune.steps();
    assert!(
        !steps.iter().any(|step| step
            .key("uses")
            .is_some_and(|uses| uses.starts_with("actions/checkout@"))),
        "the prune job checks out the repository"
    );
    let delete = steps
        .iter()
        .find_map(|step| {
            step.script()
                .filter(|script| script.contains("gh cache delete"))
        })
        .expect("the prune job deletes nothing");
    assert!(
        delete.contains("gh cache list --ref refs/heads/main "),
        "{delete}"
    );
}
