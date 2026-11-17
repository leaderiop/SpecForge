use super::*;
use specforge_repo_root::repo_root;

const CI: &str = ".github/workflows/ci.yml";

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

/// The `build` job has a budget under its timeout, starts with the clock and ends with the check (ADR 0375 D3).
#[test]
fn the_build_job_has_a_budget_under_its_timeout() {
    let workflow = read(CI);
    let jobs = jobs(&workflow);
    let build = jobs.iter().find(|job| job.id == "build").unwrap();
    assert_eq!(job_findings(CI, build), Vec::<String>::new());
    assert_eq!(build.budgets(), Ok(vec![1680]));
}
