//! `specforge run` (ADR 0339): the one runner for people and agents. The child is this test binary itself, which runs
//! [`child_program`] when `RUN_VERB_CHILD` names what to do, so every OS runs the same program.

use std::io::{Read as _, Write as _};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::support::{Cli, TestProject};

/// What the child does, read from `RUN_VERB_CHILD`: `exit:<n>`, `sleep`, `echo` (stdin to stdout), `env:<name>` or
/// `talk` (a line to each stream). Outside a `specforge run` it does nothing and passes.
#[test]
fn child_program() {
    let Ok(action) = std::env::var("RUN_VERB_CHILD") else {
        return;
    };
    let mut out = std::io::stdout();
    match action.as_str() {
        "sleep" => std::thread::sleep(Duration::from_secs(60)),
        "echo" => {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input).unwrap();
            writeln!(out, "child:got {input}").unwrap();
        }
        // Past the 4 MiB a capture once stopped at, below its 64 MiB cap.
        "flood" => out.write_all(&vec![b'x'; 5 * 1024 * 1024]).unwrap(),
        "talk" => {
            writeln!(out, "child:out").unwrap();
            eprintln!("child:err");
        }
        other => {
            if let Some(code) = other.strip_prefix("exit:") {
                out.flush().unwrap();
                std::process::exit(code.parse().unwrap());
            }
            if let Some(name) = other.strip_prefix("env:") {
                let value = std::env::var(name).unwrap_or_default();
                writeln!(out, "child:{name}={value}").unwrap();
            }
        }
    }
    out.flush().unwrap();
    std::process::exit(0);
}

/// The argv that runs [`child_program`] alone.
fn child() -> Vec<String> {
    let exe = std::env::current_exe().unwrap().display().to_string();
    [
        exe.as_str(),
        "run_verb::child_program",
        "--exact",
        "--nocapture",
        "--test-threads",
        "1",
    ]
    .iter()
    .map(ToString::to_string)
    .collect()
}

fn run<'a>(project: &'a TestProject, action: &str) -> Cli<'a> {
    Cli::on(project).env("RUN_VERB_CHILD", action)
}

fn args<'a>(flags: &[&'a str], argv: &'a [String]) -> Vec<&'a str> {
    flags
        .iter()
        .copied()
        .chain(argv.iter().map(String::as_str))
        .collect()
}

#[test]
fn the_programs_exit_passes_through() {
    let project = TestProject::new();
    let argv = child();
    run(&project, "exit:3")
        .bare(&args(&["run", "--"], &argv))
        .exits(3);
    run(&project, "exit:0")
        .bare(&args(&["run"], &argv))
        .exits(0);
}

#[test]
fn a_program_past_its_deadline_is_stopped_with_124() {
    let project = TestProject::new();
    let argv = child();
    let started = Instant::now();
    let stopped = run(&project, "sleep")
        .bare(&args(&["run", "--deadline", "1s"], &argv))
        .exits(124);
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );
    assert!(
        stopped
            .stderr
            .contains("did not finish before its deadline"),
        "{}",
        stopped.stderr
    );
}

#[test]
fn a_missing_program_is_127() {
    let project = TestProject::new();
    let missing = Cli::on(&project)
        .bare(&["run", "no-such-program-anywhere"])
        .exits(127);
    assert!(
        missing.stderr.contains("was not found"),
        "{}",
        missing.stderr
    );
}

#[test]
fn which_finds_a_program_and_runs_none() {
    let project = TestProject::new();
    let bin = Cli::binary_dir();
    let found = Cli::on(&project)
        .env("PATH", &bin)
        .bare(&[
            "run",
            "--which",
            "--format",
            "json",
            "specforge",
            "no-such-program",
        ])
        .exits(0)
        .answer();
    let path = found["programs"][0]["path"].as_str().unwrap();
    assert!(path.contains("specforge"), "{found}");
    assert_eq!(found["programs"][0]["found"], true);
    assert_eq!(found["programs"][1]["found"], false);
    assert_eq!(found["programs"][1]["path"], Value::Null);
    // As `command -v`: the path, and 1 when a program is missing.
    let shown = Cli::on(&project)
        .env("PATH", &bin)
        .bare(&["run", "--which", "specforge"])
        .exits(0);
    assert!(shown.stdout.contains("specforge"), "{}", shown.stdout);
    Cli::on(&project)
        .env("PATH", &bin)
        .bare(&["run", "--which", "no-such-program"])
        .exits(1);
}

#[test]
fn piped_stdin_and_the_users_whole_environment_reach_the_program() {
    let project = TestProject::new();
    let argv = child();
    let echoed = run(&project, "echo")
        .stdin("hello")
        .bare(&args(&["run"], &argv))
        .exits(0);
    assert!(
        echoed.stdout.contains("child:got hello"),
        "{}",
        echoed.stdout
    );
    // A typed program is the user's own act: no deny-list strips a token the user set.
    let token = run(&project, "env:RUN_VERB_TOKEN")
        .env("RUN_VERB_TOKEN", "t0k")
        .bare(&args(&["run"], &argv))
        .exits(0);
    assert!(
        token.stdout.contains("child:RUN_VERB_TOKEN=t0k"),
        "{}",
        token.stdout
    );
}

/// The captured answer with what differs between runs and machines written as placeholders, and only the child's own
/// lines of its output.
fn steady(mut answer: Value) -> Value {
    answer["duration_ms"] = "<duration_ms>".into();
    answer["argv"][0] = "<test binary>".into();
    for stream in ["stdout", "stderr"] {
        let own: String = answer[stream]
            .as_str()
            .unwrap()
            .lines()
            // libtest may print `test <name> ... ` before the child's line.
            .filter_map(|line| line.find("child:").map(|at| format!("{}\n", &line[at..])))
            .collect();
        answer[stream] = own.into();
    }
    answer
}

#[test]
fn json_captures_the_run_and_replay_answers_it_again() {
    let project = TestProject::new();
    let argv = child();
    let mut live = run(&project, "talk")
        .bare(&args(&["run", "--format", "json"], &argv))
        .exits(0)
        .json();
    // The record is the user's, and a replay is the live answer, said to be the journal's.
    let mut replayed = Cli::on(&project)
        .bare(&args(
            &["run", "--format", "json", "--effects", "replay"],
            &argv,
        ))
        .exits(0)
        .json();
    let effect = |answer: &mut Value| answer.as_object_mut().unwrap().remove("effect").unwrap();
    let (was, again) = (effect(&mut live), effect(&mut replayed));
    assert!(
        was["digest"]
            .as_str()
            .is_some_and(|d| d.starts_with("sha256:")),
        "{was}"
    );
    assert_eq!(
        (&was["digest"], &was["observed_at"], &was["from_journal"]),
        (
            &again["digest"],
            &again["observed_at"],
            &serde_json::json!(false)
        )
    );
    assert_eq!(again["from_journal"], true);
    insta::assert_json_snapshot!("run_captured", steady(live.clone()));
    assert_eq!(replayed, live);
    let picked = run(&project, "exit:4")
        .bare(&args(
            &["run", "--format", "json", "--pick", ".exit"],
            &argv,
        ))
        .exits(0);
    assert_eq!(picked.stdout, "4\n");
}

#[test]
fn replay_refuses_a_run_never_captured_and_a_pass_through_takes_no_journal_mode() {
    let project = TestProject::new();
    let argv = child();
    let missed = Cli::on(&project)
        .bare(&args(
            &["run", "--format", "json", "--effects", "replay"],
            &argv,
        ))
        .exits(1);
    assert_eq!(missed.json()["code"], "not_in_journal", "{}", missed.stdout);
    for mode in ["cached", "replay"] {
        Cli::on(&project)
            .bare(&args(&["run", "--effects", mode], &argv))
            .exits(2);
    }
    // --pick reads the captured answer.
    let picked = run(&project, "exit:5")
        .bare(&args(&["run", "--pick", ".exit"], &argv))
        .exits(0);
    assert_eq!(picked.stdout, "5\n");
}

#[test]
fn a_ceiling_with_process_off_refuses_every_run() {
    let project = TestProject::new().config(|config| {
        config["effects"] = serde_json::json!({"process": "off"});
    });
    let argv = child();
    let refused = run(&project, "exit:0")
        .bare(&args(&["run", "--format", "json"], &argv))
        .exits(1);
    assert_eq!(refused.json()["code"], "E090", "{}", refused.stdout);
}

/// A pass-through run on a terminal hands its program the terminal's foreground: a program that reads the terminal
/// itself gets its input instead of being stopped by SIGTTIN (`specforge run -- sh -c 'read x; …'`). The run is the
/// leader of a new session whose controlling terminal is a pseudo-terminal the test writes to.
#[cfg(unix)]
#[test]
fn a_pass_through_run_on_a_terminal_gives_its_program_the_input() {
    use std::os::fd::{AsRawFd as _, FromRawFd as _, OwnedFd};
    use std::os::unix::process::CommandExt as _;
    let project = TestProject::new();
    let (mut master, slave) = {
        let (mut master, mut slave) = (0, 0);
        // SAFETY: openpty writes two descriptors into the locals and reads no other memory.
        #[expect(unsafe_code, reason = "openpty(3) into two local descriptors")]
        let made = unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(made, 0, "openpty");
        // SAFETY: both descriptors were just opened and nothing else owns them.
        #[expect(unsafe_code, reason = "owning the descriptors openpty made")]
        unsafe {
            (
                std::fs::File::from_raw_fd(master),
                OwnedFd::from_raw_fd(slave),
            )
        }
    };
    let mut run = Cli::on(&project).std_command(&["run", "--", "sh", "-c", "read x; echo got:$x"]);
    let tty = slave.as_raw_fd();
    run.stdin(std::fs::File::from(slave.try_clone().unwrap()))
        .stdout(std::fs::File::from(slave.try_clone().unwrap()))
        .stderr(std::fs::File::from(slave));
    // SAFETY: setsid and ioctl(TIOCSCTTY) are async-signal-safe and touch no memory of the parent's.
    #[expect(
        unsafe_code,
        reason = "making the pseudo-terminal the child's controlling terminal"
    )]
    unsafe {
        run.pre_exec(move || {
            libc::setsid();
            libc::ioctl(tty, libc::TIOCSCTTY as _, 0);
            Ok(())
        });
    }
    let mut child = run.spawn().unwrap();
    master.write_all(b"hello\n").unwrap();
    let (seen, read) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut all = Vec::new();
        let mut chunk = [0u8; 256];
        while let Ok(n) = master.read(&mut chunk) {
            if n == 0 {
                break;
            }
            all.extend_from_slice(&chunk[..n]);
            if String::from_utf8_lossy(&all).contains("got:hello") {
                let _ = seen.send(());
                break;
            }
        }
    });
    let got = read.recv_timeout(Duration::from_secs(20)).is_ok();
    let _ = child.kill();
    let _ = child.wait();
    assert!(got, "the program never read its input: stopped by SIGTTIN");
}

/// A capture holds the spec's 64 MiB: a program that writes 5 MiB runs to its end, and the answer, which keeps 1 MiB
/// of each stream, says `truncated`.
#[test]
fn a_capture_lets_a_program_write_past_four_mib() {
    let project = TestProject::new();
    let argv = child();
    let answer = run(&project, "flood")
        .bare(&args(&["run", "--format", "json"], &argv))
        .exits(0)
        .answer();
    assert_eq!(answer["outcome"], "exited", "{}", answer["stderr"]);
    assert_eq!(answer["exit"], 0);
    assert_eq!(answer["truncated"], true);
}
