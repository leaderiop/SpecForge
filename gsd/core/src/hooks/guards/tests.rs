//! Ported from gsd-core v1.18.0 tests/read-guard.test.cjs, tests/gsd-write-guard.test.cjs and tests/gsd-write-guard.property.test.cjs, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! Upstream's suites run the script against files on disk; here the file's size is a fact the host computed, so each
//! case is restated as the facts of the call. The write guard's lines become bytes at 52 a line (40 lines are the
//! 2,048-byte floor). `UPSTREAM_*` hold the titles ported; `SKIPPED_*` the rest, each with its reason; the accounting
//! test requires every case of each suite in exactly one.

use super::*;
use crate::hooks::{accounting, facts::PathFact};

const READ_SUITE: &str =
    include_str!("../../../../parity/upstream/v1.18.0/tests/read-guard.test.cjs");
const WRITE_SUITE: &str =
    include_str!("../../../../parity/upstream/v1.18.0/tests/gsd-write-guard.test.cjs");
const PROPERTY_SUITE: &str =
    include_str!("../../../../parity/upstream/v1.18.0/tests/gsd-write-guard.property.test.cjs");

/// A line of a spec file, in bytes: 40 lines are the 2,048-byte floor.
const LINE: u64 = 52;

fn on(host: &str, class: &str, paths: Vec<PathFact>) -> Facts<'static> {
    Facts {
        host: host.into(),
        class: class.into(),
        tool: "Write".into(),
        paths,
        ..Facts::default()
    }
}

fn file(path: &str, exists: bool) -> PathFact {
    PathFact {
        path: path.into(),
        exists,
        ..PathFact::default()
    }
}

fn spec(old: Option<u64>, new: Option<u64>) -> PathFact {
    PathFact {
        path: "spec/roadmap.spec".into(),
        in_project: true,
        in_spec_root: true,
        exists: old.is_some(),
        bytes: old,
        new_bytes: new,
    }
}

fn advises(answer: &HookOutput) -> bool {
    answer
        .context
        .as_deref()
        .is_some_and(|c| c.starts_with("READ-BEFORE-EDIT REMINDER"))
}

fn denies(answer: &HookOutput) -> bool {
    answer.decision == Some(Decision::Deny)
}

// -- read-guard.test.cjs ---------------------------------------------------------------------------------------

const READ_PORTED: &[&str] = &[
    "injects read-first guidance when Write targets an existing file",
    "injects read-first guidance when Edit targets an existing file",
    "does nothing for Write to a new file (file does not exist)",
    "does nothing for non-Write/Edit tools",
    "does nothing for Read tool",
    "exits cleanly when tool_input is missing",
    "guidance message includes the filename",
    "guidance message instructs to use Read tool before editing",
    "exits cleanly when tool_input.file_path is non-string",
    "skips advisory on Claude Code runtime (CLAUDE_SESSION_ID set)",
    "skips advisory when CLAUDECODE=1 is set (Claude Code CLI env)",
    "skips advisory when CLAUDE_SESSION_ID is set (back-compat)",
    "still injects advisory when neither CLAUDECODE nor CLAUDE_SESSION_ID is set",
    "skips advisory when stdin payload includes session_id (Claude Code hook-subprocess env)",
    "skips advisory when CLAUDE_CODE_ENTRYPOINT is set (env-var fallback, no session_id on stdin)",
    "still injects advisory when no Claude Code signal is present (non-Claude host)",
];

const READ_SKIPPED: &[(&str, &str)] = &[
    (
        "gsd-read-guard hook > exits cleanly on invalid JSON input",
        "runtime: the kernel reads the event; an unreadable one is on_failure's",
    ),
    (
        "gsd-read-guard hook > hook is registered in build-hooks.js HOOKS_TO_COPY",
        "installer: gsd-core's build script; T8 pins what specforge install writes",
    ),
    (
        "gsd-read-guard hook > hook is registered in install.js uninstall hook list",
        "installer: gsd-core's installer; T8 pins what specforge install writes",
    ),
    (
        "#2304: Kimi tool vocabulary is normalized by the read guard",
        "runtime: Kimi has no renderer here",
    ),
    (
        "#2547: read guard against the production Kimi payload shape",
        "runtime: Kimi has no renderer here",
    ),
];

/// The titles the read guard's cases run under; each runs here by its host, class and path facts.
fn read_ported() -> Vec<String> {
    let one = |host: &str, class: &str, p: PathFact| read_guard(&on(host, class, vec![p]));
    let a = one("codex", "write", file("src/app.ts", true));
    assert!(advises(&a), "Write on an existing file");
    assert!(
        a.context
            .as_deref()
            .unwrap_or_default()
            .contains("\"app.ts\""),
        "the file name"
    );
    assert!(
        a.context
            .as_deref()
            .unwrap_or_default()
            .contains("MUST Read it first")
    );
    assert!(
        advises(&one("cursor", "edit", file("/abs/dir/notes.md", true))),
        "Edit on an existing file"
    );
    assert!(
        !advises(&one("codex", "write", file("src/new.ts", false))),
        "a new file"
    );
    for class in ["read", "shell", "search", "agent"] {
        assert!(
            !advises(&one("codex", class, file("src/app.ts", true))),
            "{class}"
        );
    }
    assert!(
        !advises(&read_guard(&on("codex", "write", vec![]))),
        "no path named"
    );
    for host in ["claude-code"] {
        for class in ["write", "edit"] {
            assert_eq!(
                read_guard(&on(host, class, vec![file("src/app.ts", true)])),
                HookOutput::default(),
                "{host} {class}"
            );
        }
    }
    READ_PORTED.iter().map(|t| (*t).into()).collect()
}

#[test]
fn read_guard_matches_upstreams_cases() {
    assert_eq!(read_ported().len(), READ_PORTED.len());
}

#[test]
fn read_guard_accounts_for_every_upstream_case() {
    accounting::account(READ_SUITE, &read_ported(), READ_SKIPPED);
}

#[test]
fn read_guard_is_silent_on_claude_code() {
    let f = on("claude-code", "write", vec![file("src/app.ts", true)]);
    assert_eq!(read_guard(&f), HookOutput::default());
}

#[test]
fn read_guard_warns_on_codex_and_cursor() {
    for host in ["codex", "cursor"] {
        let answer = read_guard(&on(host, "write", vec![file("src/app.ts", true)]));
        assert_eq!(
            answer.decision,
            Some(Decision::Allow),
            "{host}: advice never blocks"
        );
        assert!(advises(&answer), "{host}");
    }
}

// -- gsd-write-guard.test.cjs and its property suite ------------------------------------------------------------

const WRITE_PORTED: &[&str] = &[
    "#973 shape: 292-line ROADMAP.md overwritten with 16 lines is BLOCKED (exit 2, decision block)",
    "sub-floor file (39 lines) is exempt from the ratio check",
    "at-floor file (40 lines) IS guarded — the floor is exclusive",
    "above-floor file (41 lines) IS guarded — floor boundary from above",
    "ratio boundary: exactly 40% of old passes; one line either side behaves",
    "creating a curated file that does not exist yet passes",
    "Edit tool call is out of scope — passes even on a curated target",
    "MultiEdit tool call is out of scope — passes",
    "payload without content (non-string) fails open",
    "wholesale rewrite of a NON-curated .md passes untouched (no override-fatigue)",
    "non-roadmap file under .planning/milestones/ is not curated — passes",
];

const PROPERTY_PORTED: &[&str] = &[
    "for any oldLines ≥ FLOOR_LINES: blocked ⟺ newLines < oldLines * SHRINK_RATIO",
    "for any oldLines < FLOOR_LINES: never blocked, however deep the shrink",
];

const WRITE_SKIPPED: &[(&str, &str)] = &[
    (
        "gsd-write-guard.js: catastrophic shrink of curated artifacts > block output names the documented override",
        "fs: the override env var and the sentinel are dropped (D-r44-7)",
    ),
    (
        "gsd-write-guard.js: catastrophic shrink of curated artifacts > GSD_ALLOW_PLANNING_SHRINK=1 bypasses",
        "env: GSD_ALLOW_PLANNING_SHRINK is dropped (D-r44-7)",
    ),
    (
        "gsd-write-guard.js: catastrophic shrink of curated artifacts > milestone roadmap",
        "fs: .planning/ paths are not involved; the guarded files are the spec files (D-r44-7)",
    ),
    (
        "gsd-write-guard.js: catastrophic shrink of curated artifacts > STATE.md under .planning/",
        "fs: .planning/ paths are not involved (D-r44-7)",
    ),
    (
        "gsd-write-guard.js: catastrophic shrink of curated artifacts > non-ENOENT read error",
        "fs: the host reads the old file's size; there is no read to fail",
    ),
    (
        "gsd-write-guard.js: catastrophic shrink of curated artifacts > differently-cased path",
        "fs: the host resolves the path (crates/app/ops authoring/hook/facts.rs)",
    ),
    (
        "gsd-write-guard.js: catastrophic shrink of curated artifacts > relative file_path resolves against the payload cwd",
        "fs: the host resolves the path against the event's cwd",
    ),
    (
        "round 9: symlink resolution, CRLF counting, and denial-content pins > a Write to a non-curated SYMLINK",
        "fs: the host resolves links before it computes the facts",
    ),
    (
        "round 9: symlink resolution, CRLF counting, and denial-content pins > the denial prose does NOT publish the sentinel recipe",
        "fs: no sentinel (D-r44-7)",
    ),
    (
        "round 9: symlink resolution, CRLF counting, and denial-content pins > CRLF content counts lines correctly",
        "diverges: bytes, not lines (D-r44-7, U52)",
    ),
    (
        "gsd-write-guard.js: deliberately narrow trigger (no-op paths) > malformed JSON on stdin fails open",
        "runtime: the kernel reads the event; an unreadable one is on_failure's",
    ),
    (
        "#2304: Kimi tool vocabulary engages the write guard > Kimi WriteFile catastrophic shrink",
        "runtime: Kimi has no renderer here",
    ),
    (
        "#2304: Kimi tool vocabulary engages the write guard > module-qualified",
        "runtime: Kimi has no renderer here",
    ),
    (
        "#2304: Kimi tool vocabulary engages the write guard > block reason reaches stderr",
        "runtime: Kimi has no renderer here",
    ),
    (
        "#2304: Kimi tool vocabulary engages the write guard > Kimi StrReplaceFile",
        "runtime: Kimi has no renderer here",
    ),
    (
        "#2304: Kimi tool vocabulary engages the write guard > a spurious model-supplied file_path",
        "runtime: Kimi has no renderer here",
    ),
    (
        "#2304: Kimi tool vocabulary engages the write guard > null and primitive payloads",
        "runtime: Kimi has no renderer here",
    ),
    (
        "#2304: Kimi tool vocabulary engages the write guard > Kimi WriteFile of a non-curated path",
        "runtime: Kimi has no renderer here",
    ),
    (
        "single-use sentinel exemption",
        "fs: the sentinel is dropped (D-r44-7)",
    ),
    (
        "guard <-> complete-milestone workflow binding",
        "fs: the sentinel is dropped, so no workflow arms it (D-r44-7)",
    ),
    (
        "workstream-scoped curated paths (#4455)",
        "fs: .planning/ workstream paths are not involved (D-r44-7)",
    ),
    (
        "project-only-scoped curated paths (#4455 follow-up)",
        "fs: .planning/ project paths are not involved (D-r44-7)",
    ),
    (
        "the shipped claim matches the shipped guarantee (round 10 Major 2)",
        "fs: upstream's README and docs claims about its sentinel",
    ),
    (
        "guard <-> gsd-roadmapper binding",
        "fs: the sentinel is dropped, so no agent arms it (D-r44-7)",
    ),
];

/// Whether a Write of `new` bytes over a spec file of `old` bytes is refused.
fn refused(old: u64, new: u64) -> bool {
    denies(&write_guard(&on(
        "claude-code",
        "write",
        vec![spec(Some(old), Some(new))],
    )))
}

fn write_ported() -> Vec<String> {
    assert!(refused(292 * LINE, 16 * LINE), "#973 shape");
    assert!(!refused(39 * LINE, 2 * LINE), "sub-floor");
    assert!(refused(40 * LINE, 15 * LINE), "at-floor");
    assert!(refused(41 * LINE, 15 * LINE), "above-floor");
    assert!(
        !refused(100 * LINE, 40 * LINE)
            && refused(100 * LINE, 39 * LINE)
            && !refused(100 * LINE, 41 * LINE),
        "ratio"
    );
    let w = |p: PathFact| write_guard(&on("claude-code", "write", vec![p]));
    assert_eq!(
        w(spec(None, Some(3 * LINE))),
        HookOutput::default(),
        "a new file"
    );
    let edit = on("claude-code", "edit", vec![spec(Some(292 * LINE), None)]);
    assert_eq!(write_guard(&edit), HookOutput::default(), "Edit");
    assert_eq!(
        w(spec(Some(292 * LINE), None)),
        HookOutput::default(),
        "no content"
    );
    let mut notes = spec(Some(200 * LINE), Some(5 * LINE));
    notes.path = "docs-notes.md".into();
    notes.in_spec_root = false;
    assert_eq!(
        w(notes.clone()),
        HookOutput::default(),
        "a file that is no spec file"
    );
    notes.in_spec_root = true;
    assert_eq!(
        w(notes),
        HookOutput::default(),
        "a non-.spec file in the spec root"
    );
    let mut audit = spec(Some(200 * LINE), Some(5 * LINE));
    audit.path = "milestones/v1-MILESTONE-AUDIT.md".into();
    assert_eq!(
        w(audit),
        HookOutput::default(),
        "a file that is no curated artifact"
    );
    let mut elsewhere = spec(Some(200 * LINE), Some(5 * LINE));
    elsewhere.in_spec_root = false;
    assert_eq!(
        w(elsewhere),
        HookOutput::default(),
        "a spec file outside the spec root"
    );
    WRITE_PORTED.iter().map(|t| (*t).into()).collect()
}

#[test]
fn write_guard_matches_upstreams_cases() {
    assert_eq!(write_ported().len(), WRITE_PORTED.len());
    // The property suite, exhaustively where upstream samples: blocked exactly under 40% from the floor up, never below.
    for old in 40..=400u64 {
        for new in 1..=400u64 {
            assert_eq!(
                refused(old * LINE, new * LINE),
                new * 100 < old * 40,
                "{old} -> {new}"
            );
        }
    }
    for old in 1..40u64 {
        for new in 1..=400u64 {
            assert!(!refused(old * LINE, new * LINE), "sub-floor {old} -> {new}");
        }
    }
}

#[test]
fn write_guard_accounts_for_every_upstream_case() {
    accounting::account(WRITE_SUITE, &write_ported(), WRITE_SKIPPED);
    let property: Vec<String> = PROPERTY_PORTED.iter().map(|t| (*t).into()).collect();
    accounting::account(PROPERTY_SUITE, &property, &[]);
}

#[test]
fn a_small_spec_file_may_shrink() {
    assert!(!refused(2_047, 1));
    assert!(refused(2_048, 819), "40% of 2,048 is 819.2");
    assert!(!refused(2_048, 820));
}

#[test]
fn a_deny_names_the_hook_the_file_and_both_sizes() {
    let answer = write_guard(&on(
        "claude-code",
        "write",
        vec![spec(Some(8_424), Some(0))],
    ));
    let reason = answer.reason.unwrap_or_default();
    for want in [
        "gsd.write_guard",
        "spec/roadmap.spec",
        "8424 bytes to 0",
        "0% of current",
    ] {
        assert!(reason.contains(want), "{want}: {reason}");
    }
}
