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
    assert_eq!(reply["permission"], "deny", "{reply}");
    assert!(
        reply["agent_message"].as_str().unwrap().contains("shrink"),
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

#[cfg(unix)]
#[specforge_test(
    behavior = "run_a_hook",
    verify = "a hook is told an outside path's real path, and with reads_git its toplevel and whether it is inside a git directory"
)]
fn a_hook_is_told_the_real_path_and_toplevel_of_an_outside_path() {
    use std::process::Command;
    let git = |dir: &std::path::Path, args: &[&str]| {
        Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .current_dir(dir)
            .status()
            .is_ok_and(|s| s.success())
    };
    if Command::new("git").arg("--version").output().is_err() {
        return;
    }
    let project = project(json!({}), json!({}));
    let root = project.root().to_path_buf();
    let other = tempfile::tempdir().unwrap();
    let other_root = std::fs::canonicalize(other.path()).unwrap();
    std::fs::create_dir_all(other_root.join("src")).unwrap();
    assert!(
        git(&root, &["init", "-q", "-b", "main"])
            && git(&root, &["commit", "-q", "--allow-empty", "-m", "init"])
            && git(&other_root, &["init", "-q", "-b", "main"])
    );
    std::os::unix::fs::symlink(other_root.join("src"), root.join("mainsrc")).unwrap();
    let _git =
        specforge_project::testing::install(std::sync::Arc::new(specforge_git::GitCli::default()));
    let canonical_root = std::fs::canonicalize(&root).unwrap();
    let write = |file: &std::path::Path| {
        json!({"hook_event_name": "PreToolUse", "tool_name": "Write", "session_id": "s1", "cwd": root,
               "tool_input": {"file_path": file, "content": "x"}})
    };
    let run = hook(
        &project,
        "t.write_guard",
        "claude-code",
        &["--event", "pre_tool", "--tools", "write"],
        &write(&root.join("mainsrc/a.ts")),
    );
    let input = run.input.expect("the guard was called");
    let path = &input["paths"][0];
    assert_eq!(path["in_project"], false, "{path}");
    assert_eq!(
        path["path"],
        root.join("mainsrc/a.ts").display().to_string()
    );
    assert_eq!(
        path["real"],
        other_root.join("src/a.ts").display().to_string()
    );
    assert_eq!(path["toplevel"], other_root.display().to_string());
    assert_eq!(
        input["git"]["toplevel"],
        canonical_root.display().to_string()
    );
    assert_eq!(input["git"]["branch"], "main");
    let run = hook(
        &project,
        "t.write_guard",
        "claude-code",
        &["--event", "pre_tool", "--tools", "write"],
        &write(&other_root.join(".git/config")),
    );
    let path = &run.input.unwrap()["paths"][0];
    assert_eq!(path["in_git_dir"], true, "{path}");
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "a file_changed hook fires for the file it names, with what happened to it"
)]
fn a_file_changed_hook_fires_for_its_file_only() {
    let project = project(json!({}), json!({}));
    let args = ["--event", "file_changed", "--paths", "specforge.json"];
    let changed = |file: &str| {
        let event = json!({"hook_event_name": "FileChanged", "cwd": project.root(),
                           "file_path": project.root().join(file), "event": "change"});
        hook(&project, "t.settings_reload", "claude-code", &args, &event)
    };
    let run = changed("specforge.json");
    assert_eq!(run.out.code, 0, "{}", run.out.stderr);
    assert_eq!(
        run.out.json()["hookSpecificOutput"]["additionalContext"],
        "change specforge.json"
    );
    let run = changed("other.json");
    assert_eq!(run.out.stdout, "");
    assert!(run.exports.is_empty(), "{:?}", run.exports);
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "specforge hook never exits 2 on its own refusals: an unknown host, event or usage error exits 1"
)]
fn an_unknown_event_exits_1() {
    let project = project(json!({}), json!({}));
    for (host, event) in [("claude-code", "nope"), ("nope", "pre_tool")] {
        let run = hook(
            &project,
            "t.write_guard",
            host,
            &["--event", event],
            &json!({}),
        );
        assert_eq!(run.out.code, 1, "{host} {event}");
        assert!(
            run.out.stderr.contains("unknown host"),
            "{}",
            run.out.stderr
        );
    }
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "a writing hook is answered over a compiled project"
)]
fn a_writing_hook_is_answered_over_a_compiled_project() {
    let project = project(json!({}), json!({}));
    let write = json!({"hook_event_name": "PostToolUse", "tool_name": "Write", "session_id": "s1",
                       "cwd": project.root(),
                       "tool_input": {"file_path": project.root().join("m.spec"), "content": "x"}});
    let run = hook(
        &project,
        "t.record_hook",
        "claude-code",
        &["--event", "post_tool", "--tools", "write", "--paths", "**"],
        &write,
    );
    assert_eq!(run.out.code, 0, "{}", run.out.stderr);
    assert!(
        run.passes >= 1,
        "a writing hook compiles: {:?}",
        run.exports
    );
    assert!(
        run.exports.iter().any(|e| e == "hook__record_hook"),
        "{:?}",
        run.exports
    );
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "the spec guard compiles before it judges"
)]
fn the_spec_guard_compiles_before_it_judges() {
    let project = project(json!({}), json!({}));
    let write = json!({"hook_event_name": "PreToolUse", "tool_name": "Write", "session_id": "s1",
        "cwd": project.root(),
        "tool_input": {"file_path": project.root().join("o.spec"),
                       "content": "override O1 \"Waive\" {\n  by \"agent\"\n  phase 03\n}\n"}});
    let run = hook(
        &project,
        "core.spec_guard",
        "claude-code",
        &[
            "--event",
            "pre_tool",
            "--tools",
            "write,edit",
            "--paths",
            "**/*.spec",
        ],
        &write,
    );
    assert!(
        run.passes >= 1,
        "the spec guard compiles: {:?}",
        run.exports
    );
    let reply = run.out.json();
    assert_eq!(
        reply["hookSpecificOutput"]["permissionDecision"], "deny",
        "{reply}"
    );
    assert!(reply.to_string().contains("written by a human"), "{reply}");
}
