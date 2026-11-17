use super::*;
use specforge_test_macros::test as specforge_test;
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;

/// A machine running real programs.
fn real_machine() -> ProcessMachine {
    ProcessMachine::new(Arc::new(specforge_process::Runner))
}

/// Run `argv` in `dir` with `report` as its report location.
fn run(
    argv: &[String],
    dir: &Path,
    report: &Path,
    capture: Option<&mut dyn Write>,
    output: RunnerOutput,
) -> Result<Ran, RunFailure> {
    real_machine().run(
        &CollectorCommand {
            argv,
            dir,
            report,
            report_in_env: true,
            output,
        },
        capture,
    )
}

#[test]
fn the_process_machine_keeps_the_port_contract() {
    let dir = tempfile::tempdir().unwrap();
    // `$CARGO` is set by cargo for every test run; a binary run by hand has none.
    if let Some(cargo) = std::env::var_os("CARGO") {
        let argv = vec![cargo.to_string_lossy().into_owned(), "--version".into()];
        let mut capture = Vec::new();
        let ran = run(
            &argv,
            dir.path(),
            &dir.path().join("r.json"),
            Some(&mut capture),
            RunnerOutput::Discard,
        )
        .unwrap();
        assert_eq!(ran.exit_code, Some(0));
        assert!(String::from_utf8(capture).unwrap().starts_with("cargo "));
        assert!(ran.started <= SystemTime::now());
    }
    let missing = vec!["specforge-no-such-program-here".to_string()];
    let failure = run(
        &missing,
        dir.path(),
        &dir.path().join("r.json"),
        None,
        RunnerOutput::Discard,
    )
    .unwrap_err();
    assert!(matches!(failure, RunFailure::NotFound));
}

#[cfg(unix)]
#[test]
fn run_sets_the_report_env_in_the_directory_and_dates_its_files_after_the_start() {
    let dir = tempfile::tempdir().unwrap();
    let report = dir.path().join("r.json");
    let argv = vec![
        "sh".to_string(),
        "-c".into(),
        format!(
            "test \"$(pwd -P)\" = \"$(cd {} && pwd -P)\" && printf fresh > \"${REPORT_ENV}\"",
            dir.path().display()
        ),
    ];
    let ran = run(&argv, dir.path(), &report, None, RunnerOutput::Discard).unwrap();
    assert_eq!(ran.exit_code, Some(0));
    assert_eq!(std::fs::read_to_string(&report).unwrap(), "fresh");
    let written = std::fs::metadata(&report).unwrap().modified().unwrap();
    assert!(written >= ran.started, "{written:?} < {:?}", ran.started);
}

#[cfg(unix)]
#[specforge_test(requirement = "FEATURE-128")]
fn a_captured_stdout_is_kept_in_every_output_mode() {
    let dir = tempfile::tempdir().unwrap();
    let argv = vec!["sh".to_string(), "-c".into(), "echo 'test a ... ok'".into()];
    for mode in [
        RunnerOutput::Discard,
        RunnerOutput::Stderr,
        RunnerOutput::Inherit,
    ] {
        let mut capture = Vec::new();
        let ran = run(
            &argv,
            dir.path(),
            &dir.path().join("out"),
            Some(&mut capture),
            mode,
        )
        .unwrap();
        assert_eq!(ran.exit_code, Some(0));
        assert_eq!(String::from_utf8(capture).unwrap(), "test a ... ok\n");
    }
}

#[cfg(unix)]
#[specforge_test(requirement = "FEATURE-128")]
fn a_command_past_the_deadline_is_stopped() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let (tx, rx) = std::sync::mpsc::channel();
    // The run is left to finish on its thread when the pin fails.
    std::thread::spawn(move || {
        let argv = vec!["sh".to_string(), "-c".into(), "sleep 30".into()];
        let ran = real_machine()
            .with_deadline(std::time::Duration::from_secs(1))
            .run(
                &CollectorCommand {
                    argv: &argv,
                    dir: &root,
                    report: &root.join("r.json"),
                    report_in_env: true,
                    output: RunnerOutput::Discard,
                },
                None,
            );
        let _ = tx.send(matches!(ran, Err(RunFailure::Stopped { .. })));
    });
    let stopped = rx.recv_timeout(std::time::Duration::from_secs(5));
    assert_eq!(stopped, Ok(true), "the command outlived its deadline");
}

/// A capture that cannot be written: a full disk.
#[cfg(unix)]
struct Full;

#[cfg(unix)]
impl Write for Full {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("no space left"))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(unix)]
#[test]
fn a_capture_that_cannot_be_written_is_a_capture_failure() {
    let dir = tempfile::tempdir().unwrap();
    let argv = vec!["sh".to_string(), "-c".into(), "echo hi".into()];
    let failure = run(
        &argv,
        dir.path(),
        &dir.path().join("r.json"),
        Some(&mut Full),
        RunnerOutput::Discard,
    )
    .unwrap_err();
    assert!(
        matches!(&failure, RunFailure::Capture(e) if e.to_string() == "no space left"),
        "{failure:?}"
    );
}

#[cfg(unix)]
#[test]
fn a_stopped_command_says_why_in_the_class_words() {
    let dir = tempfile::tempdir().unwrap();
    let argv = vec!["sh".to_string(), "-c".into(), "sleep 30".into()];
    let ran = real_machine()
        .with_deadline(std::time::Duration::from_secs(1))
        .run(
            &CollectorCommand {
                argv: &argv,
                dir: dir.path(),
                report: &dir.path().join("r.json"),
                report_in_env: true,
                output: RunnerOutput::Discard,
            },
            None,
        );
    let Err(RunFailure::Stopped { why }) = ran else {
        panic!("stopped: {ran:?}")
    };
    assert_eq!(why, "did not finish before its deadline (1 s)");
}

#[cfg(windows)]
#[specforge_test(requirement = "FEATURE-128")]
#[expect(
    unsafe_code,
    reason = "set_var before any thread of this test reads the environment; the test runs in its own process under nextest"
)]
fn a_cmd_shim_on_path_runs() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("shim.cmd"), "@exit /b 3\r\n").unwrap();
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = vec![dir.path().to_path_buf()];
    paths.extend(std::env::split_paths(&path));
    // SAFETY: no other thread of this test reads the environment.
    unsafe { std::env::set_var("PATH", std::env::join_paths(paths).unwrap()) };
    let argv = vec!["shim".to_string()];
    let ran = run(
        &argv,
        dir.path(),
        &dir.path().join("r.json"),
        None,
        RunnerOutput::Discard,
    )
    .unwrap();
    assert_eq!(ran.exit_code, Some(3));
}

#[cfg(unix)]
#[test]
#[expect(
    unsafe_code,
    reason = "set_var before any thread of this test reads the environment; the test runs in its own process under nextest"
)]
fn a_collector_command_never_receives_a_credential() {
    // SAFETY: no other thread of this test reads the environment.
    unsafe {
        std::env::set_var("CARGO_REGISTRY_TOKEN", "withheld");
        std::env::set_var("CARGO_REGISTRIES_X_TOKEN", "withheld");
        std::env::set_var("CARGO_REGISTRIES_X_CREDENTIAL_PROVIDER", "cargo:token");
        std::env::set_var("CARGO_HTTP_PROXY", "http://user:pass@proxy.test:8080");
        std::env::set_var("HTTPS_PROXY", "http://proxy.test:3128");
        std::env::set_var("CARGO_SPECFORGE_PROBE", "passed");
        std::env::set_var("PGPASSWORD", "withheld");
        std::env::set_var("MYSQL_PWD", "withheld");
        std::env::set_var("GH_PAT", "withheld");
        std::env::set_var("ALL_PROXY", "user:pass@proxy.test:1080");
        std::env::set_var("DATABASE_URL", "postgres://user:pa#ss@db.test/x");
        std::env::set_var("OLDPWD", "/probe/before");
    }
    let dir = tempfile::tempdir().unwrap();
    let mut capture = Vec::new();
    let ran = run(
        &["env".to_string()],
        dir.path(),
        &dir.path().join("r.json"),
        Some(&mut capture),
        RunnerOutput::Discard,
    )
    .unwrap();
    assert_eq!(ran.exit_code, Some(0));
    let env = String::from_utf8(capture).unwrap();
    assert!(env.contains("CARGO_SPECFORGE_PROBE=passed"), "{env}");
    for withheld in [
        "CARGO_REGISTRY_TOKEN=",
        "CARGO_REGISTRIES_X_TOKEN=",
        "CARGO_REGISTRIES_X_CREDENTIAL_PROVIDER=",
        "PGPASSWORD=",
        "MYSQL_PWD=",
        "GH_PAT=",
        "user:pa",
    ] {
        assert!(!env.contains(withheld), "{withheld}: {env}");
    }
    // The proxy still routes the command's fetches: without its password, or whole when it has none.
    assert!(
        env.contains("CARGO_HTTP_PROXY=http://proxy.test:8080\n"),
        "{env}"
    );
    assert!(
        env.contains("HTTPS_PROXY=http://proxy.test:3128\n"),
        "{env}"
    );
    assert!(env.contains("ALL_PROXY=proxy.test:1080\n"), "{env}");
    assert!(env.contains("DATABASE_URL=postgres://db.test/x\n"), "{env}");
    assert!(
        env.contains("OLDPWD=/probe/before\n"),
        "a working directory: {env}"
    );
}

/// The report path a caller exported, as a test run exports its report directory.
#[cfg(unix)]
const EXPORTED_REPORT: &str = "/a/report/directory/the/caller/exported";

#[cfg(unix)]
#[test]
fn a_command_told_its_report_runs_without_the_report_env() {
    // The test runs again in a child process whose environment holds the caller's own `SPECFORGE_REPORT`, so the
    // machine meets an inherited value while this process's environment is never written.
    if std::env::var_os(REPORT_ENV).as_deref() != Some(EXPORTED_REPORT.as_ref()) {
        let module = module_path!()
            .split_once("::")
            .map_or("", |(_, module)| module);
        let name = format!("{module}::a_command_told_its_report_runs_without_the_report_env");
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([name.as_str(), "--exact", "--test-threads=1"])
            .env(REPORT_ENV, EXPORTED_REPORT)
            .output()
            .unwrap();
        let said = String::from_utf8_lossy(&child.stdout);
        assert!(
            child.status.success() && said.contains(" 1 passed;"),
            "{said}{}",
            String::from_utf8_lossy(&child.stderr)
        );
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let seen = dir.path().join("seen");
    let argv = vec![
        "sh".to_string(),
        "-c".into(),
        format!(
            "printf '%s' \"${{{REPORT_ENV}:-unset}}\" > {}",
            seen.display()
        ),
    ];
    for (report_in_env, expected) in [
        (false, "unset".to_string()),
        (true, dir.path().join("r.xml").display().to_string()),
    ] {
        let ran = real_machine()
            .run(
                &CollectorCommand {
                    argv: &argv,
                    dir: dir.path(),
                    report: &dir.path().join("r.xml"),
                    report_in_env,
                    output: RunnerOutput::Discard,
                },
                None,
            )
            .unwrap();
        assert_eq!(ran.exit_code, Some(0));
        assert_eq!(std::fs::read_to_string(&seen).unwrap(), expected);
    }
}
