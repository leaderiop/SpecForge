//! The agent isolation and worktree path guards through the command the install wrote (D-r44-16).

use serde_json::{Value, json};

use super::harness::{Project, taskboard, taskboard_at};
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

fn dispatch(p: &Project, host: &str, input: &Value) -> String {
    tool_event(p, host, "pre", tool_name(host, "agent"), input, None)
}

/// The decision of the isolation guard for `input`, on claude-code.
fn isolation(p: &Project, home: &Home, input: &Value) -> (bool, String) {
    let run = run_hook(
        p,
        "claude-code",
        "gsd.agent_isolation_guard",
        &dispatch(p, "claude-code", input),
        home,
    );
    let (denied, reason, _) = verdict("claude-code", &run);
    (denied, reason)
}

#[test]
fn an_executor_without_a_worktree_is_denied() {
    let p = taskboard();
    let home = installed(&p);
    for input in [
        json!({"subagent_type": "gsd-executor"}),
        json!({"subagent_type": "gsd-executor", "isolation": "none"}),
        json!({"subagent_type": "gsd-executor", "isolation": ""}),
    ] {
        let (denied, reason) = isolation(&p, &home, &input);
        assert!(denied, "{input}");
        assert!(
            reason.contains("Add isolation=\"worktree\" to the Agent() call"),
            "{reason}"
        );
        assert!(reason.contains("gsd.agent_isolation_guard"), "{reason}");
    }
}

#[test]
fn an_executor_in_a_worktree_is_allowed() {
    let p = taskboard();
    let home = installed(&p);
    assert!(
        !isolation(
            &p,
            &home,
            &json!({"subagent_type": "gsd-executor", "isolation": "worktree"})
        )
        .0
    );
}

#[test]
fn a_planner_is_never_denied() {
    let p = taskboard();
    let home = installed(&p);
    for kind in [
        "gsd-planner",
        "gsd-code-reviewer",
        "gsd-verifier",
        "general-purpose",
    ] {
        assert!(
            !isolation(&p, &home, &json!({"subagent_type": kind})).0,
            "{kind}"
        );
    }
}

/// The guard is Claude Code's: Cursor's dispatch tool has no `isolation` parameter, and Codex has no such tool.
#[test]
fn the_isolation_guard_is_rendered_where_the_host_has_a_dispatch_tool() {
    let p = taskboard();
    let home = installed(&p);
    assert!(has_command(&p, "claude-code", "gsd.agent_isolation_guard"));
    assert!(has_command(&p, "cursor", "gsd.agent_isolation_guard"));
    assert!(!has_command(&p, "codex", "gsd.agent_isolation_guard"));
    let run = run_hook(
        &p,
        "cursor",
        "gsd.agent_isolation_guard",
        &dispatch(&p, "cursor", &json!({"subagent_type": "gsd-executor"})),
        &home,
    );
    assert!(!verdict("cursor", &run).0, "{}", run.all());
}

#[test]
fn isolation_is_absent_while_use_worktrees_is_off() {
    let p = taskboard();
    let home = installed(&p);
    let input = json!({"subagent_type": "gsd-executor"});
    assert!(isolation(&p, &home, &input).0, "on by default");
    p.settings(json!({"workflow.use_worktrees": false}));
    assert!(!isolation(&p, &home, &input).0, "off: the hook is a no-op");
}

/// A project that is a subagent's worktree: `<tmp>/.claude/worktrees/agent-x`, with the main checkout above it.
fn in_worktree() -> (Project, Home, std::path::PathBuf) {
    let p = taskboard_at(".claude/worktrees/agent-x");
    let home = installed(&p);
    let main = p
        .root()
        .ancestors()
        .nth(3)
        .expect("the main checkout")
        .to_path_buf();
    (p, home, main)
}

fn write_to(p: &Project, host: &str, file: &std::path::Path) -> String {
    let input = json!({"file_path": file, "content": "x"});
    tool_event(p, host, "pre", tool_name(host, "write"), &input, None)
}

#[test]
fn a_write_outside_the_worktree_is_denied() {
    let (p, home, main) = in_worktree();
    for host in HOSTS {
        let run = run_hook(
            &p,
            host,
            "gsd.worktree_path_guard",
            &write_to(&p, host, &main.join("src/x.rs")),
            &home,
        );
        let (denied, reason, _) = verdict(host, &run);
        assert!(denied, "{host}: {}", run.all());
        assert!(
            reason.contains("src/x.rs") && reason.contains("agent-x"),
            "{host}: {reason}"
        );
    }
}

#[test]
fn a_write_inside_the_worktree_or_outside_every_checkout_is_allowed() {
    let (p, home, _) = in_worktree();
    let elsewhere = std::path::Path::new("/tmp/gsd-r44-scratch.txt");
    for host in HOSTS {
        for file in [
            p.root().join("spec/new.spec"),
            p.root().join("src/y.rs"),
            elsewhere.to_path_buf(),
        ] {
            let run = run_hook(
                &p,
                host,
                "gsd.worktree_path_guard",
                &write_to(&p, host, &file),
                &home,
            );
            assert!(
                !verdict(host, &run).0,
                "{host} {}: {}",
                file.display(),
                run.all()
            );
        }
    }
}

#[test]
fn a_write_outside_a_project_that_is_not_a_worktree_is_allowed() {
    let p = taskboard();
    let home = installed(&p);
    let above = p.root().parent().expect("a parent").join("src/x.rs");
    for host in HOSTS {
        let run = run_hook(
            &p,
            host,
            "gsd.worktree_path_guard",
            &write_to(&p, host, &above),
            &home,
        );
        assert!(!verdict(host, &run).0, "{host}: {}", run.all());
    }
}
