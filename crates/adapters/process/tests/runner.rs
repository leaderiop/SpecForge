//! The one process runner (ADR 0335): the Process contract, the deadline and the group kill, the cap, the
//! environment policy, text mode and Windows program resolution.
#![allow(
    clippy::unwrap_used,
    reason = "a test fails on the first unexpected value"
)]

use std::path::Path;
use std::time::Duration;

use specforge_effects::process::request;
use specforge_effects::{Output, Process, ProcessClass, ProcessRequest};
use specforge_process::{Runner, resolve, secret};

fn run(request: &ProcessRequest) -> specforge_effects::ProcessOutcome {
    Runner.run(request, &std::env::temp_dir(), Output::Buffer)
}

#[cfg(unix)]
fn sh(script: &str, deadline: Duration) -> ProcessRequest {
    request(["sh", "-c", script], deadline)
}

#[test]
fn the_runner_keeps_the_process_contract() {
    specforge_effects::testing::process_contract(&Runner);
}

#[test]
fn a_missing_program_is_not_found() {
    let outcome = run(&request(
        ["specforge-no-such-program"],
        Duration::from_secs(5),
    ));
    assert_eq!(outcome.class, ProcessClass::NotFound, "{outcome:?}");
}

#[cfg(unix)]
#[test]
fn a_grandchild_dies_with_the_group() {
    let outcome = run(&sh("sleep 30 & echo $!; wait", Duration::from_millis(300)));
    assert_eq!(outcome.class, ProcessClass::DeadlineExceeded, "{outcome:?}");
    let pid = outcome.text().trim().to_string();
    assert!(!pid.is_empty(), "{outcome:?}");
    // The killed grandchild is reaped by init: until then `kill -0` still finds it.
    let gone = (0..200).any(|_| {
        let alive = std::process::Command::new("kill")
            .args(["-0", &pid])
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap()
            .success();
        if alive {
            std::thread::sleep(Duration::from_millis(10));
        }
        !alive
    });
    assert!(gone, "the grandchild {pid} outlived the deadline");
}

#[cfg(unix)]
#[test]
fn output_past_the_cap_stops_the_run() {
    let mut capped = sh("yes", Duration::from_secs(20));
    capped.output_cap = 4096;
    let outcome = run(&capped);
    assert_eq!(outcome.class, ProcessClass::OutputCapped, "{outcome:?}");
    assert!(
        outcome.stdout.len() <= 4096 + 64 * 1024,
        "{}",
        outcome.stdout.len()
    );
}

#[cfg(unix)]
#[test]
#[expect(
    unsafe_code,
    reason = "set_var before any thread of this test reads the environment; the test runs in its own process under nextest"
)]
fn a_secret_variable_is_not_passed() {
    // SAFETY: no other thread of this test reads the environment.
    unsafe { std::env::set_var("SPECFORGE_PROBE_TOKEN", "x") };
    let mut listed = request(["env"], Duration::from_secs(10));
    listed.env = vec![specforge_effects::EnvVar::set("SPECFORGE_PROBE_SET", "y")];
    let outcome = run(&listed);
    assert_eq!(outcome.class, ProcessClass::Exited(0), "{outcome:?}");
    let env = outcome.text();
    assert!(!env.contains("SPECFORGE_PROBE_TOKEN"), "{env}");
    assert!(env.contains("SPECFORGE_PROBE_SET=y"), "{env}");
}

#[test]
fn a_secret_is_named_like_one() {
    assert!(secret("aws_region".as_ref()) && secret("GH_API_KEY".as_ref()));
    assert!(secret("SIGNING_KEY".as_ref()) && secret("AZURE_CREDENTIALS".as_ref()));
    assert!(secret("GOOGLE_APPLICATION_CREDENTIALS".as_ref()));
    assert!(!secret("TOKENIZER".as_ref()) && !secret("PATH".as_ref()));
}

#[cfg(unix)]
#[test]
fn a_request_cannot_put_a_secret_back() {
    let mut listed = request(["env"], Duration::from_secs(10));
    listed.env = vec![
        specforge_effects::EnvVar::set("SPECFORGE_PROBE_TOKEN", "x"),
        specforge_effects::EnvVar::set("SPECFORGE_PROBE_CREDENTIALS", "y"),
        specforge_effects::EnvVar::set("SPECFORGE_PROBE_PLAIN", "z"),
    ];
    let env = run(&listed).text();
    assert!(!env.contains("SPECFORGE_PROBE_TOKEN"), "{env}");
    assert!(!env.contains("SPECFORGE_PROBE_CREDENTIALS"), "{env}");
    assert!(env.contains("SPECFORGE_PROBE_PLAIN=z"), "{env}");
}

/// Whether the process `pid` still runs.
#[cfg(unix)]
fn alive(pid: &str) -> bool {
    std::process::Command::new("kill")
        .args(["-0", pid])
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap()
        .success()
}

#[cfg(unix)]
#[test]
fn a_clean_exit_leaves_its_background_child_running() {
    // The child holds stdout open past the program's exit: the run returns after the grace, with what was written.
    let started = std::time::Instant::now();
    let outcome = run(&sh("sleep 30 & echo $!", Duration::from_secs(20)));
    assert_eq!(outcome.class, ProcessClass::Exited(0), "{outcome:?}");
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "it waited for the child"
    );
    let pid = outcome.text().trim().to_string();
    assert!(alive(&pid), "the child {pid} was killed after a clean exit");
    std::process::Command::new("kill")
        .arg(&pid)
        .status()
        .unwrap();
}

#[cfg(unix)]
#[test]
fn stop_all_ends_every_running_group() {
    let dir = tempfile::tempdir().unwrap();
    let pid_file = dir.path().join("pid");
    let script = format!("sleep 30 & echo $! > {}; wait", pid_file.display());
    let running = std::thread::spawn(move || run(&sh(&script, Duration::from_secs(60))));
    let pid = (0..500)
        .find_map(|_| {
            std::thread::sleep(Duration::from_millis(10));
            std::fs::read_to_string(&pid_file)
                .ok()
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
        })
        .expect("the child started");
    assert_eq!(specforge_process::live(), 1);
    specforge_process::stop_all();
    let outcome = running.join().unwrap();
    assert_eq!(outcome.class, ProcessClass::Signalled, "{outcome:?}");
    let gone = (0..200).any(|_| {
        let up = alive(&pid);
        if up {
            std::thread::sleep(Duration::from_millis(10));
        }
        !up
    });
    assert!(gone, "the grandchild {pid} outlived stop_all");
    assert_eq!(specforge_process::live(), 0);
}

#[cfg(unix)]
#[test]
fn text_mode_reads_crlf_as_lf() {
    let mut printed = sh("printf 'a\\r\\nb\\r\\n'", Duration::from_secs(10));
    printed.text = true;
    assert_eq!(run(&printed).stdout, b"a\nb\n");
    printed.text = false;
    assert_eq!(run(&printed).stdout, b"a\r\nb\r\n");
}

#[test]
fn a_cmd_shim_resolves_through_pathext() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("shim.cmd"), "@exit /b 3\r\n").unwrap();
    let path = std::env::join_paths([Path::new("/nowhere"), dir.path()]).unwrap();
    // PATHEXT spells the extension (`.CMD`): Windows file names ignore case, so compare as it does.
    let found = |pathext: Option<&str>| {
        resolve("shim", Some(&path), pathext.map(AsRef::as_ref))
            .map(|p| p.to_string_lossy().to_lowercase())
    };
    let shim = Some(dir.path().join("shim.cmd").to_string_lossy().to_lowercase());
    assert_eq!(found(None), shim);
    assert_eq!(resolve("other", Some(&path), None), None);
    // The request's PATHEXT decides: one that names no `.cmd` finds no shim.
    assert_eq!(found(Some(".EXE")), None);
    assert_eq!(found(Some(".BAT;.CMD")), shim);
}

#[cfg(windows)]
#[test]
fn a_cmd_shim_on_path_runs() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("shim.cmd"), "@exit /b 3\r\n").unwrap();
    let outcome = Runner.run(
        &request(
            [dir.path().join("shim").to_string_lossy()],
            Duration::from_secs(10),
        ),
        dir.path(),
        Output::Buffer,
    );
    assert_eq!(outcome.class, ProcessClass::Exited(3), "{outcome:?}");
}
