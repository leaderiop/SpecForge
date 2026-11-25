//! What a reader leaves of a section it maps in part (ADR 0391 D3, D8): each line no row reads is reported at its line,
//! never dropped silently, in every Tier 1 format that reads key lines, bullets or tables out of its sections.

mod common;

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use common::{creates, entries, fact, feed, find, has, import, manifest, paths};

/// What a leftover line's entry says.
const READS: &str = "table reads: no field holds it";

/// The import of `files`, with a roadmap of phases 1 and 2 unless `files` serve one.
fn run(files: Vec<(&str, &str)>) -> CommandOutput {
    let road = "# Roadmap: X\n\n### Phase 1: One\n**Goal**: g\n**Success Criteria**:\n  1. c\n\n### Phase 2: Two\n**Goal**: h\n**Success Criteria**:\n  1. d\n";
    let mut facts: Vec<_> = files.iter().map(|(path, text)| fact(path, text)).collect();
    if !files.iter().any(|(path, _)| *path == ROADMAP) {
        facts.push(fact(ROADMAP, road));
    }
    import(
        &CommandGraph::default(),
        &feed(facts, false, Some(&manifest(&["@gsd/core"]))),
    )
}

const ROADMAP: &str = ".planning/ROADMAP.md";

/// Asserts that `file` has an `unmapped` entry saying `says` at each of `lines`.
fn reported_at(out: &CommandOutput, file: &str, lines: &[u64], says: &str) {
    let reported = entries(out, file);
    for &line in lines {
        assert!(
            has(&reported, line, "unmapped", says),
            "{file}:{line} {says}: {reported:#?}"
        );
    }
}

#[test]
fn a_roadmap_reports_each_phase_line_it_does_not_read() {
    let roadmap = "# Roadmap: X\n\nA tagline.\n\n### Phase 1: One\n\nSome prose.\n**Goal**: g\nPlans: 2 plans\n\
                   **Success Criteria**:\n\n  1. c\n\n**Plans**: 1 plan\n\n### Phase 2: Two\n\n**Goal**: h\n**Plans**: TBD\n";
    let out = run(vec![(ROADMAP, roadmap)]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    // The Success Criteria list a blank line below its key is read.
    assert_eq!(
        find(&creates(&out), "phase", "01").fields["success_criteria"],
        json!(["c"])
    );
    reported_at(&out, ROADMAP, &[3, 7, 9], READS);
    // Each `**Plans**` line, not the first alone.
    let reported = entries(&out, ROADMAP);
    for line in [14, 19] {
        assert!(
            has(&reported, line, "computed", "plans_complete"),
            "{line}: {reported:#?}"
        );
    }
}

#[test]
fn requirements_report_each_line_no_row_reads() {
    let path = ".planning/REQUIREMENTS.md";
    let text = "# Requirements: X\n\n## v1 Requirements\n\nIntro.\n\n### Core\n\n- **CORE-01**: one\n\
                - Users can share a board\n\n## Out of Scope\n\n- Sharing — later\n\n---\n*Last updated: 2026-01-01*\n";
    let out = run(vec![(path, text)]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    assert_eq!(paths(&creates(&out), "requirement"), ["CORE-01"]);
    reported_at(&out, path, &[5, 10, 14], READS);
    assert!(
        entries(&out, path).iter().any(|(l, ..)| *l == 17),
        "the footer has its entry"
    );
}

#[test]
fn a_project_reports_its_tagline_and_a_constraint_intro() {
    let path = ".planning/PROJECT.md";
    let text = "# Odd\n\nA tagline.\n\n## What This Is\n\nW.\n\n## Core Value\n\nC.\n\n## Context\n\nX.\n\n\
                ## Constraints\n\nSome intro.\n\n- **Budget**: none — free\n";
    let out = run(vec![(path, text)]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let project = find(&creates(&out), "project", "odd");
    assert_eq!(
        project.fields["constraints"],
        json!([{"kind": "budget", "what": "none", "why": "free"}])
    );
    reported_at(&out, path, &[3, 19], READS);
}

#[test]
fn a_context_and_a_research_report_each_line_no_row_reads() {
    let context_path = ".planning/phases/02-two/02-CONTEXT.md";
    let research_path = ".planning/phases/02-two/02-RESEARCH.md";
    let context = "# C\n\nA note.\n\n## Phase Boundary\n\nB.\n\n<decisions>\n## Implementation Decisions\n\nIntro prose.\n\n\
                   ### Area\n\nArea prose.\n\n- **D-01:** one\n</decisions>\n";
    let research = "# R\n\n**Confidence:** HIGH\n\n## Summary\n\nS.\n\n## Standard Stack\n\n**Installation:**\n\n### Core\n\n\
                    | Library | Version | Purpose |\n|---|---|---|\n| a | 1 | b |\n\n## Common Pitfalls\n\nIntro.\n\n\
                    ### Pitfall 1: P\n\n**What goes wrong:** w\nMore detail.\n";
    let out = run(vec![(context_path, context), (research_path, research)]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    assert_eq!(paths(&all, "decision"), ["v1.0/02/D-01"]);
    assert_eq!(
        find(&all, "stack_choice", "v1.0/02/02-RESEARCH/SC1").fields["name"],
        "a"
    );
    reported_at(
        &out,
        context_path,
        &[3, 12, 16],
        "text no row of the NN-CONTEXT.md table reads",
    );
    reported_at(
        &out,
        research_path,
        &[11, 21, 26],
        "text no row of the NN-RESEARCH.md table reads",
    );
}

#[test]
fn a_summary_title_reports_the_text_past_its_one_liner() {
    let plan = ".planning/phases/01-one/01-01-PLAN.md";
    let summary = ".planning/phases/01-one/01-01-SUMMARY.md";
    let plan_text = "---\ntype: execute\nfiles_modified: [a.js]\n---\n\n<objective>\nDo it.\n</objective>\n\n<tasks>\n\
                     <task type=\"auto\">\n  <name>Task 1: T</name>\n  <action>a</action>\n  <done>d</done>\n</task>\n</tasks>\n";
    // The one-liner's paragraph runs on to line 12; line 14 is a paragraph of its own.
    let summary_text = "---\nphase: 01-one\nplan: 01\nsubsystem: core\nstatus: complete\nrequirements-completed: []\n---\n\n\
                        # Plan 01 Summary\n\n**It works.**\nand more.\n\nA tagline.\n\n## Accomplishments\n\nx\n";
    let out = run(vec![(plan, plan_text), (summary, summary_text)]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    assert_eq!(
        find(&creates(&out), "summary", "v1.0/01/01-01/01-01-SUMMARY").fields["one_liner"],
        "It works."
    );
    reported_at(&out, summary, &[12, 14], "other than its bold one-liner");
}
