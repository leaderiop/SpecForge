//! `specforge hook`: filtered before anything is loaded, and answered in each host's own protocol.

use serde_json::json;
use specforge_test_macros::test as specforge_test;

use super::{hook, project};

#[specforge_test(
    behavior = "run_a_hook",
    verify = "a hook whose event does not match exits before loading anything"
)]
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
    verify = "a guard hook denies in the protocol of Claude Code, and in the draft protocols of Codex and Cursor as their goldens have it"
)]
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

#[specforge_test(
    behavior = "run_a_hook",
    verify = "a hook that reads no graph and writes nothing is answered without compiling the project"
)]
fn a_hook_that_reads_no_graph_runs_without_a_compile() {
    let project = project(json!({}), json!({}));
    let write = json!({"hook_event_name": "PreToolUse", "tool_name": "Write", "session_id": "s1",
                       "cwd": project.root(),
                       "tool_input": {"file_path": project.root().join("m.spec"), "content": "x"}});
    let run = hook(
        &project,
        "t.write_guard",
        "claude-code",
        &["--event", "pre_tool", "--tools", "write"],
        &write,
    );
    let reply = run.out.json();
    assert_eq!(
        reply["hookSpecificOutput"]["permissionDecision"], "deny",
        "{reply}"
    );
    assert_eq!(run.passes, 0, "the project was compiled: {:?}", run.exports);
    let status = json!({"model": {"display_name": "M"}, "cwd": project.root()});
    let run = hook(
        &project,
        "t.statusline",
        "claude-code",
        &["--event", "statusline"],
        &status,
    );
    assert!(run.passes >= 1, "a graph-reading hook compiles the project");
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "an event over the size limit or not the host's JSON is answered by the hook's failure policy"
)]
fn an_unreadable_event_applies_the_failure_policy() {
    let project = project(json!({}), json!({}));
    let over = vec![b' '; crate::hook::STDIN_LIMIT as usize + 1];
    let denied = |route: &str, stdin: &[u8]| {
        let run = super::hook_bytes(
            &project,
            route,
            "claude-code",
            &["--event", "pre_tool", "--tools", "write"],
            stdin,
        );
        assert_eq!(run.out.code, 0, "{}", run.out.stderr);
        run.out.stdout.contains("\"deny\"")
    };
    // The spec guard and a hook declared on_failure deny fail closed; a route the project does not declare allows.
    assert!(denied("core.spec_guard", &over));
    assert!(denied("core.spec_guard", b"{ not json"));
    assert!(denied("t.write_guard", &over));
    assert!(!denied("t.nobody", &over));
}
