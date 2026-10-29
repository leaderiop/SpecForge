//! Ported from gsd-core v1.18.0 tests/smart-entry.unit.test.cjs, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! The cases of upstream's suite that call `classify`, `actionsFor`, `buildSummary` and `formatGsdSlash` on a signals
//! object, or that build a project on disk only to reach them (the STATE.md and ROADMAP.md the fixture writes become the
//! signals the case reads). `UPSTREAM_CASES` names each by its title, and `every_ported_case_is_in_the_vendored_suite`
//! finds each title in the vendored file.
//!
//! Left out, because they test what this port does not have, by name: `the STATE.md body starts after the frontmatter
//! block` (STATE.md's frontmatter), `idle-stranded (git-dependent)` (git), `verify-failed inspects current phase verify
//! artifact`, `verify-failed ignores failure in a higher phase`, `verify-failed includes decimal phase directories`
//! (the phase directory), `real STATE.md schema` (STATE.md), `CLI dispatch (gsd-tools smart-entry)` (the CLI is
//! black-box tested), `stale_activity honors the template's "date — description" shape`, `detectSignals — state_head
//! commit-age freshness` and `#3099: unusable last_activity emits a diagnostic` (a clock and git).
//! `an unavailable completed count labels only the known roadmap total` is ported with the count absent: the graph knows
//! a phase count of zero, so it is `Some(0)` here.

use super::*;

/// The titles of the ported cases, as the vendored suite writes them.
const UPSTREAM_CASES: &[&str] = &[
    "classifies \"${expected}\"",
    "#3864: status \"verified\" classifies as verify-pending",
    "#3864: status \"verification\" classifies as verify-pending",
    "#3864: the stem does not over-match — \"completed\" still reaches complete",
    "#3864: \"Phase complete — ready for verification\" at 5/5 stays complete",
    "#3864: \"Phase complete — ready for verification\" mid-project is verify-pending",
    "#3864: verify-failed still wins over the verify-pending stem (ordering pinned)",
    "SITUATIONS constant lists all 11 (incl unknown) and is frozen",
    "paused beats blocked (earlier row wins)",
    "blocked beats planning (earlier row wins)",
    "no-project beats everything (no .planning)",
    "${situation}: recommended action is /gsd-progress --next",
    "${situation}: keeps its direct recommendation (${command})",
    "${situation}: exactly one recommended, 1-4 unique-id /gsd-* actions",
    "every situation yields exactly one recommended action and /gsd-* commands",
    "#4890: ${situation} summary keeps global phase and roadmap counts distinct",
    "#4890: phase ${phase} preserves a plausible legacy ${total}-phase ratio",
    "#4890: a global phase beyond the milestone total uses labeled roadmap counts",
    "#4890: an unavailable completed count labels only the known roadmap total",
    "#4890: missing roadmap counts retain the exact legacy ratio",
    "#4890: a Progress table with no phase rows keeps the legacy fallback",
    "mid-milestone with stale total_phases + unchecked roadmap phases is NOT complete",
    "all roadmap phases complete classifies as complete even with stale cached total_phases",
    "per-phase \"shipped\" status alone does NOT satisfy the completion regex",
    "per-phase \"done\" status alone does NOT satisfy the completion regex",
    "legacy fallback: empty roadmap still classifies via STATE.md comparison",
    "legacy fallback: empty roadmap with current_phase < total_phases is NOT complete",
    "planning at the first of phases 9-17 reads \"Phase 1 of 9\"",
    "#4890: a Progress table preserves the known milestone ordinal",
    "executing and ready-to-verify lines use the same ordinal",
    "phase 9 complete with 1 of 9 done is not classified complete",
    "boundary: phase 16 complete is not complete, phase 17 complete is",
    "a plain milestone numbered from 1 is unchanged",
    "zero-based numbering keeps its ordinal position",
    "dotted decimal phase ids sort into their position",
    "a current phase missing from the roadmap retains the legacy ratio",
    "a roadmap that lists no phases keeps the raw STATE.md figures",
];

#[test]
fn every_ported_case_is_in_the_vendored_suite() {
    let suite = include_str!("../../../../parity/upstream/v1.18.0/tests/smart-entry.unit.test.cjs");
    for title in UPSTREAM_CASES {
        assert!(
            suite.contains(title),
            "the vendored suite has no case '{title}'"
        );
    }
}

/// STATE.md's frontmatter of the upstream fixtures as signals: `total_phases`, `current_phase`, `status`, `progress`,
/// a roadmap file (`roadmap`), and what a Progress table gives (`roadmap_*`).
fn sig(status: &str, current: Option<&str>, total: Option<i64>) -> Signals {
    Signals {
        current_phase: current.map(String::from),
        total_phases: total,
        status: status.into(),
        has_planning: true,
        has_roadmap: true,
        ..Signals::default()
    }
}

fn with_roadmap(mut s: Signals, total: Option<i64>, done: Option<i64>) -> Signals {
    s.roadmap_total_phases = total;
    s.roadmap_completed_phases = done;
    s
}

fn range(from: u32, to: u32) -> Vec<String> {
    (from..=to).map(|n| n.to_string()).collect()
}

fn sample() -> Signals {
    let mut s = sig("executing", Some("2"), Some(5));
    s.progress = Some(60);
    s
}

#[test]
fn smart_entry_matches_upstreams_vectors() {
    let mut paused = sig("planning", None, None);
    paused.paused = true;
    let mut blocked = sig("executing", Some("2"), Some(5));
    blocked.blockers = vec!["Need API key".into()];
    let mut failed = sig("verify-failed", Some("2"), Some(5));
    failed.verify_failed = true;
    let mut none = sig("", None, None);
    none.has_planning = false;
    let mut first = sig("planning", Some("0"), Some(0));
    first.has_roadmap = false;
    let cases = [
        ("no-project", none),
        ("paused", paused),
        ("blocked", blocked),
        ("verify-failed", failed),
        ("needs-first-phase", first),
        ("planning", sig("planning", Some("2"), Some(5))),
        ("executing", sample()),
        ("verify-pending", sig("needs-review", Some("2"), Some(5))),
        ("complete", sig("complete", Some("5"), Some(5))),
        ("unknown", sig("", Some("2"), Some(5))),
    ];
    for (expected, s) in cases {
        assert_eq!(classify(&s), expected);
    }
    assert_eq!(SITUATIONS.len(), 11);
    assert!(SITUATIONS.contains(&"unknown"));
}

#[test]
fn the_verify_stem_and_the_completion_words() {
    let at = |status: &str, current: &str| classify(&sig(status, Some(current), Some(5)));
    assert_eq!(at("verified", "2"), "verify-pending");
    assert_eq!(at("verification", "2"), "verify-pending");
    assert_eq!(at("completed", "5"), "complete");
    assert_eq!(
        at("Phase complete — ready for verification", "5"),
        "complete"
    );
    assert_eq!(
        at("Phase complete — ready for verification", "2"),
        "verify-pending"
    );
    let mut failed = sig("verify-failed", Some("2"), Some(5));
    failed.verify_failed = true;
    assert_eq!(classify(&failed), "verify-failed");
}

#[test]
fn the_priority_order_holds() {
    let mut both = sig("planning", Some("2"), Some(5));
    both.paused = true;
    both.blockers = vec!["b".into()];
    assert_eq!(classify(&both), "paused");
    both.paused = false;
    assert_eq!(classify(&both), "blocked");
    both.has_planning = false;
    assert_eq!(classify(&both), "no-project");
}

#[test]
fn forward_motion_delegates_to_progress_next_and_the_rest_keep_their_command() {
    let s = sample();
    for situation in ["planning", "executing", "verify-pending"] {
        let actions = actions_for(situation, &s, "claude");
        let recommended: Vec<&Action> = actions.iter().filter(|a| a.recommended).collect();
        assert_eq!(recommended.len(), 1, "{situation}");
        assert_eq!(recommended[0].command, "/gsd-progress --next");
    }
    for (situation, command) in [
        ("no-project", "/gsd-new-project"),
        ("paused", "/gsd-resume-work"),
        ("blocked", "/gsd-debug"),
        ("verify-failed", "/gsd-verify-work"),
        ("idle-stranded", "/gsd-ship"),
        ("complete", "/gsd-new-milestone"),
    ] {
        let actions = actions_for(situation, &s, "claude");
        assert_eq!(
            actions
                .iter()
                .find(|a| a.recommended)
                .map(|a| a.command.as_str()),
            Some(command)
        );
    }
}

#[test]
fn every_situation_has_one_recommended_action_and_unique_slash_commands() {
    let s = sample();
    for situation in SITUATIONS {
        let actions = actions_for(situation, &s, "claude");
        assert!(
            (1..=4).contains(&actions.len()),
            "{situation}: {} actions",
            actions.len()
        );
        assert_eq!(
            actions.iter().filter(|a| a.recommended).count(),
            1,
            "{situation}"
        );
        let mut ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), actions.len(), "{situation}: ids are unique");
        for a in &actions {
            assert!(a.command.starts_with("/gsd-"), "{situation}/{}", a.id);
        }
        assert!(!build_summary(situation, &s).is_empty(), "{situation}");
    }
}

#[test]
fn a_summary_keeps_global_phase_and_roadmap_counts_distinct() {
    for (status, tail) in [
        ("planning", "needs a plan"),
        ("executing", "executing"),
        ("verifying", "ready to verify"),
    ] {
        let mut s = with_roadmap(sig(status, Some("13"), Some(5)), Some(5), Some(2));
        s.progress = Some(40);
        let situation = classify(&s);
        let summary = build_summary(situation, &s);
        assert!(
            summary.starts_with("Phase 13 (2 of 5 roadmap phases complete)"),
            "{summary}"
        );
        assert!(summary.ends_with(tail), "{summary}");
        assert!(!summary.contains("Phase 13 of 5"), "{summary}");
    }
    for (phase, total, done) in [("3", 5, 2), ("4", 8, 1), ("5", 5, 2)] {
        let s = with_roadmap(
            sig("planning", Some(phase), Some(total)),
            Some(total),
            Some(done),
        );
        assert_eq!(
            build_summary("planning", &s),
            format!("Phase {phase} of {total} — needs a plan")
        );
    }
    let beyond = with_roadmap(sig("planning", Some("13.1"), Some(5)), Some(5), Some(2));
    assert_eq!(
        build_summary("planning", &beyond),
        "Phase 13.1 (2 of 5 roadmap phases complete) — needs a plan"
    );
    let unknown = with_roadmap(sig("planning", Some("13"), Some(5)), Some(5), None);
    assert_eq!(
        build_summary("planning", &unknown),
        "Phase 13 (5 roadmap phases) — needs a plan"
    );
    for phase in ["4", "5", "6", "6A", "6.1.1"] {
        let s = sig("planning", Some(phase), Some(5));
        assert_eq!(
            build_summary("planning", &s),
            format!("Phase {phase} of 5 — needs a plan")
        );
    }
    let empty = sig("planning", Some("4"), Some(5));
    assert_eq!(
        build_summary("planning", &empty),
        "Phase 4 of 5 — needs a plan"
    );
}

#[test]
fn completion_is_grounded_in_the_roadmap_counts_and_the_status_words() {
    let stale = with_roadmap(
        sig("Phase 7 shipped — PR #42", Some("7"), Some(4)),
        Some(7),
        Some(4),
    );
    assert_ne!(classify(&stale), "complete");
    let done = with_roadmap(
        sig("v1.0 milestone complete", Some("5"), Some(3)),
        Some(5),
        Some(5),
    );
    assert_eq!(classify(&done), "complete");
    for status in ["shipped", "done"] {
        let s = with_roadmap(sig(status, Some("5"), Some(5)), Some(5), Some(3));
        assert_ne!(classify(&s), "complete", "{status}");
    }
    assert_eq!(classify(&sig("complete", Some("5"), Some(5))), "complete");
    assert_ne!(classify(&sig("complete", Some("3"), Some(5))), "complete");
}

#[test]
fn a_continued_milestone_is_positioned_by_its_phase_list() {
    let nine = |status: &str, current: &str| {
        let mut s = sig(status, Some(current), Some(9));
        s.milestone_phases = range(9, 17);
        s
    };
    assert_eq!(
        build_summary("planning", &nine("planning", "9")),
        "Phase 1 of 9 — needs a plan"
    );
    let table = with_roadmap(nine("planning", "11"), None, Some(2));
    assert_eq!(
        build_summary("planning", &table),
        "Phase 3 of 9 — needs a plan"
    );
    let mut exec = nine("executing", "11");
    exec.progress = Some(40);
    assert_eq!(
        build_summary("executing", &exec),
        "Phase 3 of 9 · 40% · executing"
    );
    assert_eq!(
        build_summary("verify-pending", &nine("needs-review", "17")),
        "Phase 9 of 9 · ready to verify"
    );
    assert_ne!(classify(&nine("phase complete", "9")), "complete");
    assert_ne!(classify(&nine("complete", "16")), "complete");
    assert_eq!(classify(&nine("complete", "17")), "complete");
    let mut plain = sig("planning", Some("2"), Some(5));
    plain.milestone_phases = range(1, 5);
    assert_eq!(
        build_summary("planning", &plain),
        "Phase 2 of 5 — needs a plan"
    );
    let mut zero = sig("planning", Some("1"), Some(3));
    zero.milestone_phases = vec!["0".into(), "1".into(), "2".into()];
    assert_eq!(
        build_summary("planning", &zero),
        "Phase 2 of 3 — needs a plan"
    );
    let mut dotted = sig("planning", Some("9.1"), Some(3));
    dotted.milestone_phases = vec!["9".into(), "9.1".into(), "10".into()];
    assert_eq!(
        build_summary("planning", &dotted),
        "Phase 2 of 3 — needs a plan"
    );
    assert_eq!(
        build_summary("planning", &nine("planning", "40")),
        "Phase 40 of 9 — needs a plan"
    );
    assert_eq!(
        build_summary("planning", &sig("planning", Some("2"), Some(5))),
        "Phase 2 of 5 — needs a plan"
    );
}

#[test]
fn a_slash_name_is_written_for_the_runtime() {
    assert_eq!(
        format_gsd_slash("progress --next", "claude"),
        "/gsd-progress --next"
    );
    assert_eq!(
        format_gsd_slash("progress --next", "codex"),
        "$gsd-progress --next"
    );
    assert_eq!(format_gsd_slash("/gsd:Quick", "codex"), "$gsd-quick");
    assert_eq!(
        format_gsd_slash("$gsd-plan-phase", "cursor"),
        "/gsd-plan-phase"
    );
    assert_eq!(format_gsd_slash("/gsd:", "claude"), "");
}

#[test]
fn phases_compare_by_their_dotted_parts() {
    assert_eq!(compare_phase("9", "10"), Ordering::Less);
    assert_eq!(compare_phase("9.1", "9"), Ordering::Greater);
    assert_eq!(compare_phase("02", "2"), Ordering::Equal);
    assert_eq!(compare_phase("6A", "6"), Ordering::Greater);
}
