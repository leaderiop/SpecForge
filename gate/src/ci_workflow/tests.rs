use super::*;
use specforge_repo_root::repo_root;

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
