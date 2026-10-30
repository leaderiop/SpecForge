//! Ported from gsd-core v1.18.0 tests/gsd-validate-commit-crash-policy.test.cjs and tests/gsd-validate-commit-sigpipe.test.cjs, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! The two suites spawn the shell script; here the verdict is a function of the message. What they test of the verdict
//! (a conforming and a non-conforming commit, an unknown type, a configured type, the last of a very long list) is
//! ported; what they test of bash, node, the environment and the copying of the hooks directory is skipped with its
//! reason. The script's own branches (the types, the scope, the limit, `hooks.commit_types`, fixup and squash) and the
//! two hooks that have no suite follow.

use serde_json::json;

use super::*;
use crate::hooks::accounting;
use crate::hooks::facts::PathFact;

mod opt_in;

const CRASH_SUITE: &str = include_str!(
    "../../../../parity/upstream/v1.18.0/tests/gsd-validate-commit-crash-policy.test.cjs"
);
const SIGPIPE_SUITE: &str =
    include_str!("../../../../parity/upstream/v1.18.0/tests/gsd-validate-commit-sigpipe.test.cjs");

pub(super) fn commit(message: Option<&str>) -> Facts<'static> {
    Facts {
        class: "shell".into(),
        commit_message: message.map(str::to_string),
        ..Facts::default()
    }
}

pub(super) fn verdict(message: &str, extra: &[&str]) -> HookOutput {
    let extra: Vec<String> = extra.iter().map(|t| t.to_string()).collect();
    validate_commit(&commit(Some(message)), &extra)
}

pub(super) fn denied(message: &str, extra: &[&str]) -> bool {
    verdict(message, extra).decision == Some(Decision::Deny)
}

pub(super) fn reason(message: &str) -> String {
    verdict(message, &[]).reason.unwrap_or_default()
}

#[test]
fn crash_policy_controls_are_the_verdict() {
    // CONTROL A: a conforming commit passes
    assert_eq!(verdict("feat: add thing", &[]), HookOutput::allow());
    // CONTROL B: a non-conforming one is blocked with upstream's code
    assert!(reason("wibble wobble").starts_with("CONVENTIONAL_COMMITS_VIOLATION"));
    assert!(denied("wibble wobble", &[]));
}

const CRASH_SKIPPED: &[(&str, &str)] = &[
    (
        "gsd-validate-commit.sh: #3838 could-not-run sites are surfaced, not swallowed > DEFECT (classifier)",
        "runtime: node failing to run the classifier; the host classifies the command and there is no node",
    ),
    (
        "gsd-validate-commit.sh: #3838 could-not-run sites are surfaced, not swallowed > DEFECT (config read)",
        "runtime: node failing to read .planning/config.json; the setting is read by the SDK and a failed hook is allowed",
    ),
    (
        "gsd-validate-commit.sh: #3838 could-not-run sites are surfaced, not swallowed > DEFECT (command extraction)",
        "runtime: node failing to parse the payload; the host parses the event and hands the hook its message",
    ),
];

#[test]
fn crash_policy_accounts_for_every_upstream_case() {
    let ported = [
        "CONTROL A: node available, conforming commit -> exit 0, no block payload",
        "CONTROL B: node available, non-conforming commit -> exit 2 with block payload",
    ]
    .map(String::from);
    accounting::account(CRASH_SUITE, &ported, CRASH_SKIPPED);
}

/// The type list of the sigpipe suite's last row: `zz0type` … `zz<count - 1>type`.
fn many(count: usize) -> Vec<String> {
    (0..count).map(|i| format!("zz{i}type")).collect()
}

fn with_many(message: &str, count: usize) -> HookOutput {
    validate_commit(&commit(Some(message)), &many(count))
}

#[test]
fn a_large_commit_types_config_is_read_to_its_end() {
    let count = 7000;
    // accepted at a payload that overflows a pipe buffer
    assert_eq!(
        with_many("feat(auth): add login flow", count).decision,
        Some(Decision::Allow)
    );
    // the gate is not weakened
    let out = with_many("nope(auth): not a configured type", count);
    assert_eq!(out.decision, Some(Decision::Deny));
    // the last configured type is accepted, so the whole list was read
    assert_eq!(
        with_many(
            &format!("zz{}type: use the last configured type", count - 1),
            count
        )
        .decision,
        Some(Decision::Allow)
    );
    // limit - 1, limit, limit + 1 on upstream's compiled-regex ceiling
    for n in [6050, 6051, 6052] {
        assert_eq!(
            with_many("feat(auth): add login flow", n).decision,
            Some(Decision::Allow),
            "{n}"
        );
    }
}

const SIGPIPE_SKIPPED: &[(&str, &str)] = &[
    (
        "#4429 — gsd-validate-commit.sh under a large commit_types config > CONTROL: ",
        "runtime: controls on bash's regex size cap and a `head -1` pipeline of the script, neither of which this port has",
    ),
    (
        "#4429 — subprocess statuses must not be inherited from the environment > ",
        "env: the script's status variables inherited from the environment; the hook reads no environment",
    ),
    (
        "#4429 follow-up — makeHookLayout must survive a concurrently-deleted staging directory > ",
        "installer: the copying of the hooks directory into a test layout, a helper of the suite",
    ),
];

#[test]
fn sigpipe_accounts_for_every_upstream_case() {
    let ported = [
        "a conforming subject is accepted at a payload that overflows the pipe buffer",
        "the gate is not weakened: an unknown type is still blocked",
        "a CONFIGURED extra type is accepted — proves the config is actually read",
        "conforming subject accepted with ${count} configured types",
    ]
    .map(String::from);
    accounting::account(SIGPIPE_SUITE, &ported, SIGPIPE_SKIPPED);
}

#[test]
fn every_builtin_type_is_accepted_and_each_fails_in_another() {
    for t in BUILTIN_TYPES {
        assert!(!denied(&format!("{t}: a change"), &[]), "{t}");
        assert!(!denied(&format!("{t}(scope): a change"), &[]), "{t}");
    }
    assert!(denied("added stuff", &[]));
    assert!(denied("wip: a change", &[]));
    let r = reason("wip: a change");
    assert!(
        r.contains("Valid types: feat, fix, docs, style, refactor, perf, test, build, ci, chore."),
        "{r}"
    );
}

#[test]
fn the_subject_is_type_scope_colon_space_text() {
    for ok in [
        "feat: x",
        "feat(auth): add login flow",
        "feat(a)(b): both parentheses",
        "feat(a): b): the scope backtracks",
        "feat:  two spaces",
        "feat:\ttab",
        "fix(api-v2): hyphen",
    ] {
        assert!(!denied(ok, &[]), "{ok}");
    }
    for bad in [
        "",
        "feat",
        "feat:",
        "feat: ",
        "feat:x",
        "feat : x",
        "feat(): x",
        "feat(a) : x",
        "feat(a: x",
        "Feat: x",
        "FEAT: x",
        "1feat: x",
        "-feat: x",
        "feat_x: x",
        " feat: x",
    ] {
        // an empty message is no message: the host sends none for it
        if bad.is_empty() {
            assert!(!denied(bad, &[]));
        } else {
            assert!(denied(bad, &[]), "{bad:?}");
        }
    }
}

#[test]
fn the_subject_is_the_first_line_and_at_most_72_characters() {
    let line = |n: usize| format!("feat: {}", "a".repeat(n - 6));
    assert!(!denied(&line(72), &[]));
    let long = verdict(&line(73), &[]);
    assert_eq!(long.decision, Some(Decision::Deny));
    assert_eq!(
        long.reason.as_deref(),
        Some("COMMIT_SUBJECT_TOO_LONG: Commit subject must be 72 characters or less.")
    );
    // characters, not bytes
    assert!(!denied(&format!("feat: {}", "é".repeat(66)), &[]));
    // only the first line is the subject
    assert!(!denied(&format!("feat: short\n\n{}", "b".repeat(200)), &[]));
    assert!(denied("body only\nfeat: not the subject", &[]));
    // the type is judged before the length
    assert!(
        reason(&format!("nope: {}", "a".repeat(100))).starts_with("CONVENTIONAL_COMMITS_VIOLATION")
    );
}

#[test]
fn configured_types_extend_the_built_in_ones() {
    assert!(denied("deps: bump", &[]));
    assert!(!denied("deps: bump", &["deps"]));
    assert!(reason("wip: x").contains("chore."));
    let r = verdict("wip: x", &["deps", "ops"]).reason.unwrap();
    assert!(r.contains("chore, deps, ops."), "{r}");
    // a name that is not `^[a-z][a-z0-9-]*$` is ignored, a duplicate is listed once
    for bad in ["Deps", "1deps", "de ps", "deps!", "", "feat"] {
        let r = verdict("wip: x", &[bad]).reason.unwrap();
        assert!(r.contains("chore."), "{bad}: {r}");
    }
    assert!(denied("Deps: bump", &["Deps"]));
}

/// `validate_commit` on a message with the command line it came from.
pub(super) fn with_command(message: &str, command: &str) -> bool {
    let f = Facts {
        input: serde_json::json!({ "command": command }),
        ..commit(Some(message))
    };
    validate_commit(&f, &[]).decision == Some(Decision::Deny)
}

#[test]
fn a_heredoc_message_is_judged_by_its_body() {
    let heredoc = |body: &str| format!("$(cat <<'EOF'\n{body}\nEOF\n)");
    assert!(!denied(&heredoc("feat(auth): add login flow\n\nbody"), &[]));
    assert!(!denied(&heredoc("\nfeat: leading blank line"), &[]));
    assert!(denied(&heredoc("added stuff"), &[]));
    assert!(denied(&heredoc(&format!("feat: {}", "a".repeat(80))), &[]));
    for opener in ["<<\"EOF\"", "<<-'EOF'", "<<\\EOF"] {
        assert!(
            !denied(&format!("$(cat {opener}\nfeat: x\nEOF\n)"), &[]),
            "{opener}"
        );
    }
    // an opener that is not one word is just a first line, and not a subject
    assert!(denied("$(cat <<'A B'\nfeat: x\nA B\n)", &[]));
    assert!(denied("$(cat <<\nfeat: x", &[]));
}

/// REVIEW-r44 M2: the shapes upstream blocks, because git does not get the body the text shows.
#[test]
fn a_heredoc_the_shell_would_change_is_judged_by_its_opener() {
    // a bare delimiter: bash expands the body (`$UNSET`, `${LONG}`), so git gets something else
    for opener in ["<<EOF", "<<-EOF"] {
        assert!(
            denied(&format!("$(cat {opener}\nfeat: $UNSET\nEOF\n)"), &[]),
            "{opener}"
        );
    }
    // something after the closing paren (upstream's adjacency guard)
    assert!(denied("$(cat <<'EOF'\nfeat: x\nEOF\n)$(echo junk)", &[]));
    assert!(denied("$(cat <<'EOF'\nfeat: x\nEOF\n)junk", &[]));
    // the body never closes
    assert!(denied("$(cat <<'EOF'\nfeat: x", &[]));
    let ok = "$(cat <<'EOF'\n\nfeat: x\nEOF\n)";
    assert!(!with_command(
        ok,
        "git commit -m \"$(cat <<'EOF'\n\nfeat: x\nEOF\n)\""
    ));
    // `--cleanup=verbatim` keeps the blank first line, so git's subject is empty; `whitespace`/`default` strip it
    assert!(with_command(
        ok,
        "git commit --cleanup=verbatim -m \"$(cat <<'EOF'\n\nfeat: x\nEOF\n)\""
    ));
    assert!(!with_command(
        ok,
        "git commit --cleanup=whitespace -m \"$(cat <<'EOF'\n\nfeat: x\nEOF\n)\""
    ));
    // a single-quoted `-m` hands git the literal opener
    assert!(with_command(
        ok,
        "git commit -m '$(cat <<'\\''EOF'\\''\n\nfeat: x\nEOF\n)'"
    ));
    assert!(with_command(
        ok,
        "git commit -m '$(cat <<\"EOF\"\nfeat: x\nEOF\n)'"
    ));
}

#[test]
fn fixup_squash_and_a_missing_message_are_let_through() {
    for ok in ["fixup! feat: x", "squash! whatever it was", "fixup! "] {
        assert_eq!(verdict(ok, &[]), HookOutput::allow(), "{ok}");
    }
    assert!(denied("fixup!x", &[]));
    // `git commit -F msg.txt`, `-C`, `--fixup`: the host reads no message
    assert_eq!(validate_commit(&commit(None), &[]), HookOutput::allow());
}

pub(super) fn spec_write(class: &str, in_spec_root: bool) -> Facts<'static> {
    Facts {
        class: class.into(),
        paths: vec![PathFact {
            path: "specs/roadmap.spec".into(),
            in_spec_root,
            ..PathFact::default()
        }],
        ..Facts::default()
    }
}

#[test]
fn a_spec_write_reminds_that_progress_is_computed() {
    for class in ["write", "edit"] {
        let out = phase_boundary(&spec_write(class, true));
        let ctx = out.context.unwrap();
        assert!(ctx.contains("specs/roadmap.spec"), "{ctx}");
        assert!(ctx.contains("`specforge gsd progress-report`"), "{ctx}");
        assert_eq!(out.decision, Some(Decision::Allow));
    }
    // a file outside the spec root, and a tool that does not write
    assert_eq!(
        phase_boundary(&spec_write("write", false)),
        HookOutput::default()
    );
    assert_eq!(
        phase_boundary(&spec_write("read", true)),
        HookOutput::default()
    );
    assert_eq!(phase_boundary(&Facts::default()), HookOutput::default());
}

fn node(id: &str, kind: &str, fields: &[(&str, serde_json::Value)]) -> GraphNode {
    GraphNode {
        id: id.into(),
        kind: kind.into(),
        fields: fields
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
        ..GraphNode::default()
    }
}

fn project_graph(extra: Vec<GraphNode>) -> CommandGraph {
    let mut nodes = vec![
        node("taskboard", "project", &[]),
        node(
            "v1.0",
            "milestone",
            &[
                ("name", json!("Taskboard")),
                (
                    "progress",
                    json!({"phases": 4, "phases_complete": 1, "plans": 3, "plans_complete": 1, "percent": 25}),
                ),
            ],
        ),
        node(
            "v1.0/01",
            "phase",
            &[
                ("slug", json!("scaffold")),
                ("status", json!("complete")),
                ("plans_complete", json!("1/1")),
            ],
        ),
        node(
            "v1.0/02",
            "phase",
            &[
                ("slug", json!("persistence")),
                ("status", json!("planned")),
                ("plans_complete", json!("0/1")),
            ],
        ),
    ];
    nodes.extend(extra);
    CommandGraph::new(nodes, Vec::new())
}

fn started(graph: &CommandGraph, workstream: Option<&str>) -> String {
    let mut f = Facts {
        graph: Some(graph),
        ..Facts::default()
    };
    if let Some(w) = workstream {
        f.project_state.insert("active_workstream".into(), json!(w));
    }
    let out = session_state(&f, "yolo");
    assert_eq!(out.decision, Some(Decision::Allow));
    out.context.unwrap()
}

#[test]
fn a_session_starts_with_the_progress_line() {
    let g = project_graph(Vec::new());
    let ctx = started(&g, Some("ui"));
    assert!(
        ctx.starts_with("## Project State Reminder\n\nv1.0 Taskboard: 1/4 phases complete (25%)"),
        "{ctx}"
    );
    assert!(ctx.contains("phase 02 persistence"), "{ctx}");
    assert!(ctx.contains("Active workstream: ui"), "{ctx}");
    assert!(ctx.ends_with("Config: \"mode\": \"yolo\""), "{ctx}");
    assert!(
        !ctx.contains("blockers") && !ctx.contains("paused"),
        "{ctx}"
    );
}

#[test]
fn open_blockers_and_a_pause_are_named() {
    let g = project_graph(vec![
        node("b1", "blocker", &[("text", json!("the API key"))]),
        node(
            "b2",
            "blocker",
            &[("text", json!("old")), ("resolved", json!("2026-10-01"))],
        ),
        node("pause", "pause", &[]),
    ]);
    let ctx = started(&g, None);
    assert!(ctx.contains("Open blockers: the API key\n"), "{ctx}");
    assert!(!ctx.contains("old"), "{ctx}");
    assert!(ctx.contains("Work is paused"), "{ctx}");
    assert!(!ctx.contains("workstream"), "{ctx}");
}

#[test]
fn without_a_project_a_session_is_pointed_at_new_project() {
    for graph in [None, Some(CommandGraph::default())] {
        let f = Facts {
            graph: graph.as_ref(),
            ..Facts::default()
        };
        let ctx = session_state(&f, "interactive").context.unwrap();
        assert!(
            ctx.contains("No project found - suggest /gsd-new-project if starting new work."),
            "{ctx}"
        );
        assert!(ctx.contains("\"mode\": \"interactive\""), "{ctx}");
    }
}
