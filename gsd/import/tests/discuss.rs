//! A phase's CONTEXT.md, DISCUSSION-LOG.md and RESEARCH.md through the command in process, against the taskboard
//! twin's `discuss.spec`, and hostile cases.
//!
//! Known differences from the twin (13-RESEARCH "Per-Fixture Mapping"): titles are the documents' own (the twin's are
//! hand-made, and titles compare nothing); `discussion.entries[*].notes` is carried, which the twin lacks (D9); the
//! research `summary` is the document's text where the twin paraphrases it (D8); a finding has no `topic` or `sources`
//! (D7); values keep their backticks and trailing periods (the comparator folds them).

mod common;

use gsd_import::limits::MAX_OPS;
use serde_json::json;
use specforge_extension_sdk::prelude::*;

use common::{
    creates, entries, fact, feed, find, has, import, import_fixture, manifest, paths, payload,
};

const CONTEXT: &str = ".planning/phases/02-persistence/02-CONTEXT.md";
const LOG: &str = ".planning/phases/02-persistence/02-DISCUSSION-LOG.md";
const RESEARCH: &str = ".planning/phases/02-persistence/02-RESEARCH.md";

/// The import of a roadmap of phases 1 and 2 and the served `files`.
fn run(files: Vec<(&str, &str)>) -> CommandOutput {
    let road = "# Roadmap: X\n\n### Phase 1: One\n**Goal**: g\n**Success Criteria**:\n  1. c\n\n### Phase 2: Two\n**Goal**: h\n**Success Criteria**:\n  1. d\n";
    let mut facts = vec![fact(".planning/ROADMAP.md", road)];
    facts.extend(files.into_iter().map(|(path, text)| fact(path, text)));
    import(
        &CommandGraph::default(),
        &feed(facts, false, Some(&manifest(&["@gsd/core"]))),
    )
}

#[test]
fn taskboard_context_takes_the_twins_values() {
    let out = import_fixture("taskboard");
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    let context = find(&all, "context", "v1.0/02/02-CONTEXT");
    assert_eq!(
        (context.title.as_str(), context.file.as_str()),
        ("Phase 2: Persistence - Context", "spec/gsd/phases/02.spec")
    );
    assert_eq!(
        context.fields,
        json!({
            "phase": "v1.0/02",
            "gathered": "2026-03-04",
            "boundary": "Tasks are saved to disk after every change and read back when the board starts. Listing and filtering are phase 3.",
            "discretion": ["The indent of the JSON file"],
            "specifics": "No specific requirements — open to standard approaches",
            "canonical_refs": [
                {"topic": "No external specs — requirements are fully captured in decisions above", "paths": []}
            ],
            "reusable_assets": ["`src/tasks.js`: the task list and its operations"],
            "patterns": ["ES modules, no build step"],
            "integration_points": ["`src/cli.js` loads the board before each command and saves it after"],
        })
    );
    assert_eq!(
        paths(&all, "decision"),
        [
            "D-01",
            "D-02",
            "v1.0/02/D-01",
            "v1.0/02/D-02",
            "v1.0/02/D-03"
        ]
    );
    for (id, area, text) in [
        (
            "D-01",
            "Storage format",
            "Tasks are kept in a single JSON file, `tasks.json`, in the working directory",
        ),
        (
            "D-02",
            "Storage format",
            "The file is written to a temporary name and renamed over `tasks.json`, so a crash never leaves half a file",
        ),
        (
            "D-03",
            "Titles",
            "A title is stored exactly as typed, with no trimming",
        ),
    ] {
        let decision = find(&all, "decision", &format!("v1.0/02/{id}"));
        assert_eq!(
            decision.fields,
            json!({"phase": "v1.0/02", "area": area, "text": text, "origin": "context"}),
            "{id}"
        );
    }
    let idea = find(&all, "deferred_idea", "v1.0/02/DI1");
    assert_eq!(
        idea.fields,
        json!({
            "phase": "v1.0/02",
            "origin": "discussion",
            "text": "Keeping the file in step across two machines",
            "why": "v2, SYNC-01",
        })
    );
    let reported = entries(&out, CONTEXT);
    for (line, outcome, says) in [
        (4, "computed", "context.status is computed"),
        (17, "filled", "a decision of a phase's CONTEXT.md"),
        (38, "ignored", "the template's instruction"),
        (61, "filled", "an idea deferred"),
        (67, "unmapped", "prose"),
        (68, "unmapped", "prose"),
    ] {
        assert!(
            has(&reported, line, outcome, says),
            "{line} {says}: {reported:#?}"
        );
    }
    assert!(
        !reported
            .iter()
            .any(|(l, _, _, r)| *l == 0 || r == "unknown file"),
        "the file is read, not reported whole: {reported:#?}"
    );
}

#[test]
fn taskboard_discussion_takes_the_twins_entries() {
    let out = import_fixture("taskboard");
    let all = creates(&out);
    let discussion = find(&all, "discussion", "v1.0/02/02-DISCUSSION-LOG");
    assert_eq!(discussion.title, "Phase 2: Persistence - Discussion Log");
    // The twin's entries, and the notes it lacks (D9).
    assert_eq!(
        discussion.fields,
        json!({
            "phase": "v1.0/02",
            "entries": [
                {
                    "area": "Storage format",
                    "options": ["One JSON file", "SQLite"],
                    "chosen": "One JSON file",
                    "mode": "interactive",
                    "notes": "The project ships as one script with no dependencies, so SQLite is out.",
                },
                {
                    "area": "Titles",
                    "options": ["Stored as typed", "Trimmed"],
                    "chosen": "Stored as typed",
                    "mode": "interactive",
                    "notes": "A title is the user's own words.",
                },
            ],
        })
    );
    let reported = entries(&out, LOG);
    for (line, outcome, says) in [
        (3, "ignored", "the template's instruction"),
        (6, "redundant", "the directory and CONTEXT.md"),
        (7, "redundant", "the directory and CONTEXT.md"),
        (8, "redundant", "the directory and CONTEXT.md"),
        (12, "filled", "the only shape a discussion log has"),
        (14, "unmapped", "no member of a discussion entry"),
        (36, "redundant", "CONTEXT.md gives it"),
        (40, "redundant", "CONTEXT.md gives it"),
        (46, "unmapped", "prose"),
    ] {
        assert!(
            has(&reported, line, outcome, says),
            "{line} {says}: {reported:#?}"
        );
    }
}

const PHASE_CONTEXT: &str = ".planning/phases/02-two/02-CONTEXT.md";

#[test]
fn a_context_reports_bad_decisions_and_deferred_ideas() {
    let text = "# Phase 2: Two - Context\n\n**Gathered:** soon\n\n<domain>\n## Phase Boundary\n\nB.\n</domain>\n\n\
                <decisions>\n## Implementation Decisions\n\n### Area\n- **D-1 x:** bad id\n- **D-01:** first\n\
                - no id here\n- **D-01:** again\n\n</decisions>\n\n<deferred>\n## Deferred Ideas\n\n\
                - An idea with no reason\n- Sharing — v2\n</deferred>\n\n## Odd Section\n\ntext\n";
    let out = run(vec![(PHASE_CONTEXT, text)]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    assert_eq!(paths(&all, "decision"), ["v1.0/02/D-01"]);
    assert_eq!(
        find(&all, "decision", "v1.0/02/D-01").fields["text"],
        "first"
    );
    assert_eq!(paths(&all, "deferred_idea"), ["v1.0/02/DI1", "v1.0/02/DI2"]);
    assert_eq!(
        find(&all, "deferred_idea", "v1.0/02/DI1").fields,
        json!({"phase": "v1.0/02", "origin": "discussion", "text": "An idea with no reason"})
    );
    let context = find(&all, "context", "v1.0/02/02-CONTEXT");
    assert_eq!(
        context.fields,
        json!({"phase": "v1.0/02", "boundary": "B."})
    );
    let reported = entries(&out, PHASE_CONTEXT);
    for (line, says) in [
        (3, "not a YYYY-MM-DD date"),
        (15, "`D-1 x` is no decision"),
        (17, "no `**D-NN:**` id"),
        (18, "is given twice"),
        (25, "no ` — why`"),
        (29, "a section no row of the NN-CONTEXT.md table names"),
    ] {
        assert!(
            has(&reported, line, "unmapped", says),
            "{line} {says}: {reported:#?}"
        );
    }
}

#[test]
fn a_sub_section_is_never_its_parents_field() {
    let context =
        "# C\n\n## Phase Boundary\n\nB.\n\n### Detail\n\nMore.\n\n## Specific Ideas\n\nS.\n";
    let research = format!("{HEAD}### Detail\n\nMore.\n");
    let out = run(vec![(CONTEXT, context), (RESEARCH, research.as_str())]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    assert_eq!(
        find(&all, "context", "v1.0/02/02-CONTEXT").fields,
        json!({"phase": "v1.0/02", "boundary": "B.", "specifics": "S."})
    );
    assert_eq!(
        find(&all, "research", "v1.0/02/02-RESEARCH").fields["summary"],
        "S."
    );
    for (file, line, says) in [
        (CONTEXT, 7, "no row of the NN-CONTEXT.md table"),
        (RESEARCH, 9, "no row of the NN-RESEARCH.md table"),
    ] {
        let reported = entries(&out, file);
        assert!(
            has(&reported, line, "unmapped", says),
            "{line} {says}: {reported:#?}"
        );
    }
}

#[test]
fn a_context_without_a_boundary_is_dropped_and_its_decisions_kept() {
    let text = "# C\n\n<decisions>\n## Implementation Decisions\n\n### A\n- **D-01:** kept\n</decisions>\n";
    let out = run(vec![(PHASE_CONTEXT, text)]);
    let all = creates(&out);
    assert!(paths(&all, "context").is_empty());
    assert_eq!(paths(&all, "decision"), ["v1.0/02/D-01"]);
    assert!(has(
        &entries(&out, PHASE_CONTEXT),
        1,
        "unmapped",
        "required boundary has no source"
    ));
}

#[test]
fn a_context_of_no_roadmap_phase_imports_nothing() {
    let out = run(vec![(
        ".planning/phases/07-seven/07-CONTEXT.md",
        "## Phase Boundary\n\nB.\n",
    )]);
    assert!(!creates(&out).iter().any(|c| c.kind == "context"));
    assert!(has(
        &entries(&out, ".planning/phases/07-seven/07-CONTEXT.md"),
        1,
        "unmapped",
        "phase 07 is no phase of ROADMAP.md"
    ));
}

#[test]
fn a_discussion_with_two_marked_options_takes_the_users_choice() {
    let text = "# Log\n\n## Area\n\n| Option | Selected |\n|---|---|\n| A | ✓ |\n| B | ✓ |\n\n\
                **User's choice:** B, after all\n";
    let path = ".planning/phases/02-two/02-DISCUSSION-LOG.md";
    let out = run(vec![(path, text)]);
    let all = creates(&out);
    assert_eq!(
        find(&all, "discussion", "v1.0/02/02-DISCUSSION-LOG").fields["entries"],
        json!([{"area": "Area", "options": ["A", "B"], "chosen": "B, after all", "mode": "interactive"}])
    );
    assert!(has(
        &entries(&out, path),
        5,
        "unmapped",
        "2 options are marked ✓"
    ));
}

#[test]
fn taskboard_research_takes_the_twins_values() {
    let out = import_fixture("taskboard");
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    let research = find(&all, "research", "v1.0/02/02-RESEARCH");
    assert_eq!(research.title, "Phase 2: Persistence - Research");
    assert_eq!(
        research.fields,
        json!({
            "phase": "v1.0/02",
            "researched": "2026-03-04",
            "domain": "File-backed storage for a small Node 20 command line tool",
            "confidence": "high",
            "summary": "A board of a few hundred tasks fits in one JSON file. Node's own `fs` module reads and writes it, and a write to a temporary name followed by a rename keeps the file whole if the process dies mid-write.",
            "user_constraints": ["v1.0/02/D-01", "v1.0/02/D-02", "v1.0/02/D-03"],
            "responsibility_map": [{"capability": "Persist the board", "tier": "Database/Storage"}],
        })
    );
    let item = |kind: &str, id: &str| find(&all, kind, &format!("v1.0/02/02-RESEARCH/{id}")).fields;
    let scoped = |mut v: serde_json::Value| {
        v["research"] = json!("v1.0/02/02-RESEARCH");
        v
    };
    assert_eq!(
        item("finding", "F1"),
        scoped(json!({
            "text": "`JSON.stringify` the board and `fs.renameSync` a temporary file over `tasks.json`.",
            "confidence": "high",
        }))
    );
    assert_eq!(
        item("stack_choice", "SC1"),
        scoped(
            json!({"name": "node:fs", "version": "20", "purpose": "Read and write `tasks.json`", "tier": "core"})
        )
    );
    assert_eq!(
        item("pattern", "P1"),
        scoped(json!({
            "name": "Write then rename",
            "what": "Write the new content to a temporary file in the same directory, then rename it over the target.",
            "when": "Every save.",
        }))
    );
    assert_eq!(
        item("pattern", "P2"),
        scoped(
            json!({"name": "Writing in place", "what": "a crash mid-write leaves a truncated file.", "anti": true})
        )
    );
    assert_eq!(
        item("dont_hand_roll", "DH1"),
        scoped(json!({
            "problem": "Two processes saving at once",
            "instead": "One process per board",
            "why": "Lock files go stale after a crash",
        }))
    );
    assert_eq!(
        item("pitfall", "PF1"),
        scoped(json!({
            "name": "Torn writes",
            "goes_wrong": "A crash during `writeFileSync` leaves a half-written `tasks.json`.",
            "why": "`writeFileSync` truncates the file before it writes.",
            "avoid": "Write then rename.",
            "warning_signs": "`JSON.parse` throws on start.",
        }))
    );
    assert_eq!(
        item("open_question", "OQ1"),
        scoped(json!({
            "question": "Is one file enough for a thousand tasks?",
            "known": "a task is a few dozen bytes.",
            "unclear": "how large a board grows in practice.",
            "recommendation": "revisit if a board passes ten thousand tasks.",
            "resolved_by": "v1.0/02/D-01",
        }))
    );
    assert_eq!(
        item("source", "SRC1"),
        scoped(json!({
            "url": "https://nodejs.org/docs/latest-v20.x/api/fs.html",
            "tier": "primary",
            "checked": "`renameSync` replaces the target atomically on one filesystem",
        }))
    );
    let reported = entries(&out, RESEARCH);
    for (line, outcome, says) in [
        (15, "redundant", "the phase's CONTEXT.md gives it"),
        (18, "redundant", "the phase's CONTEXT.md gives it"),
        (25, "unmapped", "no member of a responsibility map"),
        (43, "unmapped", "no field of a stack choice"),
        (63, "unmapped", "no field of a don't-hand-roll"),
        (97, "unmapped", "prose"),
    ] {
        assert!(
            has(&reported, line, outcome, says),
            "{line} {says}: {reported:#?}"
        );
    }
    assert!(
        !reported
            .iter()
            .any(|(_, _, o, r)| o == "unmapped" && r.contains("no row of the")),
        "every section is a row: {reported:#?}"
    );
}

/// The first 8 lines of a research with its two required fields.
const HEAD: &str = "# R\n\n**Confidence:** HIGH\n\n## Summary\n\nS.\n\n";

#[test]
fn a_research_leaves_out_decisions_its_context_does_not_give() {
    let context = "# C\n\n## Phase Boundary\n\nB.\n\n## Implementation Decisions\n\n### Area\n- **D-01:** one\n";
    let research = format!(
        "{HEAD}## User Constraints\n\n### Locked Decisions\n- D-01: one\n- D-09: nine\n\n\
         ## Open Questions\n\n1. **Q?** — RESOLVED: by D-07.\n   - Something else: x\n   - What we know: k\n\
         \x20  - What's unclear: u\n   - Recommendation: r\n\n## Extra\n\ntext\n"
    );
    let out = run(vec![(CONTEXT, context), (RESEARCH, research.as_str())]);
    let all = creates(&out);
    assert_eq!(
        find(&all, "research", "v1.0/02/02-RESEARCH").fields,
        json!({"phase": "v1.0/02", "confidence": "high", "summary": "S.", "user_constraints": ["v1.0/02/D-01"]})
    );
    assert_eq!(
        find(&all, "open_question", "v1.0/02/02-RESEARCH/OQ1").fields,
        json!({"research": "v1.0/02/02-RESEARCH", "question": "Q?", "known": "k", "unclear": "u", "recommendation": "r"})
    );
    let reported = entries(&out, RESEARCH);
    for (line, says) in [
        (13, "`D-09` names no decision"),
        (17, "`D-07` names no decision"),
        (18, "no member of an open question"),
        (23, "no row of the NN-RESEARCH.md table"),
    ] {
        assert!(
            has(&reported, line, "unmapped", says),
            "{line} {says}: {reported:#?}"
        );
    }
}

#[test]
fn a_research_of_a_bad_confidence_is_dropped_with_its_items() {
    let text = "# R\n\n**Confidence:** Sure\n\n## Summary\n\nS.\n\n## Sources\n\n### Primary\n- https://a.test - n\n";
    let out = run(vec![(RESEARCH, text)]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    assert!(
        !all.iter()
            .any(|c| c.kind == "research" || c.kind == "source"),
        "{all:#?}"
    );
    let reported = entries(&out, RESEARCH);
    assert!(
        has(&reported, 3, "unmapped", "none of high, medium, low"),
        "{reported:#?}"
    );
    assert!(
        has(&reported, 12, "unmapped", "parent not imported"),
        "{reported:#?}"
    );
}

#[test]
fn a_research_source_holding_a_control_character_is_reported_not_created() {
    let url = "https://x.test/a\u{7}b";
    let text = format!("{HEAD}## Sources\n\n### Primary\n- {url} - note\n");
    let out = run(vec![(RESEARCH, text.as_str())]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    // No .spec file holds a control character: the host would refuse the whole plan.
    assert!(
        !creates(&out)
            .iter()
            .any(|c| c.path == "v1.0/02/02-RESEARCH/SRC1"),
        "the source is created"
    );
    assert!(
        entries(&out, RESEARCH)
            .iter()
            .any(|(_, _, outcome, reason)| outcome == "unmapped"
                && reason.contains("SRC1: its title holds a control or bidirectional character")),
        "{:#?}",
        entries(&out, RESEARCH)
    );
    assert!(
        !payload(&out)["entries"].to_string().contains('\u{7}')
            && !entries(&out, RESEARCH)
                .iter()
                .any(|(_, s, _, r)| s.contains('\u{7}') || r.contains('\u{7}')),
        "the report holds no raw control character"
    );
}

#[test]
fn a_research_of_5000_stack_rows_is_truncated() {
    let mut text =
        "# R\n\n## Standard Stack\n\n### Core\n| Library | Version | Purpose |\n|---|---|---|\n"
            .to_string();
    for n in 0..5_000 {
        text.push_str(&format!("| lib{n} | 1 | p |\n"));
    }
    let out = run(vec![(RESEARCH, text.as_str())]);
    assert_eq!(out.exit_code, 1, "{}", out.stdout);
    assert!(out.plan.is_none());
    assert!(
        out.stderr.contains(&format!("truncated at {MAX_OPS}")),
        "{}",
        out.stderr
    );
}

#[test]
fn a_context_canonical_ref_takes_its_topic_heading_and_paths() {
    let text = "# C\n\n<domain>\n## Phase Boundary\n\nB.\n</domain>\n\n<canonical_refs>\n## Canonical References\n\n\
                **Downstream agents MUST read these before planning or implementing.**\n\n### Feed\n\
                - `docs/feed.md` — Feed rules\n- `docs/adr-1.md`\n</canonical_refs>\n";
    let out = run(vec![(PHASE_CONTEXT, text)]);
    let context = find(&creates(&out), "context", "v1.0/02/02-CONTEXT");
    assert_eq!(
        context.fields["canonical_refs"],
        json!([
            {"topic": "Feed: Feed rules", "paths": ["docs/feed.md"]},
            {"topic": "Feed", "paths": ["docs/adr-1.md"]},
        ])
    );
}
