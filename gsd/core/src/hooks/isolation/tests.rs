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

/// Upstream's agent isolation guard has no hook here (r44 post-fix N7): every case of its suite is one divergence.
#[test]
fn agent_isolation_guard_accounts_for_every_upstream_case() {
    accounting::account(
        AGENT_SUITE,
        &[],
        &[(
            "",
            "diverges: [no-isolation-guard] the guard would always allow, so none is installed (the roster drops the file)",
        )],
    );
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
    "bug-3542: gsd-executor.md prohibits `git stash` family inside worktrees" => "fs: it reads upstream's executor agent text; the port's agent texts are pinned by the agent goldens",
    "bug #260: gsd-worktree-path-guard.js > #2547: malformed Kimi edit list" => "runtime: Kimi's tool vocabulary has no renderer here",
    "bug #260: gsd-worktree-path-guard.js > #2547: a spurious file_path does not shadow Kimi's authoritative path > ${label} still blocks" => "runtime: Kimi's tool vocabulary has no renderer here",
    "bug #260: gsd-worktree-path-guard.js > #2547: a spurious file_path does not shadow Kimi's authoritative path > a spurious file_path on an in-worktree write" => "runtime: Kimi's tool vocabulary has no renderer here",
    "bug #260: gsd-worktree-path-guard.js > #2595: a non-string file_path with no path key fails open explicitly > Kimi payload" => "runtime: Kimi's tool vocabulary has no renderer here",
    "bug #260: gsd-worktree-path-guard.js > sibling path is blocked" => "git: the target is a second repository next to the worktree; whether a path is in a repository is git's to say, and the hook refuses only the main checkout",
    "bug #260: gsd-worktree-path-guard.js > dot-dot traversal is blocked" => "git: the target is a second repository; a_traversal_into_the_main_checkout_is_refused runs the traversal into the main checkout",
    "#2304 \u{2014} Kimi tool vocabulary engages the guard" => "runtime: Kimi's tool vocabulary has no renderer here",
    "install.js guard for gsd-worktree-path-guard.js" => "installer: upstream's installer registers the guard; install_hosts and the hooks tests pin what SpecForge's install writes",
    "bug #261: workflow guard blocks forced git add" => "git: the forced `git add -f` block needs shell and git (D-r44-11); T4 names it",
    "git-cmd.js isGitSubcommand: should match commit" => "runtime: git-cmd.js is validate-commit's shell parser (a loop of ~10 commit spellings, each a title `desc`); the kernel's commit fact replaces it (D-r44-14)",
    "git-cmd.js isGitSubcommand: should NOT match commit" => "runtime: git-cmd.js is validate-commit's shell parser (a loop of non-commit spellings, each a title `desc`); the kernel's commit fact replaces it (D-r44-14)",
    "git-cmd.js tokenize" => "runtime: git-cmd.js is validate-commit's shell parser; the kernel's commit fact replaces it (D-r44-14)",
    "gsd-validate-commit.sh delegates to git-cmd.js" => "runtime: git-cmd.js is validate-commit's shell parser; the kernel's commit fact replaces it (D-r44-14)",
    "git-cmd.js resolveCommitSubject > MAJOR 3: a TRUNCATED capture" => "diverges: [truncated-heredoc] upstream's `-m` capture truncates at a quote and its resolver reads a complete first line of a cut body; the kernel hands the whole message, and a heredoc with no terminator is judged by its opener (stricter)",
    "git-cmd.js resolveCommitSubject > property:" => "env: fast-check properties over generated heredocs; the cases above are the examples they generalise",
    "git-cmd.js extractBranchArgument" => "runtime: git-cmd.js is validate-commit's shell parser; the kernel's commit fact replaces it (D-r44-14)",
];

#[test]
fn worktree_path_guard_accounts_for_every_upstream_case() {
    let mut skipped: Vec<(&str, &str)> = SAFETY_SKIPPED.to_vec();
    skipped.extend(
        WORKTREE_LIBRARY_BLOCKS
            .iter()
            .map(|b| (*b, WORKTREE_LIBRARY)),
    );
    let mut ported = safety_ported();
    ported.extend(crate::hooks::community::tests::resolve_subject_ported());
    accounting::account(SAFETY_SUITE, &ported, &skipped);
}

/// REVIEW-r44 M4: the main checkout under another spelling or by a relative path is still the main checkout.
#[test]
fn the_main_checkout_is_refused_by_case_and_by_a_relative_path() {
    let outside = |path: &str| Facts {
        host: "claude-code".into(),
        cwd: WT.into(),
        class: "write".into(),
        input: json!({ "file_path": path }),
        paths: vec![PathFact {
            path: path.into(),
            in_project: false,
            ..PathFact::default()
        }],
        ..Facts::default()
    };
    assert!(refused(&outside("/t/repo/src/a.ts")));
    assert!(refused(&outside("/t/REPO/src/a.ts")), "case");
    assert!(refused(&outside("/T/Repo/SRC/a.ts")), "case");
    assert!(
        refused(&outside("../../../src/a.ts")),
        "relative, out of the worktree"
    );
    assert!(refused(&outside("../../../../repo/src/a.ts")));
    assert!(!refused(&outside("../../../../elsewhere/a.ts")));
    assert!(
        !refused(&outside("/t/repository/a.ts")),
        "a sibling that shares a prefix"
    );
}
