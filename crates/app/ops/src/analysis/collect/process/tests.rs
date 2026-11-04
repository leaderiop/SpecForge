use super::*;
use crate::analysis::collect::Collector;
use specforge_test_macros::test as specforge_test;
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;

fn collector(report: &str) -> Collector {
    Collector {
        extension: "@acme/runner".into(),
        name: "acme".into(),
        export: "collect__acme".into(),
        detect: vec!["acme.config.*".into()],
        run: vec!["acme".into(), "--out={report}".into()],
        report: report.into(),
        capture: None,
        writes: false,
        inputs: Vec::new(),
    }
}

/// A machine with its consent store at `store`, running real programs.
fn real_machine(store: PathBuf) -> ProcessMachine {
    ProcessMachine::with_store(store, Arc::new(specforge_process::Runner))
}

/// Run `argv` in `dir` with `report` as its report location.
fn run(
    argv: &[String],
    dir: &Path,
    report: &Path,
    capture: Option<&mut dyn Write>,
    output: RunnerOutput,
) -> Result<Ran, RunFailure> {
    real_machine(dir.join("consent.json")).run(
        &CollectorCommand {
            argv,
            dir,
            report,
            output,
        },
        capture,
    )
}

#[specforge_test(
    port = "CollectorMachine",
    verify = "CollectorMachine contract is satisfied"
)]
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
    assert!(matches!(failure, RunFailure::NotRun(_)));

    let machine = real_machine(dir.path().join("consent.json"));
    let mut c = collector("r.json");
    let approval = c.approval(dir.path());
    assert!(!machine.approved(&approval));
    machine.approve(&approval).unwrap();
    assert!(machine.approved(&approval));
    assert!(!machine.approved(&c.approval(&dir.path().join("other"))));
    c.run.push("--extra".into());
    let changed = c.approval(dir.path());
    assert!(!machine.approved(&changed), "a changed command re-prompts");
    machine.approve(&changed).unwrap();
    assert_eq!(
        machine.load().unwrap().approved.len(),
        1,
        "approval replaced"
    );

    // A store whose parent is a file cannot be written.
    let file = dir.path().join("a-file");
    std::fs::write(&file, "").unwrap();
    let stuck = real_machine(file.join("consent.json"));
    assert!(stuck.approve(&approval).is_err());
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
#[specforge_test(
    behavior = "run_collector_command",
    verify = "a captured stdout is kept in every output mode"
)]
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

#[specforge_test(
    behavior = "approve_collector_command",
    verify = "consent is keyed by project, extension and command"
)]
fn consent_is_keyed_by_project_extension_and_command() {
    let dir = tempfile::tempdir().unwrap();
    let machine = real_machine(dir.path().join("consent.json"));
    let root = dir.path();
    let mut c = collector("r.json");
    let approval = c.approval(root);
    assert!(!machine.approved(&approval));
    machine.approve(&approval).unwrap();
    assert!(machine.approved(&approval));
    assert!(!machine.approved(&c.approval(&root.join("other"))));
    c.run.push("--extra".into());
    let changed = c.approval(root);
    assert!(!machine.approved(&changed), "a changed command re-prompts");
    machine.approve(&changed).unwrap();
    assert_eq!(
        machine.load().unwrap().approved.len(),
        1,
        "approval replaced"
    );
}

#[cfg(unix)]
#[specforge_test(
    behavior = "run_collector_command",
    verify = "a collector command past its deadline is stopped"
)]
fn a_command_past_the_deadline_is_stopped() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let (tx, rx) = std::sync::mpsc::channel();
    // The run is left to finish on its thread when the pin fails.
    std::thread::spawn(move || {
        let argv = vec!["sh".to_string(), "-c".into(), "sleep 30".into()];
        let ran = real_machine(root.join("consent.json"))
            .with_deadline(std::time::Duration::from_secs(1))
            .run(
                &CollectorCommand {
                    argv: &argv,
                    dir: &root,
                    report: &root.join("r.json"),
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
    let ran = real_machine(dir.path().join("consent.json"))
        .with_deadline(std::time::Duration::from_secs(1))
        .run(
            &CollectorCommand {
                argv: &argv,
                dir: dir.path(),
                report: &dir.path().join("r.json"),
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
#[specforge_test(
    behavior = "run_collector_command",
    verify = "a .cmd shim on PATH runs as a collector command on Windows"
)]
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
#[specforge_test(
    behavior = "approve_collector_command",
    verify = "the consent store is replaced privately and atomically, never through a link"
)]
fn the_consent_store_is_replaced_privately_never_through_a_link() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let victim = dir.path().join("victim.json");
    std::fs::write(&victim, "{}").unwrap();
    std::fs::set_permissions(&victim, std::fs::Permissions::from_mode(0o644)).unwrap();
    let store = dir.path().join("consent.json");
    std::os::unix::fs::symlink(&victim, &store).unwrap();
    let approval = collector("r.json").approval(dir.path());
    real_machine(store.clone()).approve(&approval).unwrap();
    assert_eq!(
        std::fs::read(&victim).unwrap(),
        b"{}",
        "the link's target is untouched"
    );
    let meta = std::fs::symlink_metadata(&store).unwrap();
    assert!(meta.file_type().is_file(), "the store is a regular file");
    assert_eq!(meta.permissions().mode() & 0o777, 0o600);

    let nested = dir
        .path()
        .join("home")
        .join(".specforge")
        .join("consent.json");
    real_machine(nested.clone()).approve(&approval).unwrap();
    let parent = std::fs::metadata(nested.parent().unwrap()).unwrap();
    assert_eq!(parent.permissions().mode() & 0o777, 0o700);
}

#[specforge_test(
    behavior = "approve_collector_command",
    verify = "an approval is never written over a consent store that cannot be read"
)]
fn the_consent_store_is_never_overwritten_when_it_cannot_be_read() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("consent.json");
    let broken = br#"{"approved": [{"project": "/elsewhere", "extension": "@a/b", "collector": "c", "run": ["x"]},]}"#;
    std::fs::write(&store, broken).unwrap();
    let machine = real_machine(store.clone());
    let approval = collector("r.json").approval(dir.path());
    let err = machine.approve(&approval).unwrap_err();
    assert!(err.contains("consent.json"), "{err}");
    assert_eq!(std::fs::read(&store).unwrap(), broken.to_vec());
    assert!(!machine.approved(&approval));
}
