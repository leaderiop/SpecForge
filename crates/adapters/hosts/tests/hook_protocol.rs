#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]
//! Each host's recorded hook stdin, the neutral event it reads into, a neutral answer and what the host is written
//! (`tests/protocol/<host>/*.json`).

use std::path::Path;

use serde_json::Value;
use specforge_hosts::renderer;
use specforge_ops::host::{HookAnswer, Host, MISSING_GUARD, Rendered, RenderedHook};
use specforge_protocol_types::{Decision, HookEvent, OnFailure};

fn check(host: Host) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/protocol")
        .join(host.as_str());
    let adapter = renderer(host).expect("an adapter");
    let mut cases = 0;
    for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.unwrap().path();
        let case: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let stdin = serde_json::to_vec(&case["stdin"]).unwrap();
        let event = adapter
            .read_event(&stdin)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            serde_json::to_value(&event).unwrap(),
            case["event"],
            "{name}: the event read"
        );
        let answer: HookAnswer = serde_json::from_value(case["answer"].clone()).unwrap();
        let reply = adapter.write_answer(&event, &answer);
        assert_eq!(
            reply.stdout,
            case["stdout"].as_str().unwrap(),
            "{name}: stdout"
        );
        assert_eq!(
            i64::from(reply.exit_code),
            case["exit_code"].as_i64().unwrap(),
            "{name}: exit code"
        );
        cases += 1;
    }
    assert!(cases >= 5, "{} holds {cases} cases", dir.display());
}

#[specforge_test_macros::test(
    behavior = "run_a_hook",
    verify = "each host reads its hook stdin into the neutral event and writes the neutral answer as the host expects it"
)]
fn claude_code_reads_and_answers_its_hook_protocol() {
    check(Host::ClaudeCode);
}

#[test]
fn a_host_input_that_is_no_event_is_refused() {
    let adapter = renderer(Host::ClaudeCode).unwrap();
    assert!(adapter.read_event(b"not json").is_err());
    assert!(adapter.read_event(b"[1]").is_err());
    assert!(
        adapter
            .read_event(br#"{"hook_event_name":"Elicitation"}"#)
            .is_err()
    );
}

#[test]
fn codex_reads_and_answers_its_hook_protocol() {
    check(Host::Codex);
}

#[test]
fn cursor_reads_and_answers_its_hook_protocol() {
    check(Host::Cursor);
}

fn write_guard(on_failure: OnFailure) -> RenderedHook {
    on_event(HookEvent::PreTool, on_failure)
}

fn on_event(event: HookEvent, on_failure: OnFailure) -> RenderedHook {
    RenderedHook {
        route: "t.write_guard".into(),
        description: String::new(),
        event,
        tools: vec!["write".into()],
        paths: Vec::new(),
        timeout_ms: 5_000,
        on_failure,
    }
}

const GUARD: &str = "command -v specforge >/dev/null 2>&1 || { echo 'specforge is not on PATH: the hook t.write_guard cannot run, so it denies' >&2; exit 2; }; specforge hook t.write_guard --host codex";

#[specforge_test_macros::test(
    behavior = "install_into_a_host",
    verify = "a hook whose policy is deny blocks with exit 2 when specforge is not on PATH, and an allow hook's line is the plain command"
)]
fn a_deny_hook_line_fails_closed_without_the_binary() {
    let line = write_guard(OnFailure::Deny).command(Host::Codex);
    assert!(line.starts_with(GUARD), "{line}");
    assert!(line.ends_with("# specforge-install"), "{line}");
    #[cfg(unix)]
    {
        let ran = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(&line)
            .env("PATH", "")
            .output()
            .unwrap();
        assert_eq!(ran.status.code(), Some(2));
        assert_eq!(
            String::from_utf8_lossy(&ran.stderr),
            "specforge is not on PATH: the hook t.write_guard cannot run, so it denies\n"
        );
    }
    let plain = write_guard(OnFailure::Allow).command(Host::Codex);
    assert!(plain.starts_with("specforge hook "), "{plain}");
}

/// The guarded line with a stub `specforge` on PATH passes the binary's stdout and exit through, in every sh-like
/// shell this machine has.
#[cfg(unix)]
#[test]
fn a_deny_hook_line_runs_the_binary_when_it_is_present() {
    let dir = std::env::temp_dir().join(format!("specforge-stub-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let stub = dir.join("specforge");
    std::fs::write(&stub, "#!/bin/sh\necho stub-ran\nexit 0\n").unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let line = write_guard(OnFailure::Deny).command(Host::Codex);
    let path = format!("{}:/usr/bin:/bin", dir.display());
    for shell in ["/bin/sh", "/bin/bash", "/bin/zsh"] {
        if !Path::new(shell).exists() {
            continue;
        }
        let ran = std::process::Command::new(shell)
            .args(["-f", "-c"]) // -f: zsh skips the user's rc files, which reset PATH
            .arg(&line)
            .env("PATH", &path)
            .output()
            .unwrap();
        assert_eq!(ran.status.code(), Some(0), "{shell}");
        assert_eq!(
            String::from_utf8_lossy(&ran.stdout),
            "stub-ran\n",
            "{shell}"
        );
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[specforge_test_macros::test(
    behavior = "install_into_a_host",
    verify = "a deny hook is fail-closed without sh syntax on Windows: Claude Code's handler carries onFailure block, Codex's carries commandWindows, Cursor's Windows line is the plain command"
)]
fn a_deny_hook_is_fail_closed_without_sh_syntax_on_windows() {
    let deny = write_guard(OnFailure::Deny);
    let plain = |host: Host| format!("specforge hook t.write_guard --host {}", host.as_str());
    // Claude Code: the plain line on every OS; the handler asks the host to block on failure.
    assert!(
        deny.command_on(Host::ClaudeCode, false)
            .starts_with(&plain(Host::ClaudeCode))
    );
    assert!(
        deny.command_on(Host::ClaudeCode, true)
            .starts_with(&plain(Host::ClaudeCode))
    );
    // Cursor: the guard on Unix, the plain line when install runs on Windows.
    assert!(
        deny.command_on(Host::Cursor, false)
            .starts_with(MISSING_GUARD)
    );
    assert!(
        deny.command_on(Host::Cursor, true)
            .starts_with(&plain(Host::Cursor))
    );
    // Codex: the guard in `command`, the plain line in `commandWindows`.
    assert!(
        deny.command_on(Host::Codex, true)
            .starts_with(MISSING_GUARD)
    );
    assert_eq!(
        deny.plain_command(Host::Codex).split(' ').next(),
        Some("specforge")
    );
    let handler = |host: Host, file: &str| {
        let tree = renderer(host).unwrap().render(&Rendered {
            host,
            agents: Vec::new(),
            skills: Vec::new(),
            slashes: Vec::new(),
            hooks: vec![deny.clone()],
        });
        let merged = tree.merged.iter().find(|m| m.path == file).unwrap();
        merged
            .entries
            .iter()
            .find_map(|e| e.value.get("hooks").map(|h| h[0].clone()))
            .unwrap()
    };
    let claude = handler(Host::ClaudeCode, ".claude/settings.json");
    assert_eq!(claude["onFailure"], "block");
    assert!(
        claude["command"]
            .as_str()
            .unwrap()
            .starts_with("specforge hook ")
    );
    let codex = handler(Host::Codex, ".codex/hooks.json");
    assert!(
        codex["command"]
            .as_str()
            .unwrap()
            .starts_with(MISSING_GUARD)
    );
    assert_eq!(codex["commandWindows"], deny.plain_command(Host::Codex));
    // An allow hook gets neither field.
    let allow = write_guard(OnFailure::Allow);
    assert!(
        allow
            .command_on(Host::Codex, false)
            .starts_with("specforge hook ")
    );
}

#[specforge_test_macros::test(
    behavior = "install_into_a_host",
    verify = "a stop or subagent_stop hook never gets the fail-closed line"
)]
fn a_deny_stop_hook_line_is_the_plain_command() {
    for event in [
        HookEvent::Stop,
        HookEvent::SubagentStop,
        HookEvent::PostTool,
        HookEvent::SessionStart,
    ] {
        let hook = on_event(event, OnFailure::Deny);
        assert!(!hook.fail_closed(), "{event:?}");
        for host in Host::ALL {
            assert!(
                hook.command_on(host, false).starts_with("specforge hook "),
                "{host:?} {event:?}"
            );
        }
    }
    for event in [
        HookEvent::PreTool,
        HookEvent::PreCommit,
        HookEvent::UserPrompt,
    ] {
        assert!(on_event(event, OnFailure::Deny).fail_closed(), "{event:?}");
    }
}

fn answer_of(
    host: Host,
    event: HookEvent,
    decision: Decision,
    reason: &str,
    context: &str,
) -> Value {
    let adapter = renderer(host).unwrap();
    let event = specforge_ops::host::HostHookEvent {
        event,
        host: host.as_str().into(),
        ..Default::default()
    };
    let answer = HookAnswer {
        decision: Some(decision),
        reason: Some(reason.into()),
        context: Some(context.into()),
        ..Default::default()
    };
    let reply = adapter.write_answer(&event, &answer);
    serde_json::from_str(&reply.stdout).unwrap()
}

#[specforge_test_macros::test(
    behavior = "run_a_hook",
    verify = "Cursor wires a user_prompt hook on beforeSubmitPrompt and answers a denied prompt with continue false"
)]
fn cursor_renders_user_prompt_as_before_submit_prompt() {
    let cursor = renderer(Host::Cursor).unwrap();
    assert_eq!(
        cursor.event_name(HookEvent::UserPrompt),
        Some("beforeSubmitPrompt")
    );
    let reply = answer_of(
        Host::Cursor,
        HookEvent::UserPrompt,
        Decision::Deny,
        "no",
        "c",
    );
    assert_eq!(
        reply,
        serde_json::json!({"continue": false, "user_message": "no"})
    );
}

#[specforge_test_macros::test(
    behavior = "run_a_hook",
    verify = "Cursor answers a denied tool call with permission deny, its reason to the user and the agent"
)]
fn cursor_denies_a_tool_call_with_permission_deny() {
    for decision in [Decision::Deny, Decision::Ask] {
        let reply = answer_of(Host::Cursor, HookEvent::PreTool, decision, "r", "c");
        assert_eq!(
            reply,
            serde_json::json!({"permission": "deny", "user_message": "r", "agent_message": "r",
                               "additional_context": "c"})
        );
    }
}

#[specforge_test_macros::test(
    behavior = "run_a_hook",
    verify = "Claude Code wires a file_changed hook on FileChanged, matching the files by name, and reads the changed file and what happened"
)]
fn claude_code_reads_and_answers_file_changed() {
    let adapter = renderer(Host::ClaudeCode).unwrap();
    let read = adapter
        .read_event(
            br#"{"hook_event_name":"FileChanged","cwd":"/p","session_id":"s","file_path":"/p/specforge.json","event":"change"}"#,
        )
        .unwrap();
    assert_eq!(read.event, HookEvent::FileChanged);
    assert_eq!(read.file.as_deref(), Some("/p/specforge.json"));
    assert_eq!(read.change.as_deref(), Some("change"));
    assert_eq!(
        adapter.event_name(HookEvent::FileChanged),
        Some("FileChanged")
    );
    let reply = answer_of(
        Host::ClaudeCode,
        HookEvent::FileChanged,
        Decision::Allow,
        "r",
        "c",
    );
    assert_eq!(
        reply,
        serde_json::json!({"hookSpecificOutput": {"additionalContext": "c", "hookEventName": "FileChanged"}})
    );
    for host in [Host::Codex, Host::Cursor] {
        assert_eq!(
            renderer(host).unwrap().event_name(HookEvent::FileChanged),
            None
        );
    }
}

#[specforge_test_macros::test(
    behavior = "run_a_hook",
    verify = "Cursor cannot deny after a tool ran: a post_tool deny reaches the agent as context only"
)]
fn cursor_drops_a_post_tool_deny_to_context() {
    let reply = answer_of(
        Host::Cursor,
        HookEvent::PostTool,
        Decision::Deny,
        "no",
        "ctx",
    );
    assert!(reply.get("permission").is_none(), "{reply}");
    assert!(reply.to_string().contains("ctx"), "{reply}");
}
