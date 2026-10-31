//! The workflow marker and guard through the commands the install wrote (D-r44-11, D-r44-16).

use serde_json::{Value, json};

use super::harness::{Project, taskboard};
use super::hooks_support::{
    HOSTS, Home, has_command, install_in, run_hook, tool_event, tool_name, verdict,
};

fn installed(p: &Project) -> Home {
    let home = Home::new();
    for host in HOSTS {
        install_in(p, host, &home);
    }
    home
}

/// A user prompt as claude-code and codex write it (`session` `s1`).
fn prompt_event(p: &Project, text: &str) -> String {
    json!({"hook_event_name": "UserPromptSubmit", "session_id": "s1",
           "cwd": p.root().display().to_string(), "prompt": text})
    .to_string()
}

fn mark(p: &Project, home: &Home, host: &str, text: &str) {
    let run = run_hook(p, host, "gsd.workflow_marker", &prompt_event(p, text), home);
    assert_eq!(run.code, 0, "{}", run.all());
}

fn edit(p: &Project, home: &Home, host: &str, file: &str) -> String {
    let input = json!({"file_path": file, "content": "x"});
    let event = tool_event(p, host, "pre", tool_name(host, "write"), &input, None);
    let run = run_hook(p, host, "gsd.workflow_guard", &event, home);
    verdict(host, &run).2
}

fn session_state(p: &Project) -> Value {
    let file = ".specforge/state/gsd__core/sessions/s1.json";
    assert!(p.exists(file), "no {file}");
    serde_json::from_str(&p.read(file)).expect("a JSON state file")
}

fn guarded() -> Project {
    let p = taskboard();
    p.settings(json!({"hooks.workflow_guard": true}));
    p
}

/// P2: the prompt hook leaves the command in the session's state file, on both hosts that have the event.
#[test]
fn a_gsd_prompt_marks_the_session() {
    for host in ["claude-code", "codex"] {
        let p = guarded();
        let home = installed(&p);
        mark(&p, &home, host, "/gsd-plan-phase 2");
        assert_eq!(
            session_state(&p)["workflow"]["command"],
            "/gsd-plan-phase",
            "{host}"
        );
    }
}

#[test]
fn an_edit_outside_a_workflow_is_warned() {
    for host in HOSTS {
        let p = guarded();
        let home = installed(&p);
        let advice = edit(&p, &home, host, "src/app.js");
        assert!(advice.contains("WORKFLOW ADVISORY"), "{host}: {advice}");
        assert!(advice.contains("/gsd-fast") && advice.contains("/gsd-quick"));
        mark(&p, &home, "claude-code", "plain words, not a command");
        let advice = edit(&p, &home, host, "src/app.js");
        assert!(advice.contains("WORKFLOW ADVISORY"), "{host}: {advice}");
    }
}

#[test]
fn an_edit_inside_a_workflow_is_silent() {
    for host in ["claude-code", "codex"] {
        let p = guarded();
        let home = installed(&p);
        mark(&p, &home, host, "$gsd-fast fix the typo");
        assert_eq!(edit(&p, &home, host, "src/app.js"), "", "{host}");
    }
    // The next command replaces the marker (U51); the session's other keys stay.
    let p = guarded();
    let home = installed(&p);
    mark(&p, &home, "claude-code", "/gsd-fast");
    mark(&p, &home, "claude-code", "/gsd-quick x");
    assert_eq!(session_state(&p)["workflow"]["command"], "/gsd-quick");
}

#[test]
fn a_spec_root_edit_is_silent_without_a_workflow() {
    let p = guarded();
    let home = installed(&p);
    assert_eq!(edit(&p, &home, "claude-code", ".planning/STATE.md"), "");
    assert_eq!(edit(&p, &home, "claude-code", ".gitignore"), "");
}

#[test]
fn the_guard_is_absent_by_default() {
    let p = taskboard();
    let _home = installed(&p);
    for host in HOSTS {
        assert!(!has_command(&p, host, "gsd.workflow_guard"), "{host}");
    }
}

#[test]
fn the_marker_is_rendered_on_cursor_as_before_submit_prompt() {
    let p = guarded();
    let home = Home::new();
    let report = install_in(&p, "cursor", &home);
    install_in(&p, "claude-code", &home);
    install_in(&p, "codex", &home);
    assert!(has_command(&p, "claude-code", "gsd.workflow_marker"));
    assert!(has_command(&p, "codex", "gsd.workflow_marker"));
    assert!(has_command(&p, "cursor", "gsd.workflow_marker"));
    assert!(has_command(&p, "cursor", "gsd.workflow_guard"));
    assert!(
        !report.stdout.contains("hook gsd.workflow_marker"),
        "the install report does not drop it: {}",
        report.all()
    );
}

/// T15 (#3504): a force-add in a shell command, on a real repository's agent branch, is refused; on another branch it
/// is not. Skipped without git.
#[test]
fn a_force_add_on_an_agent_branch_is_refused() {
    let force = |p: &Project, home: &Home, host: &str, command: &str| {
        let input = json!({"command": command});
        let event = tool_event(p, host, "pre", tool_name(host, "shell"), &input, None);
        verdict(host, &run_hook(p, host, "gsd.workflow_guard", &event, home))
    };
    for (branch, blocked) in [
        ("agent-x", true),
        ("worktree-agent-x", true),
        ("main", false),
    ] {
        let p = guarded();
        let made = std::process::Command::new("git")
            .args(["init", "-q", "-b", branch])
            .current_dir(p.root())
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .status();
        if !made.is_ok_and(|s| s.success()) {
            return;
        }
        let home = installed(&p);
        for host in HOSTS {
            for command in [
                "git add -f secret.txt",
                "cd sub && git -c core.hooksPath=/tmp/x add --force secret.txt",
            ] {
                let (denied, reason, _) = force(&p, &home, host, command);
                assert_eq!(denied, blocked, "{host} {branch} {command}: {reason}");
                if blocked {
                    assert!(reason.contains("must not run git add -f"), "{reason}");
                }
            }
            assert!(!force(&p, &home, host, "git add secret.txt").0, "{host}");
        }
    }
}
