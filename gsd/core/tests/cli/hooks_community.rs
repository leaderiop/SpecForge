//! The community hooks through the commands the install wrote (D-r44-14, D-r44-15, D-r44-16), and the one test that
//! runs every hook line an install writes.

use serde_json::{Value, json};

use super::commands_support::gsd;
use super::harness::{Project, taskboard};
use super::hooks_support::{
    HOSTS, Home, all_commands, has_command, install_in, parses, run_hook, tool_event, tool_name,
    verdict,
};

const COMMUNITY: [&str; 3] = [
    "gsd.validate_commit",
    "gsd.phase_boundary",
    "gsd.session_state",
];

fn community(extra: Value) -> Project {
    let p = taskboard();
    let mut settings = json!({"hooks.community": true});
    if let (Some(s), Some(e)) = (settings.as_object_mut(), extra.as_object()) {
        s.extend(e.clone());
    }
    p.settings(settings);
    p
}

fn installed(p: &Project) -> Home {
    let home = Home::new();
    for host in HOSTS {
        install_in(p, host, &home);
    }
    home
}

/// What `git commit` with this command line is told on `host`: `(denied, reason)`.
fn commit(p: &Project, home: &Home, host: &str, command: &str) -> (bool, String) {
    let input = json!({ "command": command });
    let event = tool_event(p, host, "pre", tool_name(host, "shell"), &input, None);
    let run = run_hook(p, host, "gsd.validate_commit", &event, home);
    let (denied, reason, _) = verdict(host, &run);
    (denied, reason)
}

/// P5: a subject that is no Conventional Commit is refused with upstream's code, on every host.
#[test]
fn bad_commit_subject_is_refused() {
    let p = community(json!({}));
    let home = installed(&p);
    for host in HOSTS {
        let (denied, reason) = commit(&p, &home, host, "git commit -m \"added stuff\"");
        assert!(denied, "{host}");
        assert!(
            reason.contains("CONVENTIONAL_COMMITS_VIOLATION"),
            "{host}: {reason}"
        );
        assert!(
            reason.contains("Valid types: feat, fix, docs"),
            "{host}: {reason}"
        );
        let (denied, _) = commit(&p, &home, host, "git commit -m \"feat(auth): add login\"");
        assert!(!denied, "{host}");
        let long = format!("git commit -m \"feat: {}\"", "a".repeat(70));
        let (denied, reason) = commit(&p, &home, host, &long);
        assert!(
            denied && reason.contains("COMMIT_SUBJECT_TOO_LONG"),
            "{host}: {reason}"
        );
        // the subject is the first line of a heredoc message
        let heredoc = "git commit -m \"$(cat <<'EOF'\nfeat(auth): add login\n\nbody\nEOF\n)\"";
        assert!(!commit(&p, &home, host, heredoc).0, "{host}");
        // not a commit at all
        assert!(!commit(&p, &home, host, "git status").0, "{host}");
    }
}

#[test]
fn a_commit_without_a_message_is_allowed() {
    let p = community(json!({}));
    let home = installed(&p);
    p.write("msg.txt", "added stuff\n");
    for host in HOSTS {
        for command in [
            "git commit -F msg.txt",
            "git commit --fixup HEAD",
            "git commit --amend --no-edit",
        ] {
            assert!(!commit(&p, &home, host, command).0, "{host}: {command}");
        }
        assert!(
            !commit(&p, &home, host, "git commit -m \"fixup! feat: x\"").0,
            "{host}"
        );
    }
}

#[test]
fn configured_commit_types_are_accepted() {
    let p = community(json!({"hooks.commit_types": ["deps", "Bad Type"]}));
    let home = installed(&p);
    for host in HOSTS {
        assert!(
            !commit(&p, &home, host, "git commit -m \"deps: bump serde\"").0,
            "{host}"
        );
        let (denied, reason) = commit(&p, &home, host, "git commit -m \"wip: bump serde\"");
        assert!(
            denied && reason.contains("chore, deps."),
            "{host}: {reason}"
        );
    }
}

/// A Write or Edit of a spec file, after the fact, on `host`.
fn spec_write(p: &Project, home: &Home, host: &str, file: &str) -> String {
    let input = json!({"file_path": p.root().join(file), "content": "x"});
    let event = tool_event(
        p,
        host,
        "post",
        tool_name(host, "write"),
        &input,
        Some(&json!("ok")),
    );
    let run = run_hook(p, host, "gsd.phase_boundary", &event, home);
    verdict(host, &run).2
}

#[test]
fn a_spec_write_reminds_that_state_is_computed() {
    let p = community(json!({}));
    let home = installed(&p);
    for host in HOSTS {
        let context = spec_write(&p, &home, host, "spec/plans.spec");
        assert!(context.contains("spec/plans.spec"), "{host}: {context}");
        // the command the reminder names is run on the same project
        let named = "`specforge gsd progress-report`";
        assert!(context.contains(named), "{host}: {context}");
        let run = p.run(&["gsd", "progress-report"]);
        assert_eq!(run.code, 0, "{}", run.all());
        // a file outside the spec root draws no reminder
        assert_eq!(spec_write(&p, &home, host, "src/app.js"), "", "{host}");
    }
}

/// A host's session start, in its own shape.
fn session_event(p: &Project, host: &str) -> String {
    let cwd = p.root().display().to_string();
    if host == "cursor" {
        json!({"hook_event_name": "sessionStart", "conversation_id": "s1", "workspace_roots": [cwd]})
    } else {
        json!({"hook_event_name": "SessionStart", "session_id": "s1", "cwd": cwd, "source": "startup"})
    }
    .to_string()
}

/// The text a session starts with on `host`.
fn started(p: &Project, home: &Home, host: &str) -> String {
    let run = run_hook(p, host, "gsd.session_state", &session_event(p, host), home);
    assert_eq!(run.code, 0, "{}", run.all());
    let v: Value =
        serde_json::from_str(&run.stdout).unwrap_or_else(|e| panic!("{e}: {}", run.all()));
    let found = [
        &v["hookSpecificOutput"]["additionalContext"],
        &v["additional_context"],
        &v["additionalContext"],
    ]
    .into_iter()
    .find_map(|x| x.as_str());
    found
        .unwrap_or_else(|| panic!("no context on {host}: {}", run.stdout))
        .to_string()
}

#[test]
fn session_start_injects_the_progress_line() {
    let p = community(json!({"mode": "yolo"}));
    assert_eq!(p.verify_phase("v1.0/01"), 7);
    for args in [
        &[
            "workstreams",
            "create",
            "--name",
            "ui",
            "--purpose",
            "the UI",
        ][..],
        &["workstreams", "switch", "--name", "ui"][..],
    ] {
        let run = gsd(&p, args[0], &args[1..]);
        assert_eq!(run.code, 0, "{}", run.all());
    }
    let state: Value =
        serde_json::from_str(&p.read(".specforge/state/gsd__core/project.json")).expect("JSON");
    assert_eq!(state, json!({"active_workstream": "ui"}));
    let home = installed(&p);
    for host in HOSTS {
        let text = started(&p, &home, host);
        assert!(
            text.starts_with("## Project State Reminder"),
            "{host}: {text}"
        );
        assert!(
            text.contains("v1.0 Taskboard 1.0: 1/4 phases complete (25%)"),
            "{host}: {text}"
        );
        assert!(
            text.contains("phase 02 persistence: 1 plan(s), 0 summary(ies)"),
            "{host}: {text}"
        );
        assert!(text.contains("Active workstream: ui"), "{host}: {text}");
        assert!(text.contains("\"mode\": \"yolo\""), "{host}: {text}");
    }
}

#[test]
fn session_start_without_a_project_suggests_new_project() {
    let p = super::harness::project("empty");
    p.settings(json!({"hooks.community": true}));
    let home = installed(&p);
    for host in HOSTS {
        let text = started(&p, &home, host);
        assert!(
            text.contains("No project found - suggest /gsd-new-project if starting new work."),
            "{host}: {text}"
        );
    }
}

#[test]
fn community_hooks_are_absent_by_default() {
    let p = taskboard();
    let _home = installed(&p);
    for host in HOSTS {
        for route in COMMUNITY {
            assert!(!has_command(&p, host, route), "{host}: {route}");
        }
    }
}

/// The event that suits `route`, on `host`.
fn event_for(p: &Project, host: &str, route: &str) -> String {
    let file = json!({"file_path": p.root().join("notes.txt"), "content": "x"});
    let read = json!({"file_path": p.root().join("notes.txt")});
    let tool = |phase: &str, class: &str, input: &Value| {
        tool_event(
            p,
            host,
            phase,
            tool_name(host, class),
            input,
            Some(&json!("ok")),
        )
    };
    let cwd = p.root().display().to_string();
    match route {
        "gsd.secret_read_guard" => tool("pre", "read", &read),
        "gsd.injection_scanner" => tool("post", "read", &read),
        "gsd.agent_isolation_guard" => tool("pre", "agent", &json!({"prompt": "go"})),
        "gsd.validate_commit" => tool("pre", "shell", &json!({"command": "git commit -m \"feat: x\""})),
        "gsd.context_monitor" | "gsd.phase_boundary" => tool("post", "write", &file),
        "gsd.workflow_marker" => json!({"hook_event_name": "UserPromptSubmit", "session_id": "s1", "cwd": cwd, "prompt": "/gsd-fast x"}).to_string(),
        "gsd.context_compacted" => json!({"hook_event_name": "PreCompact", "session_id": "s1", "cwd": cwd}).to_string(),
        "gsd.session_state" => session_event(p, host),
        "gsd.statusline" => json!({"session_id": "s1", "cwd": cwd, "model": {"display_name": "Opus 4"}}).to_string(),
        _ => tool("pre", "write", &file),
    }
}

/// Every bool on, three hosts: every gsd hook line the install wrote parses, runs once and answers.
#[test]
fn every_installed_hook_answers() {
    let p = community(json!({
        "hooks.workflow_guard": true,
        "hooks.context_warnings": true,
        "workflow.use_worktrees": true,
    }));
    let home = installed(&p);
    for (host, lines) in [("claude-code", 15), ("codex", 12), ("cursor", 12)] {
        let commands: Vec<String> = all_commands(&p, host)
            .into_iter()
            .filter(|c| c.starts_with("specforge hook gsd."))
            .collect();
        assert_eq!(commands.len(), lines, "{host}: {commands:?}");
        for command in &commands {
            assert!(parses(command), "{host}: {command}");
            let route = command.split_whitespace().nth(2).unwrap_or_default();
            let run = run_hook(&p, host, route, &event_for(&p, host, route), &home);
            assert_eq!(run.code, 0, "{host} {route}: {}", run.all());
            if route != "gsd.statusline" && !run.stdout.trim().is_empty() {
                serde_json::from_str::<Value>(run.stdout.trim().lines().next().unwrap_or_default())
                    .unwrap_or_else(|_| panic!("{host} {route}: {}", run.stdout));
            }
        }
    }
}
