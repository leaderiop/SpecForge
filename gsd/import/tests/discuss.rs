//! A phase's CONTEXT.md and DISCUSSION-LOG.md through the command in process, against the taskboard twin's
//! `discuss.spec`, and hostile cases.
//!
//! Known differences from the twin (13-RESEARCH "Per-Fixture Mapping"): titles are the documents' own (the twin's are
//! hand-made, and titles compare nothing); `discussion.entries[*].notes` is carried, which the twin lacks (D9).

mod common;

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use common::{creates, entries, fact, feed, find, has, import, import_fixture, manifest, paths};

const CONTEXT: &str = ".planning/phases/02-persistence/02-CONTEXT.md";
const LOG: &str = ".planning/phases/02-persistence/02-DISCUSSION-LOG.md";

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
