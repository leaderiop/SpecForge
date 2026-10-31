//! Ported from gsd-core v1.18.0 tests/workflow-guard.test.cjs and tests/workflow-guard-registration.test.cjs, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! Upstream's guard cases run the hook in a temp git repo. The advisory cases decide on facts (the tool, the path, the
//! session) and are ported, and so is the `git add -f` block over the commands and the branch the host reads; the node
//! hook's crash posture is skipped with its reason. The registration suite drives upstream's installer, which the
//! install goldens replace.

use serde_json::{Map, Value, json};

use super::*;
use crate::hooks::accounting;
use crate::hooks::facts::PathFact;

const GUARD_SUITE: &str =
    include_str!("../../../../parity/upstream/v1.18.0/tests/workflow-guard.test.cjs");
const REGISTRATION_SUITE: &str =
    include_str!("../../../../parity/upstream/v1.18.0/tests/workflow-guard-registration.test.cjs");

fn edit_of(class: &str, file: &str, session: Option<Value>) -> Facts<'static> {
    Facts {
        host: "claude-code".into(),
        class: class.into(),
        tool: "Write".into(),
        input: json!({"file_path": file}),
        paths: vec![PathFact {
            path: file.into(),
            in_project: true,
            in_spec_root: file.starts_with(".planning/"),
            ..PathFact::default()
        }],
        session_state: session.and_then(|v| match v {
            Value::Object(m) => Some(m),
            _ => None,
        }),
        ..Facts::default()
    }
}

/// A shell call of `commands` (each split at `&&`, `;`, `|`, `||` and into words by whitespace) on `branch`.
fn on_branch(branch: &str, commands: &[&str]) -> Facts<'static> {
    let words = commands
        .iter()
        .flat_map(|c| c.split(['&', ';', '|']))
        .map(|c| c.split_whitespace().map(String::from).collect::<Vec<_>>())
        .filter(|w| !w.is_empty())
        .collect();
    Facts {
        host: "claude-code".into(),
        class: "shell".into(),
        tool: "Bash".into(),
        input: json!({"command": commands.join(" ")}),
        commands: words,
        git: Some(crate::hooks::facts::Git {
            toplevel: "/t".into(),
            git_dir: "/t/.git".into(),
            branch: branch.into(),
            status: None,
        }),
        ..Facts::default()
    }
}

fn denied(f: &Facts) -> bool {
    workflow_guard(f).decision == Some(Decision::Deny)
}

fn blocks(command: &str, branch: &str) -> bool {
    denied(&on_branch(branch, &[command]))
}

fn advises(f: &Facts) -> bool {
    let out = workflow_guard(f);
    assert_ne!(out.decision, Some(Decision::Deny), "the guard never blocks");
    out.context.is_some()
}

fn marked() -> Option<Value> {
    Some(json!({"workflow": {"command": "/gsd-fast", "at": "2026-10-30T10:00:00Z"}}))
}

fn prompt(text: &str) -> Facts<'static> {
    Facts {
        host: "claude-code".into(),
        prompt: Some(text.into()),
        now: "2026-10-30T10:00:00Z".into(),
        ..Facts::default()
    }
}

fn marker_command(text: &str) -> Option<String> {
    let patch = workflow_marker(&prompt(text)).state?;
    assert!(patch.project.is_none());
    patch.session?["workflow"]["command"]
        .as_str()
        .map(str::to_string)
}

/// The cases of `workflow-guard.test.cjs` this module runs, each asserted where it is written.
fn ported() -> Vec<String> {
    let mut ported = Vec::new();
    let mut port = |title: &str| ported.push(title.to_string());

    assert!(advises(&edit_of("write", "src/app.js", None)));
    port("WriteFile outside .planning/ gets the workflow advisory like Write");
    assert!(!advises(&edit_of("edit", ".planning/notes.md", None)));
    port("StrReplaceFile editing .planning/ passes silently");
    let mut shell = edit_of("shell", "x", None);
    shell.tool = "Shell".into();
    shell.input = json!({"command": "git status"});
    assert!(!advises(&shell));
    port("benign Shell command passes through");
    shell.input = json!({"command": "git status", "edit": [null]});
    assert!(!advises(&shell));
    port("benign command with a malformed edit field still passes (no over-block)");

    // The force-add block: the commands the host split, the branch it read.
    assert!(blocks("git add -f secrets.env", "agent-1"));
    port(
        "#2304: Kimi tool vocabulary engages the workflow guard > Bash (Claude vocabulary) still blocks \u{2014} normalization is additive",
    );
    for edit in [json!([null]), json!("nope"), Value::Null] {
        let mut f = on_branch("worktree-agent-2", &["git add -f secrets.env"]);
        f.input = json!({"command": "git add -f secrets.env", "edit": edit});
        assert!(denied(&f));
    }
    port(
        "#2304: Kimi tool vocabulary engages the workflow guard > #2547: a spurious malformed edit field does not disarm the force-add block > force-add still blocks with a spurious edit field (${label})",
    );
    for command in [
        "git -c core.hooksPath=/tmp/x add -f secrets.env",
        "git -cfoo.bar=1 add -f secrets.env",
        "git --no-optional-locks add -f secrets.env",
        "git --literal-pathspecs add -f secrets.env",
        "git --namespace=foo add -f secrets.env",
        "GIT_PAGER=cat git --no-replace-objects add -f secrets.env",
        "cd /tmp && git add --force secrets.env",
        "git add -Af x",
    ] {
        assert!(blocks(command, "worktree-agent-2"), "{command}");
        assert!(!blocks(command, "main"), "{command}");
    }
    port(
        "#3504: global-flag spellings of git add -f reach the shared classifier > force-add is blocked behind a global flag (${label})",
    );
    assert!(!blocks(
        "git -c core.hooksPath=/tmp/x add -f secrets.env",
        "main"
    ));
    port(
        "#3504: global-flag spellings of git add -f reach the shared classifier > the same spellings on a non-agent branch stay allowed (no over-block)",
    );
    ported
}

const GUARD_SKIPPED: &[(&str, &str)] = &[
    (
        "#2304: Kimi tool vocabulary engages the workflow guard > Shell force-add",
        "runtime: Kimi's tool vocabulary has no renderer here",
    ),
    (
        "#2304: Kimi tool vocabulary engages the workflow guard > module-qualified",
        "runtime: Kimi's tool vocabulary has no renderer here",
    ),
    (
        "#3504: internal error fails closed for the enabled force-add guard > ",
        "runtime: fail-closed on an internal error: a hook here does not crash into a policy",
    ),
    (
        "#3504: global-flag spellings of git add -f reach the shared classifier > -C resolves the probed repo",
        "diverges: [cwd-branch] the branch is the hook's cwd's, not the `-C` target's",
    ),
];

const REGISTRATION_SKIPPED: &[(&str, &str)] = &[
    (
        "workflow-guard hook registration (#1767) > ",
        "installer: upstream's installer registers a node command; the install goldens and tests/cli/hooks_workflow.rs cover this port's registration",
    ),
    (
        "hook registration completeness anti-pattern guard > ",
        "installer: scans upstream's installer source for node commands",
    ),
    (
        "atomic write temp-tracking — shared Set parity guard > ",
        "installer: upstream's installer writes Cursor's hooks.json; the install goldens cover it",
    ),
    (
        "cursor hook registration — behavioral guard (module-owned hooks) > ",
        "installer: upstream's installer writes Cursor's lifecycle hooks; the install goldens cover them",
    ),
];

#[test]
fn workflow_guard_accounts_for_every_upstream_case() {
    assert_eq!(ported().len(), 8);
    accounting::account(GUARD_SUITE, &ported(), GUARD_SKIPPED);
    accounting::account(REGISTRATION_SUITE, &[], REGISTRATION_SKIPPED);
}

#[test]
fn an_unmarked_session_is_advised() {
    let out = workflow_guard(&edit_of("write", "src/app.js", None));
    let text = out.context.expect("an advisory");
    assert!(text.contains("app.js") && text.contains("/gsd-fast") && text.contains("/gsd-quick"));
    assert!(advises(&edit_of(
        "edit",
        "src/app.js",
        Some(json!({"other": 1}))
    )));
    assert!(advises(&edit_of(
        "write",
        "src/app.js",
        Some(json!({"workflow": {}}))
    )));
}

#[test]
fn a_marked_session_is_silent() {
    assert!(!advises(&edit_of("write", "src/app.js", marked())));
    assert!(!advises(&edit_of("edit", "src/app.js", marked())));
}

#[test]
fn what_upstream_exempts_is_silent() {
    for file in [
        ".planning/STATE.md",
        "x/.gitignore",
        ".env.local",
        "CLAUDE.md",
        "docs/AGENTS.md",
        "GEMINI.md",
        ".claude/settings.json",
    ] {
        assert!(!advises(&edit_of("write", file, None)), "{file}");
    }
    let mut sub = edit_of("write", "src/app.js", None);
    sub.input = json!({"file_path": "src/app.js", "is_subagent": true});
    assert!(!advises(&sub));
    sub.input = json!({"file_path": "src/app.js", "is_subagent": false});
    assert!(advises(&sub));
    for class in ["read", "shell", "search"] {
        assert!(!advises(&edit_of(class, "src/app.js", None)), "{class}");
    }
    let mut none = edit_of("write", "x", None);
    none.input = json!({});
    none.paths.clear();
    assert!(!advises(&none));
}

#[test]
fn a_gsd_prompt_marks_the_session() {
    assert_eq!(
        marker_command("/gsd-plan-phase 2").as_deref(),
        Some("/gsd-plan-phase")
    );
    assert_eq!(
        marker_command("$gsd-fast fix typo").as_deref(),
        Some("/gsd-fast")
    );
    assert_eq!(
        marker_command("  /gsd-quick").as_deref(),
        Some("/gsd-quick")
    );
    let patch = workflow_marker(&prompt("/gsd-fast"))
        .state
        .expect("a patch");
    assert_eq!(
        patch.session,
        Some(json!({"workflow": {"command": "/gsd-fast", "at": "2026-10-30T10:00:00Z"}}))
    );
}

#[test]
fn any_other_prompt_changes_nothing() {
    for text in [
        "fix the bug",
        "please run /gsd-fast",
        "/gsd-",
        "/gsd",
        "/gsdx-fast",
        "/GSD-fast",
        "/gsd-fast!",
        "",
    ] {
        assert!(workflow_marker(&prompt(text)).state.is_none(), "{text:?}");
    }
    let mut none = prompt("x");
    none.prompt = None;
    assert!(workflow_marker(&none).state.is_none());
}

#[test]
fn the_marker_feeds_the_guard() {
    let patch = workflow_marker(&prompt("/gsd-quick"))
        .state
        .expect("a patch");
    let Some(Value::Object(session)) = patch.session else {
        panic!("an object")
    };
    let state: Map<String, Value> = session;
    let mut f = edit_of("write", "src/app.js", None);
    f.session_state = Some(state);
    assert!(!advises(&f));
}

/// Upstream's shell walk and its negatives.
#[test]
fn only_a_git_add_with_a_force_flag_before_the_double_dash_is_a_force_add() {
    for branch in ["agent-1", "worktree-agent-2"] {
        assert!(blocks("git add --force x", branch));
        assert!(blocks("/usr/bin/git add -f x", branch));
        assert!(blocks("git -C /r add -Af x", branch));
    }
    for command in [
        "git add -- -f",
        "git add x",
        "git add --follow",
        "git commit -f",
        "echo git add -f",
        "gitx add -f x",
        "git status; ls -f",
    ] {
        assert!(!blocks(command, "agent-1"), "{command}");
    }
    // The splitter has unquoted the word, so the value holds a space: upstream's `/^-c\S*=/` still skips it.
    let mut f = on_branch("agent-1", &[]);
    f.commands = vec![
        ["git", "-cuser.name=a b", "add", "-f", "x"]
            .map(String::from)
            .to_vec(),
    ];
    assert!(denied(&f), "a glued -c value may hold spaces");
    assert!(!blocks("git add -f x", "feature"), "not an agent branch");
    assert!(!blocks("git add -f x", ""), "detached");
    let mut f = on_branch("agent-1", &["git add -f x"]);
    f.git = None;
    assert!(!denied(&f), "no repository");
}
