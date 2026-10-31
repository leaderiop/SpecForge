//! The config reload through the command the install wrote (ADR 0321): Claude Code's `FileChanged` runs it with the
//! changed file on stdin.

use serde_json::{Value, json};

use super::harness::{Project, taskboard};
use super::hooks_support::{Home, has_command, install_in, installed_command, run_hook};

fn event(p: &Project, file: &str, what: &str) -> String {
    let root = p.root().display().to_string();
    json!({"hook_event_name": "FileChanged", "session_id": "s1", "cwd": root,
           "file_path": format!("{root}/{file}"), "event": what})
    .to_string()
}

fn context(stdout: &str) -> String {
    let v: Value = serde_json::from_str(stdout).unwrap_or_else(|e| panic!("{e}: {stdout}"));
    assert_eq!(v["hookSpecificOutput"]["hookEventName"], "FileChanged");
    v["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

#[test]
fn a_changed_specforge_json_tells_the_session_the_settings() {
    let p = taskboard();
    p.settings(json!({"mode": "yolo", "hooks.community": true}));
    let home = Home::new();
    install_in(&p, "claude-code", &home);
    assert!(
        installed_command(&p, "claude-code", "gsd.config_reload")
            .contains("--paths specforge.json")
    );
    let run = run_hook(
        &p,
        "claude-code",
        "gsd.config_reload",
        &event(&p, "specforge.json", "change"),
        &home,
    );
    assert_eq!(run.code, 0, "{}", run.all());
    let said = context(&run.stdout);
    assert!(
        said.starts_with("GSD config reloaded (specforge.json updated):\n"),
        "{said}"
    );
    assert!(said.contains("  mode: yolo"), "{said}");
    assert!(said.contains("community=true"), "{said}");

    let run = run_hook(
        &p,
        "claude-code",
        "gsd.config_reload",
        &event(&p, "specforge.json", "unlink"),
        &home,
    );
    assert!(context(&run.stdout).starts_with("GSD config (specforge.json) was deleted."));

    let run = run_hook(
        &p,
        "claude-code",
        "gsd.config_reload",
        &event(&p, "other.json", "change"),
        &home,
    );
    assert_eq!(run.stdout.trim(), "");
}

#[test]
fn only_claude_code_has_a_file_changed_event() {
    let p = taskboard();
    let home = Home::new();
    install_in(&p, "codex", &home);
    install_in(&p, "cursor", &home);
    assert!(!has_command(&p, "codex", "gsd.config_reload"));
    assert!(!has_command(&p, "cursor", "gsd.config_reload"));
}
