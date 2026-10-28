//! `specforge hook`: filtered before anything is loaded, and answered in each host's own protocol.

use serde_json::json;
use specforge_test_macros::test as specforge_test;

use super::{hook, project};

#[specforge_test(
    behavior = "run_a_hook",
    verify = "a hook whose event does not match exits before loading anything"
)]
#[ignore = "K9 T10"]
fn a_hook_that_does_not_match_exits_before_loading() {
    let project = project(json!({}), json!({}));
    let read = json!({"hook_event_name": "PreToolUse", "tool_name": "Read", "session_id": "s1",
                      "cwd": project.root(), "tool_input": {"file_path": project.root().join("m.spec")}});
    let run = hook(
        &project,
        "t.write_guard",
        "claude-code",
        &["--event", "pre_tool", "--tools", "write"],
        &read,
    );
    assert_eq!(run.out.code, 0);
    assert!(
        run.exports.is_empty(),
        "no component was loaded: {:?}",
        run.exports
    );
    assert!(run.out.stderr.is_empty(), "{}", run.out.stderr);
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "a guard hook denies in the protocol of Claude Code, Codex and Cursor"
)]
#[ignore = "K9 T10"]
fn a_guard_hook_denies_on_claude_code_codex_and_cursor() {
    let project = project(json!({}), json!({}));
    let file = project.root().join("m.spec");
    let write = |event: &str| {
        json!({"hook_event_name": event, "tool_name": "Write", "session_id": "s1", "cwd": project.root(),
               "tool_input": {"file_path": file, "content": "x"}})
    };
    for (host, event) in [("claude-code", "PreToolUse"), ("codex", "PreToolUse")] {
        let run = hook(
            &project,
            "t.write_guard",
            host,
            &["--event", "pre_tool", "--tools", "write"],
            &write(event),
        );
        assert_eq!(run.out.code, 0, "{host}: {}", run.out.stderr);
        let reply = run.out.json();
        assert_eq!(
            reply["hookSpecificOutput"]["hookEventName"], "PreToolUse",
            "{host}: {reply}"
        );
        assert_eq!(
            reply["hookSpecificOutput"]["permissionDecision"], "deny",
            "{host}: {reply}"
        );
        assert!(
            reply["hookSpecificOutput"]["permissionDecisionReason"]
                .as_str()
                .unwrap()
                .contains("shrink")
        );
    }
    let run = hook(
        &project,
        "t.write_guard",
        "cursor",
        &["--event", "pre_tool", "--tools", "write"],
        &write("preToolUse"),
    );
    let reply = run.out.json();
    assert_eq!(reply["block"], true, "{reply}");
    assert!(
        reply["reason"].as_str().unwrap().contains("shrink"),
        "{reply}"
    );
}
