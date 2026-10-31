//! Ported from gsd-core v1.18.0 tests/gsd-secret-read-guard.test.cjs, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! The cases gsd-core v1.19.0 adds to that suite (#5627, #5628, #5046, #5513), written out by hand while the replay
//! still reads the v1.18.0 recording (plan 06 T2 folds them into the v1.19.0 replay). Each row is a Bash command and
//! the answer upstream's test asserts.

use super::*;

/// What upstream's test asserts of a command.
enum Want {
    Allow,
    /// Denied as a secret read, naming this word.
    Read(String),
    /// Denied as the debugger spawned from a shell.
    Debugger,
    /// Denied unscanned as too large.
    TooLarge,
}

fn read(path: &str) -> Want {
    Want::Read(path.into())
}

fn bash(command: &str) -> HookOutput {
    secret_read_guard(&Facts {
        class: "shell".into(),
        tool: "Bash".into(),
        input: serde_json::json!({ "command": command }),
        ..Facts::default()
    })
}

/// The rows whose answer differs from `want`, each with what we said.
fn misses(rows: Vec<(String, Want)>) -> Vec<String> {
    rows.into_iter()
        .filter_map(|(command, want)| {
            let answer = bash(&command);
            let reason = answer.reason.unwrap_or_default();
            let denied = answer.decision == Some(Decision::Deny);
            let ok = match &want {
                Want::Allow => !denied,
                Want::Read(path) => denied && reason.contains(&format!("would read '{path}'")),
                Want::Debugger => denied && reason.contains("'--agent gsd-debugger'"),
                Want::TooLarge => denied && reason.contains("over 1048576 characters"),
            };
            let shown: String = command.chars().take(120).collect();
            (!ok).then(|| {
                format!(
                    "{shown:?}: {}",
                    if denied { reason } else { "allowed".into() }
                )
            })
        })
        .collect()
}

fn holds(rows: Vec<(String, Want)>) {
    let n = rows.len();
    let missed = misses(rows);
    assert!(
        missed.is_empty(),
        "{} of {n} rows differ:\n{}",
        missed.len(),
        missed.join("\n")
    );
}

const WRAPPERS: [&str; 37] = [
    "timeout 5",
    "timeout -s KILL 5",
    "timeout --signal=KILL --kill-after=2 5",
    "timeout --foreground 5",
    "nice -n 5",
    "nice -5",
    "nice -n5",
    "nice --adjustment=5",
    "nice --adjustment 5",
    "timeout 5 --",
    "timeout -- 5",
    "env -P /usr/bin",
    "env -i",
    "env -u HOME",
    "env -C /tmp",
    "env -i A=1",
    "env --",
    "env -0",
    "command -p",
    "command --",
    "time -p",
    "time -f %e",
    "stdbuf -oL",
    "stdbuf -i0 -oL -eL",
    "stdbuf --output=L",
    "setsid",
    "setsid -w",
    "exec",
    "exec -a name",
    "nohup",
    "sudo -u root",
    "sudo -E",
    "sudo --",
    "sudo -u root -g wheel",
    "doas -u root",
    "nice -n 5 timeout -s KILL 5 env -i",
    "sudo -n timeout 5 nohup",
];
const NESTED: &str = "bash -c 'cat .env'";

/// `describe nested shell behind a wrapper with flags (#5627)`.
#[test]
fn a_nested_shell_behind_a_wrapper_with_flags_is_read() {
    let mut rows: Vec<(String, Want)> = WRAPPERS
        .iter()
        .map(|w| (format!("{w} {NESTED}"), read(".env")))
        .collect();
    for route in [
        "timeout 5 bash <<'EOF'\ncat .env\nEOF",
        "nice -n 5 bash <<< 'cat .env'",
        "echo cat .env | nice -n 5 bash",
        "timeout 5 eval 'cat .env'",
        "echo .env | timeout 5 xargs cat",
        "nice -n 5 git -c alias.x='!cat .env' x",
    ] {
        rows.push((route.into(), read(".env")));
    }
    for allowed in [
        "timeout 5 bash -c 'echo hi'",
        "nice -n 5 bash -c 'cat README.md'",
        "env -i bash -c 'cat .env.example'",
        "timeout 5 ls",
        "timeout 5",
        "nice -n 5",
        "env -i",
        "time -p cat README.md",
    ] {
        rows.push((allowed.into(), Want::Allow));
    }
    rows.push((
        "timeout -s KILL 600 claude --agent gsd-debugger -p hi".into(),
        Want::Debugger,
    ));
    holds(rows);
}

/// Upstream's property draws 1–3 wrappers that hold no inner `timeout`/`env`; here every stack of up to 40 is
/// walked, the wrappers taken in a fixed rotation.
#[test]
fn any_wrapper_stack_in_front_of_a_nested_read_is_read() {
    let rows = (0..=40)
        .map(|depth| {
            let stack: Vec<&str> = (0..depth)
                .map(|k| WRAPPERS[(k * 7 + depth) % WRAPPERS.len()])
                .collect();
            (format!("{} {NESTED}", stack.join(" ")), read(".env"))
        })
        .collect();
    holds(rows);
}

/// `describe gsd-debugger CLI spawn (#5214)`: the wrapper walk replaces the `timeout` unwrap (#5628).
#[test]
fn the_debugger_is_seen_through_stacked_wrappers() {
    let mut rows: Vec<(String, Want)> = vec![
        (
            "timeout 1 timeout 1 claude --agent gsd-debugger".into(),
            Want::Debugger,
        ),
        (
            "sudo timeout 1 env X=1 timeout --foreground 2 claude --agent=gsd-debugger".into(),
            Want::Debugger,
        ),
    ];
    for allowed in [
        "timeout 1 timeout",
        "timeout",
        "timeout 1",
        "timeout -s",
        "timeout 1 timeout 1 claude --agent gsd-executor",
    ] {
        rows.push((allowed.into(), Want::Allow));
    }
    for depth in 0..=40 {
        let stack = "timeout 1 ".repeat(depth);
        rows.push((
            format!("{stack}claude --agent gsd-debugger"),
            Want::Debugger,
        ));
        rows.push((format!("{stack}claude --agent gsd-executor"), Want::Allow));
    }
    holds(rows);
}

/// 1 MiB of `timeout 1 ` in front of the debugger: the walk is linear, so it is scanned inside upstream's 4 s kill
/// bound; one byte more is refused unscanned.
#[test]
fn a_mebibyte_of_wrappers_is_walked_in_linear_time() {
    const TAIL: &str = "claude --agent gsd-debugger";
    let padded = |bytes: usize| {
        let base = format!("{}{TAIL}", "timeout 1 ".repeat((bytes - TAIL.len()) / 10));
        let pad = " ".repeat(bytes - base.len());
        base + &pad
    };
    let command = padded(MAX_COMMAND_LENGTH - 1);
    let start = std::time::Instant::now();
    holds(vec![(command, Want::Debugger)]);
    let took = start.elapsed();
    assert!(took.as_secs() < 4, "the 1 MiB scan took {took:?}");
    holds(vec![
        (padded(MAX_COMMAND_LENGTH), Want::Debugger),
        (padded(MAX_COMMAND_LENGTH + 1), Want::TooLarge),
    ]);
}
