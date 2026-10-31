//! Ported from gsd-core v1.19.0 tests/gsd-secret-read-guard.test.cjs, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! What the v1.19.0 recording cannot replay of that suite (#5627, #5628, #5046, #5513): the calls over the recorder's
//! input cap, written out by hand, and upstream's properties walked over their whole domain rather than one run's
//! draws. Each row is a Bash command and the answer upstream's test asserts.

use super::*;

/// The cases of v1.19.0's suite whose calls are too long for the recording (over `REC_INPUT_MAX`), by the test that
/// runs them: the accounting counts them as ported.
pub const PORTED: &[(&str, &[&str])] = &[
    (
        "a_mebibyte_of_wrappers_is_walked_in_linear_time",
        &[
            "gsd-secret-read-guard: gsd-debugger CLI spawn (#5214) > blocks: 1 MiB of leading timeout wrappers finishes inside the hook budget",
            "gsd-secret-read-guard: gsd-debugger CLI spawn (#5214) > a ${bytes}-byte command is denied as ${code}",
        ],
    ),
    (
        "a_long_or_dense_cluster_is_judged_as_a_short_one",
        &[
            "gsd-secret-read-guard: bundled short-flag clusters (#5046) > dense value, ${n} units: `.env` x N + `.example` is allowed",
            "gsd-secret-read-guard: bundled short-flag clusters (#5046) > dense value, ${n} units: `.envrc` x N is allowed",
            "gsd-secret-read-guard: bundled short-flag clusters (#5046) > dense value, ${n} units: `.envrc` x N + `/.env` is blocked",
            "gsd-secret-read-guard: bundled short-flag clusters (#5046) > dense value, ${n} units: `.env` x N is blocked",
        ],
    ),
];

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

/// `property: stacked timeout wrappers only change the verdict through the final agent`, over its whole domain (the
/// recording holds one run's draws).
#[test]
fn the_debugger_is_seen_through_stacked_wrappers() {
    let mut rows: Vec<(String, Want)> = Vec::new();
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
/// bound; one byte more is refused unscanned. The bound is on work, not time: the quadratic walk this replaced
/// (`unwrap_timeout`, a `resolve` per wrapper) took 2.85 s here against 0.34 s, inside any safe wall-clock bound.
#[test]
fn a_mebibyte_of_wrappers_is_walked_in_linear_time() {
    const TAIL: &str = "claude --agent gsd-debugger";
    let padded = |bytes: usize| {
        let base = format!("{}{TAIL}", "timeout 1 ".repeat((bytes - TAIL.len()) / 10));
        let pad = " ".repeat(bytes - base.len());
        base + &pad
    };
    let command = padded(MAX_COMMAND_LENGTH - 1);
    let words = command.split_whitespace().count();
    scan::RESOLVED_WORDS.with(|n| n.set(0));
    holds(vec![(command, Want::Debugger)]);
    // One `resolve` of the segment per scan (the secret read, the debugger spawn): linear in the words.
    let resolved = scan::RESOLVED_WORDS.with(std::cell::Cell::get);
    assert!(
        resolved <= 2 * words,
        "the 1 MiB scan resolved {resolved} words of {words}"
    );
    holds(vec![
        (padded(MAX_COMMAND_LENGTH), Want::Debugger),
        (padded(MAX_COMMAND_LENGTH + 1), Want::TooLarge),
    ]);
}

/// The 20000-unit dense values are over 80 000 characters, more than upstream's recorder stores (the 199–201-unit rows
/// replay); the `.a` interleave with a space before `pat` is a variant of upstream's row the recording does not hold.
#[test]
fn a_long_or_dense_cluster_is_judged_as_a_short_one() {
    let mut rows: Vec<(String, Want)> = [199, 200, 201]
        .into_iter()
        .map(|n| (format!("grep -{} pat f", ".a".repeat(n)), Want::Allow))
        .collect();
    let n = 20000;
    rows.push((
        format!("grep -i{}.example pat f", ".env".repeat(n)),
        Want::Allow,
    ));
    rows.push((format!("grep -i{} pat f", ".envrc".repeat(n)), Want::Allow));
    let word = format!("-i{}/.env", ".envrc".repeat(n));
    rows.push((format!("grep {word} pat f"), read(&word)));
    let word = format!("-if{}", ".env".repeat(n));
    rows.push((format!("grep {word} pat f"), read(&word)));
    holds(rows);
}

const CLUSTER_PREFIX: [&str; 16] = [
    "i", "n", "v", "q", "c", "2", "1", "0", "i2", "2i", "n2", "if", "nf", "i1f", "vfq2", "c1n",
];
const SECRET: [&str; 5] = [
    ".env",
    ".env.local",
    ".secrets",
    "config/.env.production",
    ".ENV",
];
const LOOKALIKE: [&str; 5] = [
    ".env.example",
    ".env.sample",
    ".env.template",
    ".env.dist",
    ".envrc",
];

/// Upstream's two cluster properties, over every pair of their generators' constants.
#[test]
fn every_cluster_shape_hides_no_secret_and_spares_the_templates() {
    let mut rows: Vec<(String, Want)> = Vec::new();
    for prefix in CLUSTER_PREFIX {
        for secret in SECRET {
            let cluster = format!("-{prefix}{secret}");
            rows.push((format!("grep {cluster} pat f"), read(&cluster)));
        }
        for lookalike in LOOKALIKE {
            rows.push((format!("grep -{prefix}{lookalike} pat f"), Want::Allow));
        }
    }
    holds(rows);
}

/// `property: a bundled value is judged like the same value unbundled`, over a fixed sample of its generators.
#[test]
fn a_bundled_value_is_judged_like_the_same_value_unbundled() {
    const FLAG_RUN: [&str; 6] = ["i", "n", "f", "2A", "vf1", "Ain2"];
    const LEAD: [&str; 8] = [
        ".env",
        ".envrc",
        ".ENVRC",
        ".secrets.bak",
        ".env.example",
        ".secretsx",
        "/",
        ".",
    ];
    const DENSE: [&str; 8] = [
        ".env",
        ".envrc",
        ".ENVRC",
        ".secrets.bak",
        ".env.example",
        ".secretsx",
        "a",
        "/",
    ];
    const TERMINAL: [&str; 11] = [
        "", ".example", ".local", ".env", ".ENV", ".envrc", ".secrets", "/x", ".sample", ".env.",
        ":x",
    ];
    let mut differ = Vec::new();
    let mut k = 0;
    for flags in FLAG_RUN {
        for lead in LEAD {
            for terminal in TERMINAL {
                k += 1;
                let units: String = (0..k % 4)
                    .map(|u| DENSE[(k + u * 3) % DENSE.len()])
                    .collect();
                let value = format!("{lead}{units}{terminal}");
                let denied = |command: String| bash(&command).decision == Some(Decision::Deny);
                if denied(format!("grep -{flags}{value} pat f")) != denied(format!("cat {value}")) {
                    differ.push(format!("-{flags}{value}"));
                }
            }
        }
    }
    assert!(
        differ.is_empty(),
        "bundled and unbundled differ: {differ:?}"
    );
}

/// Only a command that opens a heredoc can read differently: any other is not scanned again (`cat .env` would be
/// denied by any scan, so `None` means none ran).
#[test]
fn no_heredoc_runs_one_reading() {
    assert_eq!(
        scan::find_secret_read_under_other_readings("cat .env"),
        None
    );
    assert_eq!(
        scan::find_secret_read_under_other_readings("cat <<EOF\nx\nEOF\ncat .env"),
        Some(".env".into())
    );
}

#[test]
fn the_reading_is_restored_after_a_panic() {
    let other = shell::Reading {
        bodies_skipped_in_spans: false,
        tags_as_words: false,
    };
    let caught = std::panic::catch_unwind(|| {
        shell::with_reading(other, || {
            assert!(!shell::reading().tags_as_words);
            panic!("inside the reading")
        })
    });
    assert!(caught.is_err());
    let now = shell::reading();
    assert!(now.bodies_skipped_in_spans && now.tags_as_words);
}
