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

/// `describe Bash blocks` and `Bash allows` (#5046 rows), and `describe bundled short-flag clusters (#5046)`.
#[test]
fn a_secret_bundled_behind_short_flags_is_read() {
    let mut rows: Vec<(String, Want)> = Vec::new();
    for cluster in [
        "-if.env",
        "-nif.env",
        "-if.secrets",
        "-2if.env",
        "-1f.env",
        "-2f.env",
        "-2if.secrets",
        "-2.env",
        "-1.env",
        "-i.env",
        "-2nif.env",
        "-f=.env",
        "-if=.env",
        "-nf=.env",
    ] {
        rows.push((format!("grep {cluster} pat f"), read(cluster)));
    }
    // Quoted: the reported word is the one after quote removal.
    rows.push(("grep -if'.env' pat f".into(), read("-if.env")));
    rows.push(("grep -if\".env\" pat f".into(), read("-if.env")));
    // New denials against v1.18.0: a secret-named value of a non-file option, and the documented fail-safe class
    // where the option run swallows a value's leading letters and digits.
    for (command, word) in [
        ("git commit -am.env", "-am.env"),
        ("head -n1.env", "-n1.env"),
        ("grep -fapp.env pat f", "-fapp.env"),
        ("grep -fprod.env pat f", "-fprod.env"),
        ("grep -ofoo.env pat f", "-ofoo.env"),
        ("git log -Sconfig.env", "-Sconfig.env"),
        ("sort -k2.env f", "-k2.env"),
        ("git log -S.env", "-S.env"),
    ] {
        rows.push((command.into(), read(word)));
    }
    for allowed in [
        "grep -if.env.example pat f",
        "grep -n.envrc pat f",
        "grep -2if.env.example pat f",
        "grep -1f.env.sample pat f",
        "grep -2n.envrc pat f",
        "grep -i pat f",
        "grep -2 pat f",
        "grep -12 pat f",
        "grep -vvv pat f",
    ] {
        rows.push((allowed.into(), Want::Allow));
    }
    // Expansion forms: the clustered form is judged as its unbundled one, both allowed.
    for (clustered, plain) in [
        ("grep -if.env* pat f", "grep .env* f"),
        ("grep -if.en? pat f", "grep .en? f"),
        ("grep -if.env~ pat f", "grep .env~ f"),
        ("grep -if$(echo .env) pat f", "grep $(echo .env) f"),
        ("grep -if{.env,x} pat f", "grep {.env,x} f"),
    ] {
        rows.push((clustered.into(), Want::Allow));
        rows.push((plain.into(), Want::Allow));
    }
    holds(rows);
}

/// The length rows: the verdict is the same at every length, and costs at most three name checks (the 20000-unit
/// rows are over 80 000 characters, more than upstream's recorder stores, so they stay hand ports).
#[test]
fn a_long_or_dense_cluster_is_judged_as_a_short_one() {
    let mut rows: Vec<(String, Want)> = Vec::new();
    for n in [199, 200, 201] {
        let a = "a".repeat(n);
        rows.push((format!("grep -{a}.env pat f"), read(&format!("-{a}.env"))));
        rows.push((format!("grep -{a} pat f"), Want::Allow));
        let dots = ".".repeat(n);
        rows.push((format!("grep -{dots}pat f"), Want::Allow));
        rows.push((format!("grep -{dots}.env pat f"), Want::Allow));
        rows.push((format!("grep -{} pat f", ".a".repeat(n)), Want::Allow));
    }
    for n in [199, 200, 201, 20000] {
        rows.push((
            format!("grep -i{}.example pat f", ".env".repeat(n)),
            Want::Allow,
        ));
        rows.push((format!("grep -i{} pat f", ".envrc".repeat(n)), Want::Allow));
        let word = format!("-i{}/.env", ".envrc".repeat(n));
        rows.push((format!("grep {word} pat f"), read(&word)));
        let word = format!("-if{}", ".env".repeat(n));
        rows.push((format!("grep {word} pat f"), read(&word)));
    }
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

/// `describe heredoc inside a command substitution (#5513)`.
#[test]
fn a_heredoc_inside_a_substitution_is_read_as_bash_reads_it() {
    let mut rows: Vec<(String, Want)> = Vec::new();
    for denied in [
        "echo \"$(cat <<E$(x)F\nx\nE$(x)F\n)\"; cat .env",
        "echo \"$(cat <<EOF\r\nx\nEOF\r\n)\"; cat .env",
        "echo \"$(cat <<EOF\n)\"; cat .env; \"\nEOF\n)\"",
        "echo \"$(cat <<E`x`F\nx\nE`x`F\n)\"; cat .env",
        "echo \"$(cat <<E${x}F\nx\nE${x}F\n)\"; cat .env",
        // The earlier tag reading is kept: bash never ends this heredoc, but it was denied and stays denied.
        "cat <<EOF\r\nx\nEOF\ncat .env",
        "cat <<EOF\r\ndata\r\nEOF\r\ncat .env",
        "cat <<E$(x)F\ndata\nE$(x)F\ncat .env",
    ] {
        rows.push((denied.into(), read(".env")));
    }
    for allowed in [
        "echo \"$(cat <<E$(x)F\nx\nE$(x)F\n)\"; echo ok",
        "echo \"$(cat <<EOF\r\nx\nEOF\r\n)\"; echo ok",
        "echo \"$(cat <<EOF\n)\"; echo ok; \"\nEOF\n)\"",
        "cat <<EOF\ncat .env\nEOF",
        "cat <<EOF\r\ncat .env\r\nEOF\r\n",
        "cat <<E$(x)F\ncat .env\nE$(x)F",
        "git commit -m \"$(cat <<'EOF'\nfix: handle the empty case\n\nBody line (with parens).\nEOF\n)\"",
        "git commit -m \"$(cat <<'EOF'\r\nfix: x\r\nEOF\r\n)\"",
    ] {
        rows.push((allowed.into(), Want::Allow));
    }
    holds(rows);
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
