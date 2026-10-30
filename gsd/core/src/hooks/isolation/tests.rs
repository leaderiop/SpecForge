//! Ported from gsd-core v1.18.0 tests/gsd-agent-isolation-guard.test.cjs and tests/worktree-safety.test.cjs (the `bug #260: gsd-worktree-path-guard.js` and `#1342` blocks), MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! Upstream's cases run the hook in a temp project with a sentinel file, the runtime registry and real git worktrees.
//! None of those is a fact here, so a case is ported when what it decides is a fact (the tool, the `isolation`
//! parameter, where the project is and where the path is) and skipped, with its reason, when the decision was the
//! file's or git's. Each ported case is written out below with its upstream title.

use serde_json::{Value, json};

use super::*;
use crate::hooks::accounting;
use crate::hooks::facts::PathFact;

const AGENT_SUITE: &str =
    include_str!("../../../../parity/upstream/v1.18.0/tests/gsd-agent-isolation-guard.test.cjs");
const SAFETY_SUITE: &str =
    include_str!("../../../../parity/upstream/v1.18.0/tests/worktree-safety.test.cjs");

fn dispatch(host: &str, tool: &str, input: Value) -> Facts<'static> {
    Facts {
        host: host.into(),
        class: if matches!(tool, "Agent" | "Task") {
            "agent"
        } else {
            "shell"
        }
        .into(),
        tool: tool.into(),
        input,
        ..Facts::default()
    }
}

fn denied(f: &Facts) -> bool {
    agent_isolation_guard(f).decision == Some(Decision::Deny)
}

fn executor(isolation: Option<&str>) -> Value {
    match isolation {
        Some(i) => json!({"subagent_type": "gsd-executor", "isolation": i}),
        None => json!({"subagent_type": "gsd-executor"}),
    }
}

/// The titles of the agent suite this module runs (each asserted where it is written).
fn agent_ported() -> Vec<String> {
    let mut ported = Vec::new();
    let mut port = |title: &str| ported.push(title.to_string());
    let claude = |tool: &str, input: Value| dispatch("claude-code", tool, input);

    assert!(denied(&claude("Agent", executor(None))));
    port("row 1: absent isolation param, harness-worktree, GSD project -> DENY");
    assert!(!denied(&claude("Agent", executor(Some("worktree")))));
    port("row 2: isolation=\"worktree\" present -> allow");
    assert!(denied(&claude("Agent", executor(Some("")))));
    port("row 3: isolation=\"\" (empty) -> DENY");
    assert!(denied(&claude("Agent", executor(Some("none")))));
    port("row 4: isolation=\"none\" -> DENY");
    assert!(!denied(&claude(
        "Agent",
        json!({"subagent_type": "gsd-code-reviewer"})
    )));
    port("row 5: subagent_type=gsd-code-reviewer (not an executor) -> allow");
    assert!(!denied(&claude("Bash", json!({"command": "echo hi"}))));
    port("row 10: wrong tool (Bash) -> allow");
    assert!(!denied(&claude("Agent", json!({}))));
    port("row 11a: subagent_type absent -> allow, must not throw");
    assert!(!denied(&claude(
        "Agent",
        json!({"subagent_type": ["gsd-executor"]})
    )));
    port("row 11b: subagent_type malformed (non-string, e.g. array) -> allow, must not throw");
    assert!(!denied(&claude("Agent", Value::Null)));
    port("row 11c: tool_input entirely absent -> allow, must not throw");

    let reason = agent_isolation_guard(&claude("Agent", executor(None)))
        .reason
        .unwrap_or_default();
    assert!(
        reason.contains("Add isolation=\"worktree\" to the Agent() call"),
        "{reason}"
    );
    port("reason names the exact parameter to add (self-correction requirement)");

    // The property: for any string, blocked unless it is exactly "worktree".
    let mut values: Vec<String> = [
        "",
        "none",
        "worktree",
        "Worktree",
        "WORKTREE",
        " worktree",
        "worktree ",
        "worktree\n",
        "worktrees",
        "work",
        "tree",
        "harness-worktree",
        "orchestrator-worktree",
        "0",
        "null",
        "true",
        "\u{0}",
        "worktree\u{0}",
        "\u{200b}worktree",
        "wörktree",
    ]
    .map(String::from)
    .to_vec();
    values.extend((0..="worktree".len()).map(|n| "worktree"[..n].to_string()));
    for v in &values {
        assert_eq!(
            denied(&claude("Agent", executor(Some(v)))),
            v != "worktree",
            "{v:?}"
        );
        assert_eq!(
            denied(&claude("Task", executor(Some(v)))),
            v != "worktree",
            "Task {v:?}"
        );
    }
    port("for any string value, dispatch is blocked unless the value is exactly \"worktree\"");

    assert!(denied(&claude("Task", executor(None))));
    port("tool_name=\"Task\" behaves identically to \"Agent\" (#3045 MAJOR 1)");
    assert!(!denied(&claude("Task", executor(Some("worktree")))));
    port("tool_name=\"Task\" with isolation=\"worktree\" present -> allow, same as \"Agent\"");
    ported
}

/// Every host but Claude Code is let through: its dispatch tool has no `isolation` parameter (D-r44-5).
#[test]
fn the_isolation_guard_is_claude_codes() {
    for host in ["codex", "cursor"] {
        assert!(!denied(&dispatch(host, "Task", executor(None))), "{host}");
    }
}

const AGENT_SKIPPED: &[(&str, &str)] = &[
    (
        "gsd-agent-isolation-guard.js: applicability matrix (#3045) > row 6",
        "env: the resolved mode comes from the runtime registry and GSD_RUNTIME; here the host decides",
    ),
    (
        "gsd-agent-isolation-guard.js: applicability matrix (#3045) > row 7",
        "env: the resolved mode comes from the runtime registry and GSD_RUNTIME; here the host decides",
    ),
    (
        "gsd-agent-isolation-guard.js: applicability matrix (#3045) > row 8",
        "fs: .planning/config.json is unreadable; no hook reads it",
    ),
    (
        "gsd-agent-isolation-guard.js: applicability matrix (#3045) > row 9",
        "fs: whether .planning/config.json exists; the hook's `when` is the setting",
    ),
    (
        "gsd-agent-isolation-guard.js: applicability matrix (#3045) > row 12",
        "fs: .planning/config.json is corrupt; no hook reads it",
    ),
    (
        "gsd-agent-isolation-guard.js: applicability matrix (#3045) > row 11d",
        "host: the kernel reads the event; a payload that is not a tool event never reaches a hook",
    ),
    (
        "gsd-agent-isolation-guard.js: applicability matrix (#3045) > row 11e",
        "host: the kernel reads the event; a payload that is not a tool event never reaches a hook",
    ),
    (
        "#2486 regression: inspect-dispatch-isolation is the sentinel-free read",
        "fs: gsd-tools' dispatch-isolation verbs and their sentinel; the typed commands do not record one",
    ),
    (
        "#3045 CORE REDESIGN — dispatch-isolation records as an unconditional side effect",
        "fs: the dispatch-isolation sentinel file (gsd-tools record-dispatch-isolation), which no hook reads; the hook's `when` is workflow.use_worktrees (D-r44-10)",
    ),
    (
        "#3045 MAJOR — --harness-flag can now accept a bare CLI-flag value (Cursor real registry value + generalized parsing)",
        "fs: the dispatch-isolation sentinel file (gsd-tools record-dispatch-isolation), which no hook reads; the hook's `when` is workflow.use_worktrees (D-r44-10)",
    ),
    (
        "#3045 MINOR — writer/reader sentinel path derivation now agrees for a linked worktree without its own .planning/",
        "fs: the dispatch-isolation sentinel file (gsd-tools record-dispatch-isolation), which no hook reads; the hook's `when` is workflow.use_worktrees (D-r44-10)",
    ),
    (
        "gsd-agent-isolation-guard.js: #3045 BLOCKER 2 — default-install fail-open (isolated two-review finding)",
        "env: the runtime identity read from ~/.gsd/defaults.json and the sentinel; the guard is Claude Code's",
    ),
    (
        "gsd-agent-isolation-guard.js: #3045 BLOCKER regression — sentinel is authoritative over registry capability > sentinel says isolation=none",
        "fs: the dispatch-isolation sentinel file (gsd-tools record-dispatch-isolation), which no hook reads; the hook's `when` is workflow.use_worktrees (D-r44-10)",
    ),
    (
        "gsd-agent-isolation-guard.js: #3045 BLOCKER regression — sentinel is authoritative over registry capability > sentinel says isolation=orchestrator-worktree",
        "fs: the dispatch-isolation sentinel file (gsd-tools record-dispatch-isolation), which no hook reads; the hook's `when` is workflow.use_worktrees (D-r44-10)",
    ),
    (
        "gsd-agent-isolation-guard.js: #3045 BLOCKER regression — sentinel is authoritative over registry capability > sentinel says isolation=harness-worktree + dispatch missing",
        "fs: the dispatch-isolation sentinel file (gsd-tools record-dispatch-isolation), which no hook reads; the hook's `when` is workflow.use_worktrees (D-r44-10)",
    ),
    (
        "gsd-agent-isolation-guard.js: #3045 BLOCKER regression — sentinel is authoritative over registry capability > sentinel says isolation=harness-worktree + dispatch carries",
        "fs: the dispatch-isolation sentinel file (gsd-tools record-dispatch-isolation), which no hook reads; the hook's `when` is workflow.use_worktrees (D-r44-10)",
    ),
    (
        "gsd-agent-isolation-guard.js: #3045 BLOCKER regression — sentinel is authoritative over registry capability > STALE sentinel",
        "fs: the dispatch-isolation sentinel file (gsd-tools record-dispatch-isolation), which no hook reads; the hook's `when` is workflow.use_worktrees (D-r44-10)",
    ),
    (
        "gsd-agent-isolation-guard.js: #3045 BLOCKER regression — sentinel is authoritative over registry capability > MALFORMED sentinel",
        "fs: the dispatch-isolation sentinel file (gsd-tools record-dispatch-isolation), which no hook reads; the hook's `when` is workflow.use_worktrees (D-r44-10)",
    ),
    (
        "gsd-agent-isolation-guard.js: #3045 BLOCKER regression — sentinel is authoritative over registry capability > no sentinel + workflow.use_worktrees=false",
        "fs: the dispatch-isolation sentinel file (gsd-tools record-dispatch-isolation), which no hook reads; the hook's `when` is workflow.use_worktrees (D-r44-10)",
    ),
    (
        "gsd-agent-isolation-guard.js: #3045 BLOCKER regression — sentinel is authoritative over registry capability > no sentinel + workflow.use_worktrees absent",
        "fs: the dispatch-isolation sentinel file (gsd-tools record-dispatch-isolation), which no hook reads; the hook's `when` is workflow.use_worktrees (D-r44-10)",
    ),
    (
        "gsd-agent-isolation-guard.js: #3045 MAJOR — clock seam boundary coverage (in-process, no subprocess wall-clock race)",
        "fs: the sentinel's staleness clock",
    ),
    (
        "gsd-agent-isolation-guard.js: #3045 MAJOR 2 — undeterminable runtime does not demand the Claude kwarg",
        "env: the runtime identity (GSD_RUNTIME, config.json); here the host is the fact",
    ),
    (
        "gsd-agent-isolation-guard.js: #3045 SECURITY F2 — sentinel bound to phase/plan, mismatch is \"no applicable sentinel\"",
        "fs: the dispatch-isolation sentinel file (gsd-tools record-dispatch-isolation), which no hook reads; the hook's `when` is workflow.use_worktrees (D-r44-10)",
    ),
    (
        "gsd-agent-isolation-guard.js: #3566 — per-install .gsd-runtime marker rung (in-process)",
        "env: the runtime identity (the .gsd-runtime marker); here the host is the fact",
    ),
    (
        "gsd-agent-isolation-guard.js: #3582 cold tree — RuntimeBuildError surfaces distinctly",
        "env: upstream's compiled runtime library self-build; there is none here",
    ),
    (
        "gsd-agent-isolation-guard.js: #4594 rows 15/28-32 — prompt-first extraction + sentinel-discard reporting",
        "fs: the dispatch-isolation sentinel file (gsd-tools record-dispatch-isolation), which no hook reads; the hook's `when` is workflow.use_worktrees (D-r44-10)",
    ),
    (
        "gsd-agent-isolation-guard.js: #4734 — a non-git project root is never demanded worktree isolation",
        "git: whether the project root is a repository; the hook's `when` is the setting (K19 git facts)",
    ),
    (
        "guard fallback — worktreesOptedOut ladder (#3972)",
        "fs: the config ladder (workstream and root use_worktrees); the hook's `when` is the resolved setting",
    ),
    (
        "hooks/lib/isolation-deny-reason.js — sanitizeForReason (#4594 F2/F5/F6)",
        "decision: the reason here interpolates only the subagent type, which is one of a fixed set; nothing from a sentinel to sanitise",
    ),
    (
        "worktreesOptedOut — ladder unit semantics (#3972)",
        "fs: the config ladder; the hook's `when` is the resolved setting",
    ),
];

#[test]
fn agent_isolation_guard_matches_upstreams_cases() {
    assert_eq!(agent_ported().len(), 13);
}

#[test]
fn agent_isolation_guard_accounts_for_every_upstream_case() {
    accounting::account(AGENT_SUITE, &agent_ported(), AGENT_SKIPPED);
}

// ---------------------------------------------------------------- the path guard

const MAIN: &str = "/t/repo";
const WT: &str = "/t/repo/.claude/worktrees/agent-test";
const FEATURE: &str = "/t/repo/.claude/worktrees/feature-x";

/// A write or edit of `file` (the tool's `file_path`) by a host running in `cwd`, with the project at `root`: a path
/// under the root is in the project (named relatively, as the kernel names it), any other as the tool named it.
fn touch(cwd: &str, root: &str, tool: &str, file: Value) -> Facts<'static> {
    let class = match tool {
        "Write" => "write",
        "Edit" | "MultiEdit" => "edit",
        "Read" => "read",
        "Grep" => "search",
        _ => "shell",
    };
    let paths = match file.as_str() {
        Some(p) => {
            // The kernel resolves `..` before it judges where a path is.
            let abs = if p.starts_with('/') {
                resolved(p)
            } else {
                p.to_string()
            };
            let inside = abs
                .strip_prefix(root)
                .filter(|r| r.is_empty() || r.starts_with('/'));
            vec![match inside {
                Some(rel) => PathFact {
                    path: rel.trim_start_matches('/').into(),
                    in_project: true,
                    ..PathFact::default()
                },
                None if !p.starts_with('/') => PathFact {
                    path: p.into(),
                    in_project: true,
                    ..PathFact::default()
                },
                None => PathFact {
                    path: p.into(),
                    ..PathFact::default()
                },
            }]
        }
        None => Vec::new(),
    };
    Facts {
        host: "claude-code".into(),
        cwd: cwd.into(),
        class: class.into(),
        tool: tool.into(),
        input: json!({"file_path": file}),
        paths,
        ..Facts::default()
    }
}

fn refused(f: &Facts) -> bool {
    worktree_path_guard(f).decision == Some(Decision::Deny)
}

/// The cases of `bug #260: gsd-worktree-path-guard.js` and `#1342` this module runs.
fn safety_ported() -> Vec<String> {
    let mut ported = Vec::new();
    let mut port = |title: &str| ported.push(title.to_string());
    let into_main = || json!(format!("{MAIN}/src/index.ts"));

    assert!(!refused(&touch(
        MAIN,
        MAIN,
        "Edit",
        json!(format!("{MAIN}/src/foo.ts"))
    )));
    port("Edit call in main repo (.git is a directory) exits 0");
    assert!(!refused(&touch(
        MAIN,
        MAIN,
        "Write",
        json!(format!("{MAIN}/out.txt"))
    )));
    port("Write call in main repo exits 0");
    assert!(!refused(&touch(WT, WT, "Edit", json!("src/foo.ts"))));
    port("Edit with relative file_path exits 0 even in worktree");
    assert!(!refused(&touch(WT, WT, "Write", json!("out.txt"))));
    port("Write with relative file_path exits 0 in worktree");
    for (tool, title) in [
        ("Bash", "Bash tool exits 0"),
        ("Read", "Read tool exits 0"),
        ("Grep", "Grep tool exits 0"),
    ] {
        assert!(!refused(&touch(WT, WT, tool, into_main())), "{tool}");
        port(title);
    }
    assert!(!refused(&touch(
        WT,
        WT,
        "Edit",
        json!(format!("{WT}/src/foo.ts"))
    )));
    port("Edit with absolute path inside worktree root exits 0");
    assert!(!refused(&touch(WT, WT, "Edit", json!(WT))));
    port("Edit targeting exactly the worktree root exits 0");
    assert!(refused(&touch(WT, WT, "Edit", into_main())));
    port("Edit targeting main repo root exits 2 with block decision");
    assert!(refused(&touch(
        WT,
        WT,
        "Write",
        json!(format!("{MAIN}/out.txt"))
    )));
    port("Write targeting main repo root exits 2 with block decision");
    let leak = format!("{MAIN}/src/leak.ts");
    let reason = worktree_path_guard(&touch(WT, WT, "Edit", json!(leak)))
        .reason
        .unwrap_or_default();
    assert!(reason.contains(&leak), "{reason}");
    port("block output includes the offending path in reason");
    assert!(refused(&touch(WT, WT, "Edit", into_main())));
    port("native Claude payload (no path field) still blocks on file_path alone");
    for bad in [json!([]), json!({}), json!(42), json!(true)] {
        assert!(!refused(&touch(WT, WT, "Edit", bad)));
    }
    port("native Claude Edit with a ${label} file_path exits 0 without crashing");
    assert!(refused(&touch(WT, WT, "Edit", into_main())));
    port("control: a string cross-root file_path with no path key still blocks");
    let sub = format!("{WT}/src");
    assert!(refused(&touch(&sub, WT, "Edit", into_main())));
    port("hook fires when cwd is a subdirectory of the worktree, not just its root");
    assert!(!refused(&touch(
        &sub,
        WT,
        "Edit",
        json!(format!("{WT}/src/foo.ts"))
    )));
    port("path inside worktree passes even when cwd is a subdirectory");
    assert!(refused(&touch(WT, WT, "MultiEdit", into_main())));
    port("MultiEdit with outside absolute path is blocked");
    assert!(!refused(&touch(
        WT,
        WT,
        "MultiEdit",
        json!(format!("{WT}/src/foo.ts"))
    )));
    port("MultiEdit with inside absolute path passes");

    // #1342: the gate on a GSD agent's worktree, and the paths outside every repository.
    assert!(!refused(&touch(
        FEATURE,
        FEATURE,
        "Write",
        json!("/t/plans/my-plan.md")
    )));
    port("(1) non-agent linked worktree: Write to a path outside all git repos exits 0 (no block)");
    assert!(!refused(&touch(FEATURE, FEATURE, "Edit", into_main())));
    port(
        "(2) non-agent linked worktree: Edit targeting main repo root exits 0 (gate no-op, not #260 block)",
    );
    assert!(refused(&touch(WT, WT, "Edit", into_main())));
    port("(3) GSD-managed worktree: Edit targeting main repo root exits 2 with block decision");
    assert!(!refused(&touch(
        WT,
        WT,
        "Edit",
        json!(format!("{WT}/src/foo.ts"))
    )));
    port("(4) GSD-managed worktree: absolute target inside the active worktree exits 0");
    assert!(!refused(&touch(WT, WT, "Write", json!("/t/ext/notes.md"))));
    port(
        "(5) GSD-managed worktree: target outside all git repos exits 0 (fail open, not #260 vector)",
    );
    let internals = worktree_path_guard(&touch(
        WT,
        WT,
        "Write",
        json!(format!("{MAIN}/.git/config")),
    ));
    assert_eq!(internals.decision, Some(Decision::Deny));
    assert!(internals.reason.unwrap_or_default().contains(".git"));
    port(
        "(6) blocks absolute writes into the main repo .git internals from a GSD worktree (#1342)",
    );
    ported
}

#[test]
fn worktree_path_guard_matches_upstreams_cases() {
    assert_eq!(safety_ported().len(), 25);
}

/// What the cases do not say: a `..` that climbs out of the worktree into the main checkout, a worktree that is no
/// agent's, and a path elsewhere in the same file system.
#[test]
fn a_traversal_into_the_main_checkout_is_refused() {
    assert!(refused(&touch(
        WT,
        WT,
        "Edit",
        json!(format!("{WT}/../../../src/index.ts"))
    )));
    assert!(!refused(&touch(
        WT,
        WT,
        "Edit",
        json!(format!("{WT}/../agent-test/src/foo.ts"))
    )));
    assert!(!refused(&touch(WT, WT, "Edit", json!("/tmp/scratch.txt"))));
    for dir in [
        "agent-",
        "worktree",
        "agent",
        "wf_",
        "xagent-1",
        "agent-a b",
    ] {
        let cwd = format!("{MAIN}/.claude/worktrees/{dir}");
        assert!(
            !refused(&touch(
                &cwd,
                &cwd,
                "Edit",
                json!(format!("{MAIN}/src/x.ts"))
            )),
            "{dir}"
        );
    }
    for dir in ["agent-1a2b3c4", "wf_phase-3", "agent-x.y_z"] {
        let cwd = format!("{MAIN}/.claude/worktrees/{dir}");
        assert!(
            refused(&touch(
                &cwd,
                &cwd,
                "Edit",
                json!(format!("{MAIN}/src/x.ts"))
            )),
            "{dir}"
        );
    }
}

macro_rules! block {
    ($($title:expr => $why:expr),* $(,)?) => { &[$(($title, $why)),*] };
}

const WORKTREE_LIBRARY: &str = "git: gsd-tools' worktree library (git worktree plans, create, prune, clean-up); no hook calls it";
const WORKTREE_LIBRARY_BLOCKS: [&str; 39] = [
    "resolveWorktreeContext > ",
    "resolveWorktreeLinkage > ",
    "shared isSpawnTimeout predicate — parity for worktree-base-ref evaluateWorktreeBaseDegrade (#3050) > ",
    "parseWorktreePorcelain > ",
    "planWorktreePrune > ",
    "executeWorktreePrunePlan > ",
    "listLinkedWorktreePaths > ",
    "inspectWorktreeHealth > ",
    "snapshotWorktreeInventory > ",
    "prune flow under degraded git > ",
    "planWorktreeWaveCleanup > ",
    "planWorktreeRecordAgent > ",
    "worktree branch namespace boundaries (#1995) > ",
    "cmdWorktreeRecordAgent > ",
    "worktree record-agent — real CLI dispatch (#1298) > ",
    "planWorktreeCreate > ",
    "executeWorktreeCreatePlan > ",
    "cmdWorktreeCreate > ",
    "cmdWorktreeCreate / cmdWorktreeRecordAgent — on-disk entry parity (#2584 FIX 2) > ",
    "executeWorktreeWaveCleanupPlan > ",
    "worktree-safety: resolveWorktreeRoot and pruneOrphanedWorktrees relocation identity > ",
    "worktree-safety: resolveWorktreeRoot behaviour > ",
    "worktree-safety: pruneOrphanedWorktrees behaviour > ",
    "bug-3707: executeWorktreeWaveCleanupPlan unlocks and retries on locked worktree > ",
    "bug-3707: reapOrphanWorktrees > ",
    "bug-3707: startup orphan sweep is wired into workflow entry points > ",
    "bug #3384: adjacent worktree data-loss guards > ",
    "Submodule worktree-isolation gate intersects planned paths (#2772) > ",
    "execute-phase.md dispatch wires USE_WORKTREES_FOR_PLAN (#2772) > ",
    "quick.md executor pre-commit submodule guard (#2772) > ",
    "#2596 scope conformance — planWaveScopeConformance (pure) > ",
    "#2596 SUMMARY-artifact predicate and its parity with the walker > ",
    "#2596 scope conformance — executeWorktreeWaveCleanupPlan integration > ",
    "#2596 files_modified on the cleanup manifest > ",
    "#2596 --files on the record-agent and create verbs > ",
    "#3003 — declared deletions: authorization is exact set membership > ",
    "planWorktreeRecordAgent — fast-check parity invariant (#1298) > ",
    "bug-3707: reapOrphanWorktrees — nowMs clock-injection BOUNDARY tests (#1191) > ",
    "bug-3707: reapOrphanWorktrees — adversarial edge cases > ",
];

const SAFETY_SKIPPED: &[(&str, &str)] = block![
    "bug-3542: stash pushed in main checkout is visible inside a linked worktree" => "git: a stash in a linked worktree; git's, and no hook",
    "bug-3542: gsd-executor.md prohibits `git stash` family inside worktrees" => "docs: it reads upstream's executor agent text; the port's agent texts are pinned by the agent goldens",
    "bug #260: gsd-worktree-path-guard.js > #2547: malformed Kimi edit list" => "runtime: Kimi's tool vocabulary has no renderer here",
    "bug #260: gsd-worktree-path-guard.js > #2547: a spurious file_path does not shadow Kimi's authoritative path > ${label} still blocks" => "runtime: Kimi's tool vocabulary has no renderer here",
    "bug #260: gsd-worktree-path-guard.js > #2547: a spurious file_path does not shadow Kimi's authoritative path > a spurious file_path on an in-worktree write" => "runtime: Kimi's tool vocabulary has no renderer here",
    "bug #260: gsd-worktree-path-guard.js > #2595: a non-string file_path with no path key fails open explicitly > Kimi payload" => "runtime: Kimi's tool vocabulary has no renderer here",
    "bug #260: gsd-worktree-path-guard.js > sibling path is blocked" => "git: the target is a second repository next to the worktree; whether a path is in a repository is git's to say, and the hook refuses only the main checkout",
    "bug #260: gsd-worktree-path-guard.js > dot-dot traversal is blocked" => "git: the target is a second repository; a_traversal_into_the_main_checkout_is_refused runs the traversal into the main checkout",
    "#2304 \u{2014} Kimi tool vocabulary engages the guard" => "runtime: Kimi's tool vocabulary has no renderer here",
    "install.js guard for gsd-worktree-path-guard.js" => "installer: upstream's installer registers the guard; install_hosts and the hooks tests pin what SpecForge's install writes",
    "bug #261: workflow guard blocks forced git add" => "git: the forced `git add -f` block needs shell and git (D-r44-11); T4 names it",
    "git-cmd.js tokenize" => "decision: git-cmd.js is validate-commit's shell parser; the kernel's commit fact replaces it (D-r44-14)",
    "gsd-validate-commit.sh delegates to git-cmd.js" => "decision: git-cmd.js is validate-commit's shell parser; the kernel's commit fact replaces it (D-r44-14)",
    "git-cmd.js resolveCommitSubject" => "decision: git-cmd.js is validate-commit's shell parser; the kernel's commit fact replaces it (D-r44-14)",
    "git-cmd.js extractBranchArgument" => "decision: git-cmd.js is validate-commit's shell parser; the kernel's commit fact replaces it (D-r44-14)",
];

#[test]
fn worktree_path_guard_accounts_for_every_upstream_case() {
    let mut skipped: Vec<(&str, &str)> = SAFETY_SKIPPED.to_vec();
    skipped.extend(
        WORKTREE_LIBRARY_BLOCKS
            .iter()
            .map(|b| (*b, WORKTREE_LIBRARY)),
    );
    accounting::account(SAFETY_SUITE, &safety_ported(), &skipped);
}
