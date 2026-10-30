//! The context monitor through the commands the install wrote (D-r44-12, D-r44-16). The statusline writes the
//! metrics in production; these tests write the session's state file the way it does.

use std::process::Command;

use serde_json::{Value, json};

use super::harness::{Project, taskboard};
use super::hooks_support::{
    HOSTS, Home, has_command, install_in, run_hook, tool_event, tool_name, verdict,
};

const STATE: &str = ".specforge/state/gsd__core/sessions/s1.json";

fn utc_now() -> String {
    let out = Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .expect("date");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// The statusline's write: `remaining` percent left, read just now.
fn metrics(p: &Project, remaining: u32) {
    let at = utc_now();
    let state = json!({"context": {"remaining_percentage": remaining, "used_pct": 100 - remaining, "at": at}});
    let mut now: Value = if p.exists(STATE) {
        serde_json::from_str(&p.read(STATE)).expect("a state file")
    } else {
        json!({})
    };
    now["context"] = state["context"].clone();
    p.write(STATE, &now.to_string());
}

fn installed(p: &Project) -> Home {
    let home = Home::new();
    for host in HOSTS {
        install_in(p, host, &home);
    }
    home
}

/// A shell tool use, the monitor's answer on `host`: the advice it injected.
fn use_tool(p: &Project, home: &Home, host: &str) -> String {
    let event = tool_event(
        p,
        host,
        "post",
        tool_name(host, "shell"),
        &json!({"command": "ls"}),
        Some(&json!({"stdout": ""})),
    );
    let run = run_hook(p, host, "gsd.context_monitor", &event, home);
    verdict(host, &run).2
}

fn compact(p: &Project, home: &Home, host: &str) {
    let event = json!({"hook_event_name": "PreCompact", "session_id": "s1",
                       "cwd": p.root().display().to_string()})
    .to_string();
    let run = run_hook(p, host, "gsd.context_compacted", &event, home);
    assert_eq!(run.code, 0, "{}", run.all());
}

/// P4: what the session's state holds is what the monitor warns about.
#[test]
fn the_monitor_reads_what_the_session_holds() {
    for host in ["claude-code", "codex"] {
        let p = taskboard();
        let home = installed(&p);
        metrics(&p, 30);
        let advice = use_tool(&p, &home, host);
        assert!(advice.contains("CONTEXT WARNING"), "{host}: {advice}");
        assert!(advice.contains("Remaining: 30%"), "{advice}");
    }
    let p = taskboard();
    let home = installed(&p);
    metrics(&p, 20);
    assert!(use_tool(&p, &home, "claude-code").contains("CONTEXT CRITICAL"));
}

#[test]
fn the_monitor_is_silent_without_metrics() {
    let p = taskboard();
    let home = installed(&p);
    for host in HOSTS {
        assert_eq!(use_tool(&p, &home, host), "", "{host}");
    }
    metrics(&p, 80);
    for host in HOSTS {
        assert_eq!(use_tool(&p, &home, host), "", "{host}: plenty left");
    }
}

#[test]
fn the_fifth_call_warns_again() {
    let p = taskboard();
    let home = installed(&p);
    metrics(&p, 30);
    assert!(use_tool(&p, &home, "claude-code").contains("CONTEXT WARNING"));
    // Four tool uses are debounced; the fifth after a warning warns again.
    for n in 2..=6 {
        let advice = use_tool(&p, &home, "claude-code");
        assert_eq!(advice.is_empty(), n < 6, "call {n}: {advice}");
    }
    let state: Value = serde_json::from_str(&p.read(STATE)).expect("state");
    assert!(state.to_string().len() < 1024, "{state}");
}

#[test]
fn critical_bypasses_the_debounce() {
    let p = taskboard();
    let home = installed(&p);
    metrics(&p, 30);
    assert!(use_tool(&p, &home, "claude-code").contains("WARNING"));
    assert_eq!(use_tool(&p, &home, "claude-code"), "", "debounced");
    metrics(&p, 20);
    assert!(use_tool(&p, &home, "claude-code").contains("CONTEXT CRITICAL"));
}

#[test]
fn metrics_before_a_compaction_are_ignored() {
    for host in ["claude-code", "codex"] {
        let p = taskboard();
        let home = installed(&p);
        metrics(&p, 20);
        assert!(use_tool(&p, &home, host).contains("CRITICAL"));
        compact(&p, &home, host);
        let state: Value = serde_json::from_str(&p.read(STATE)).expect("state");
        assert!(state.get("context").is_none(), "{state}");
        assert!(state["monitor"]["compacted_at"].is_string(), "{state}");
        assert_eq!(
            use_tool(&p, &home, host),
            "",
            "{host}: nothing left to read"
        );
        // The statusline renders again at once: that is the compaction's own render.
        metrics(&p, 20);
        assert_eq!(
            use_tool(&p, &home, host),
            "",
            "{host}: inside the grace window"
        );
    }
}

#[test]
fn thresholds_come_from_settings() {
    let p = taskboard();
    p.settings(
        json!({"hooks.context_warning_threshold": 50, "hooks.context_critical_threshold": 30}),
    );
    let home = installed(&p);
    metrics(&p, 45);
    assert!(use_tool(&p, &home, "claude-code").contains("CONTEXT WARNING"));
    metrics(&p, 28);
    assert!(use_tool(&p, &home, "claude-code").contains("CONTEXT CRITICAL"));
}

#[test]
fn an_invalid_pair_falls_back() {
    let p = taskboard();
    p.settings(
        json!({"hooks.context_warning_threshold": 20, "hooks.context_critical_threshold": 30}),
    );
    let home = installed(&p);
    metrics(&p, 40);
    assert_eq!(
        use_tool(&p, &home, "claude-code"),
        "",
        "the defaults: 40 is above 35"
    );
    metrics(&p, 32);
    let advice = use_tool(&p, &home, "claude-code");
    assert!(advice.contains("CONTEXT WARNING"), "{advice}");
}

#[test]
fn a_compaction_resets_even_when_warnings_are_off() {
    let p = taskboard();
    p.settings(json!({"hooks.context_warnings": false}));
    let home = installed(&p);
    assert!(!has_command(&p, "claude-code", "gsd.context_monitor"));
    assert!(has_command(&p, "claude-code", "gsd.context_compacted"));
    assert!(has_command(&p, "codex", "gsd.context_compacted"));
    assert!(!has_command(&p, "cursor", "gsd.context_compacted"));
    metrics(&p, 20);
    compact(&p, &home, "claude-code");
    let state: Value = serde_json::from_str(&p.read(STATE)).expect("state");
    assert!(state.get("context").is_none(), "{state}");
}
