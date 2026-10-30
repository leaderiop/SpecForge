//! The statusline through the command the install wrote (D-r44-13, D-r44-16): Claude Code's `statusLine` setting
//! runs it with the status JSON on stdin and shows the line it prints.

use serde_json::{Value, json};

use super::harness::{Project, project, taskboard};
use super::hooks_support::{
    HOSTS, Home, has_command, install_in, installed_command, run_hook, run_installed, tool_event,
    tool_name, verdict,
};

const STATE: &str = ".specforge/state/gsd__core/sessions/s1.json";

fn golden(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../parity/goldens/taskboard")
        .join(format!("{name}.json"));
    serde_json::from_str(&std::fs::read_to_string(&path).expect("a golden")).expect("JSON")
}

fn installed(p: &Project) -> Home {
    let home = Home::new();
    for host in HOSTS {
        install_in(p, host, &home);
    }
    home
}

/// Claude Code's status JSON: 200k window, `remaining` percent left.
fn status(p: &Project, remaining: u32, usage: Option<Value>) -> String {
    let mut window = json!({"context_window_size": 200_000, "remaining_percentage": remaining});
    if let Some(u) = usage {
        window["current_usage"] = u;
    }
    json!({"session_id": "s1", "cwd": p.root().display().to_string(), "model": {"display_name": "Opus 4"},
           "workspace": {"current_dir": p.root().display().to_string()}, "context_window": window})
    .to_string()
}

fn plain(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            chars.by_ref().find(|d| *d == 'm');
        } else {
            out.push(c);
        }
    }
    out
}

/// The line the installed statusline prints for `status`, with the escape codes removed.
fn line(p: &Project, home: &Home, status: &str) -> String {
    let run = run_hook(p, "claude-code", "gsd.statusline", status, home);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(
        run.stdout.trim_end().lines().count(),
        1,
        "one line: {}",
        run.stdout
    );
    // The kernel writes a hook's control characters out as text, so the line carries no colour codes.
    assert!(
        !run.stdout.contains("\\u{1b}") && !run.stdout.contains('\x1b'),
        "{}",
        run.all()
    );
    plain(run.stdout.trim_end())
}

fn state(p: &Project) -> Value {
    serde_json::from_str(&p.read(STATE)).expect("a state file")
}

/// A twin whose phase 01 passed verification: one phase of four is complete.
fn verified() -> Project {
    let p = taskboard();
    assert_eq!(p.verify_phase("v1.0/01"), 7);
    p
}

/// P3: the line holds the model, the milestone, the first open phase, the milestone's counts and the meter.
#[test]
fn statusline_answers_on_claude_code() {
    let p = verified();
    let home = installed(&p);
    let out = line(&p, &home, &status(&p, 70, None));
    let progress = golden("state-json")["progress"].clone();
    assert!(out.starts_with("Opus 4 │ "), "{out}");
    assert!(out.contains("v1.0"), "{out}");
    assert!(out.contains("persistence"), "{out}");
    let counts = format!(
        "{}/{}",
        progress["completed_phases"], progress["total_phases"]
    );
    assert!(out.contains(&counts), "{counts} in {out}");
    assert!(out.contains(&format!("{}%", progress["percent"])), "{out}");
    assert!(out.contains("36%"), "the meter: {out}");
    assert_eq!(state(&p)["context"]["remaining_percentage"], 64);
}

#[test]
fn the_progress_is_the_milestones_not_the_plan_percent() {
    let p = verified();
    let home = installed(&p);
    let out = line(&p, &home, &status(&p, 70, None));
    let plans = golden("progress-json")["percent"].to_string();
    assert_eq!(plans, "67");
    assert!(out.contains("25%") && !out.contains("67%"), "{out}");
}

#[test]
fn the_compact_format_drops_the_name_and_bar() {
    let p = verified();
    p.settings(json!({"statusline.state_format": "compact"}));
    let home = installed(&p);
    let out = line(&p, &home, &status(&p, 70, None));
    assert!(out.contains("v1.0 · P1/4 · "), "{out}");
    assert!(!out.contains("█") || out.matches('█').count() <= 4, "{out}");
}

#[test]
fn the_meter_moves_with_context_position() {
    let p = verified();
    let home = installed(&p);
    let end = line(&p, &home, &status(&p, 70, None));
    assert!(
        end.find("36%")
            > end.find(
                &p.root()
                    .file_name()
                    .expect("a name")
                    .to_string_lossy()
                    .to_string()
            )
    );
    p.settings(json!({"statusline.context_position": "front"}));
    let front = line(&p, &home, &status(&p, 70, None));
    assert!(front.find("36%") < front.find(" │ "), "{front}");
}

#[test]
fn tokens_show_when_set() {
    let p = verified();
    let usage =
        json!({"input_tokens": 1000, "cache_read_input_tokens": 150_000, "output_tokens": 5000});
    let home = installed(&p);
    assert!(!line(&p, &home, &status(&p, 70, Some(usage.clone()))).contains("(156k)"));
    p.settings(json!({"statusline.show_context_tokens": true}));
    assert!(line(&p, &home, &status(&p, 70, Some(usage))).contains("% (156k)"));
}

#[test]
fn the_last_command_shows_when_set() {
    let p = verified();
    p.settings(json!({"statusline.show_last_command": true}));
    let home = installed(&p);
    assert!(!line(&p, &home, &status(&p, 70, None)).contains("last:"));
    let prompt = json!({"hook_event_name": "UserPromptSubmit", "session_id": "s1",
                        "cwd": p.root().display().to_string(), "prompt": "/gsd-plan-phase 2"})
    .to_string();
    let run = run_hook(&p, "claude-code", "gsd.workflow_marker", &prompt, &home);
    assert_eq!(run.code, 0, "{}", run.all());
    let out = line(&p, &home, &status(&p, 70, None));
    assert!(out.ends_with("last: /gsd-plan-phase"), "{out}");
}

#[test]
fn the_active_workstream_shows() {
    let p = project("workstreams");
    let home = installed(&p);
    let run = p.run(&["gsd", "workstreams", "switch", "--name", "auth"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let out = line(&p, &home, &status(&p, 70, None));
    assert!(out.contains("ws auth"), "{out}");
}

#[test]
fn show_git_is_inert() {
    let p = verified();
    let home = installed(&p);
    let before = line(&p, &home, &status(&p, 70, None));
    p.settings(json!({"statusline.show_git": true, "statusline.show_state_freshness": true}));
    assert_eq!(line(&p, &home, &status(&p, 70, None)), before);
}

#[test]
fn statusline_is_not_rendered_on_codex_or_cursor() {
    let p = verified();
    let home = Home::new();
    for host in ["codex", "cursor"] {
        let report = install_in(&p, host, &home);
        assert!(!has_command(&p, host, "gsd.statusline"), "{host}");
        assert!(
            report.stdout.contains("hook gsd.statusline"),
            "{host}: {}",
            report.all()
        );
    }
    install_in(&p, "claude-code", &home);
    assert!(has_command(&p, "claude-code", "gsd.statusline"));
    let command = installed_command(&p, "claude-code", "gsd.statusline");
    assert!(command.contains("--host claude-code"), "{command}");
}

/// P4 again, end to end: the statusline writes the metrics and the monitor warns off them.
#[test]
fn the_monitor_reads_the_statusline_metrics() {
    let p = verified();
    let home = installed(&p);
    line(&p, &home, &status(&p, 20, None));
    let event = tool_event(
        &p,
        "claude-code",
        "post",
        tool_name("claude-code", "shell"),
        &json!({"command": "ls"}),
        Some(&json!({"stdout": ""})),
    );
    let run = run_hook(&p, "claude-code", "gsd.context_monitor", &event, &home);
    let (_, _, advice) = verdict("claude-code", &run);
    assert!(advice.contains("CONTEXT CRITICAL"), "{advice}");
    // The installed line is plain text a shell can run: no host, no escape needed.
    let command = installed_command(&p, "claude-code", "gsd.statusline");
    let again = run_installed(&p, &command, &status(&p, 20, None), &home);
    assert_eq!(again.code, 0, "{}", again.all());
}
