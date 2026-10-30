//! The agent isolation and worktree path guards through the command the install wrote (D-r44-16).

use serde_json::{Value, json};

use super::harness::{Project, taskboard, taskboard_at};
use super::hooks_support::{
    HOSTS, Home, has_command, install_in, run_hook, tool_event, tool_name, verdict,
};
use super::prompts_support::{payload, prompt, text_of};

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

/// C1: @gsd/core runs its executors one after another in the checkout, and its execute prompts say to omit
/// `isolation="worktree"`; the installed guard must let that dispatch through under default settings.
#[test]
fn the_dispatch_the_execute_prompts_tell_the_agent_to_make_passes_the_guard() {
    let p = taskboard();
    let home = installed(&p);
    for (id, args) in [
        ("execute_phase", vec!["v1.0/01"]),
        ("execute_plan", vec!["v1.0/02/02-01"]),
    ] {
        let text = text_of(&payload(&prompt(&p, id, &args, "claude-code")));
        assert!(
            text.contains("Omit `isolation=\"worktree\"`")
                || text.contains("no worktree isolation")
                || text.contains("not part of @gsd/core"),
            "{id} no longer says the executors run in the checkout"
        );
    }
    for input in [
        json!({"subagent_type": "gsd-executor", "description": "Execute plan 02-01", "prompt": "go"}),
        json!({"subagent_type": "gsd-executor", "isolation": "none"}),
        json!({"subagent_type": "gsd-executor", "isolation": ""}),
    ] {
        let (denied, reason) = isolation(&p, &home, &input);
        assert!(!denied, "{input}: {reason}");
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
