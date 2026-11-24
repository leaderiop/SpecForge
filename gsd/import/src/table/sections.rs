//! The rows of each Tier 1 format (13-RESEARCH Part B, "Tier 1 section and key tables"), one `const` per format. A key
//! is the section, key or row as the document writes it; a quick plan's and summary's keys start `PLAN ` / `SUMMARY `.

use super::How::{Blocks, Id, List, Ref, Text, Word};
use super::{Home, How, ID, LATER, Row, TITLE};

const fn row(file: &'static str, key: &'static str, homes: &'static [Home]) -> Row {
    Row { file, key, homes }
}

const fn to(kind: &'static str, field: &'static str, how: How) -> Home {
    Home::To { kind, field, how }
}

const fn computed(kind: &'static str, field: &'static str) -> Home {
    Home::Computed { kind, field }
}

/// The entity's id.
const fn id(kind: &'static str) -> Home {
    to(kind, ID, Id)
}

/// The entity's title.
const fn title(kind: &'static str) -> Home {
    to(kind, TITLE, Id)
}

const fn filled(
    kind: &'static str,
    field: &'static str,
    value: &'static str,
    why: &'static str,
) -> Home {
    Home::Filled {
        kind,
        field,
        value,
        why,
    }
}

const fn reported(why: &'static str) -> Home {
    Home::Reported(why)
}

const fn redundant(why: &'static str) -> Home {
    Home::Redundant(why)
}

const PROSE: &str = "prose: no field holds it";
const EMPTY: &str = "an empty placeholder";
const VERDICT: &str = "the verifier's per-item results are verdicts, never imported (ADR 0391 D5): the plans give the entities";
const CONTEXT_MD: &str = "the phase's CONTEXT.md gives it";
const INSTRUCTION: &str = "the template's instruction to agents";
const HARNESS_TS: &str = "a harness timestamp the import does not write (ADR 0391 D7)";

const P: &str = "PROJECT.md";
#[rustfmt::skip]
pub const PROJECT: &[Row] = &[
    row(P, "# Name", &[id("project"), title("project"), to("project", "name", Text)]),
    row(P, "## What This Is", &[to("project", "what", Text)]),
    row(P, "## Core Value", &[to("project", "core_value", Text)]),
    row(P, "## Requirements", &[redundant("REQUIREMENTS.md gives the requirements; its ### Validated and ### Active lists are computed from them")]),
    row(P, "## Business Context", &[reported(LATER)]),
    row(P, "## Current Milestone", &[reported(LATER)]),
    row(P, "### Validated", &[computed("project", "validated")]),
    row(P, "### Active", &[computed("project", "active")]),
    row(P, "(None yet", &[Home::Ignored(EMPTY)]),
    row(P, "### Out of Scope", &[to("project", "out_of_scope.item", Blocks), to("project", "out_of_scope.why", Blocks)]),
    row(P, "## Context", &[to("project", "context", Text)]),
    row(P, "## Constraints", &[
        to("project", "constraints.kind", Word),
        to("project", "constraints.what", Blocks),
        to("project", "constraints.why", Blocks),
        filled("project", "constraints.kind", "other", "a label that names no kind keeps its words in `what`"),
    ]),
    row(P, "## Key Decisions", &[
        id("decision"),
        to("decision", "text", Text),
        to("decision", "rationale", Text),
        to("decision", "outcome", Word),
        filled("decision", "origin", "project", "a Key Decisions row is a project decision"),
        to("project", "key_decisions", Ref),
    ]),
    row(P, "*Last updated:", &[computed("project", "updated")]),
];

const RQ: &str = "REQUIREMENTS.md";
#[rustfmt::skip]
pub const REQUIREMENTS: &[Row] = &[
    row(RQ, "**Defined:**", &[to("requirement", "defined", Text)]),
    row(RQ, "**Core Value:**", &[redundant("PROJECT.md's ## Core Value gives it")]),
    row(RQ, "## v1 Requirements", &[to("requirement", "tier", Word)]),
    row(RQ, "## v2 Requirements", &[to("requirement", "tier", Word)]),
    row(RQ, "## Out of Scope", &[to("requirement", "tier", Word)]),
    row(RQ, "### Category", &[to("requirement", "category", Text)]),
    row(RQ, "- **ID**: text", &[id("requirement"), title("requirement"), to("requirement", "text", Text)]),
    row(RQ, "- [x]", &[computed("requirement", "status")]),
    row(RQ, "| Feature | Reason |", &[
        filled("requirement", ID, "OOS-<NN>", "GSD gives an Out of Scope row no id, so the import numbers the rows"),
        title("requirement"),
        to("requirement", "text", Text),
        to("requirement", "reason", Text),
        filled("requirement", "category", "Out of Scope", "the row's section"),
    ]),
    row(RQ, "## Traceability", &[redundant("ROADMAP.md's **Requirements** lines give each phase's requirements")]),
    row(RQ, "**Coverage:**", &[redundant("the coverage counts follow from the roadmap's requirements")]),
    row(RQ, "*Requirements defined:", &[reported(PROSE)]),
    row(RQ, "*Last updated:", &[reported("the graph keeps no edit date of a file")]),
];

const RM: &str = "ROADMAP.md";
#[rustfmt::skip]
pub const ROADMAP: &[Row] = &[
    row(RM, "(no milestone)", &[filled(
        "milestone",
        ID,
        "v1.0",
        "the roadmap names no milestone: GSD's initial roadmap is v1.0; its title is authored after import",
    )]),
    row(RM, "# Roadmap:", &[redundant("PROJECT.md's # Name names the project")]),
    row(RM, "## Milestones", &[id("milestone"), title("milestone")]),
    row(RM, "milestone status", &[computed("milestone", "state")]),
    row(RM, "**Milestone Goal:**", &[to("milestone", "goal", Text)]),
    row(RM, "## Phase Details", &[Home::Ignored("a heading over the phases, each mapped under its own heading")]),
    row(RM, "<details>", &[to("phase", "milestone", Ref)]),
    row(RM, "## Overview", &[reported(PROSE)]),
    row(RM, "## Phases", &[computed("phase", "status")]),
    row(RM, "**Plans**: N/M plans complete", &[computed("phase", "plans_complete")]),
    row(RM, "## Progress", &[computed("phase", "completed")]),
    row(RM, "Phase Numbering", &[reported(PROSE)]),
    row(RM, "### Phase N: Name", &[id("phase"), title("phase"), to("phase", "slug", Text)]),
    row(RM, "(INSERTED)", &[computed("phase", "inserted")]),
    row(RM, "**Goal**", &[to("phase", "goal", Text)]),
    row(RM, "**Depends on**", &[to("phase", "depends_on", Ref)]),
    row(RM, "Nothing (first phase)", &[Home::Ignored(EMPTY)]),
    row(RM, "**Requirements**", &[to("phase", "requirements", Ref)]),
    row(RM, "TBD", &[Home::Ignored(EMPTY)]),
    row(RM, "**Success Criteria**", &[to("phase", "success_criteria", List)]),
    row(RM, "**Mode**", &[reported(LATER)]),
    row(RM, "Plans:", &[title("plan")]),
    row(RM, "TBD (run /gsd-plan-phase", &[Home::Ignored(EMPTY)]),
];

const ST: &str = "STATE.md";
/// STATE.md maps nothing (13-RESEARCH Part B): a section no row names is unmapped, "derived".
#[rustfmt::skip]
pub const STATE: &[Row] = &[
    row(ST, "---", &[computed("milestone", "progress")]),
    row(ST, "## Current Position", &[computed("phase", "status")]),
    row(ST, "## Session Continuity", &[Home::Runtime("a session's state: a pause needs the host's clock and who paused, and the session is over once the project is imported")]),
    row(ST, "### Blockers/Concerns", &[reported(LATER)]),
    row(ST, "None yet.", &[Home::Ignored(EMPTY)]),
];

/// A phase plan's and a quick plan's keys, tags and tasks: only where the id comes from differs.
macro_rules! plan_rows {
    ($file:expr, $prefix:literal) => {
        &[
            row($file, concat!($prefix, "type"), &[to("plan", "type", Word)]),
            row(
                $file,
                concat!($prefix, "autonomous"),
                &[computed("plan", "autonomous")],
            ),
            row(
                $file,
                concat!($prefix, "files_modified"),
                &[to("plan", "files_modified", List)],
            ),
            row(
                $file,
                concat!($prefix, "must_haves.truths"),
                &[
                    id("truth"),
                    title("truth"),
                    to("truth", "plan", Ref),
                    to("truth", "text", Text),
                ],
            ),
            row(
                $file,
                concat!($prefix, "must_haves.artifacts"),
                &[
                    id("artifact"),
                    title("artifact"),
                    to("artifact", "plan", Ref),
                    to("artifact", "path", Text),
                    to("artifact", "provides", Text),
                    to("artifact", "contains", Text),
                    to("artifact", "min_lines", Text),
                    to("artifact", "exports", List),
                ],
            ),
            row(
                $file,
                concat!($prefix, "must_haves.key_links"),
                &[
                    id("key_link"),
                    title("key_link"),
                    to("key_link", "plan", Ref),
                    to("key_link", "from", Text),
                    to("key_link", "to", Text),
                    to("key_link", "via", Text),
                    to("key_link", "pattern", Text),
                ],
            ),
            row(
                $file,
                concat!($prefix, "must_haves.prohibitions"),
                &[reported(LATER)],
            ),
            row($file, concat!($prefix, "files_deleted"), &[reported(LATER)]),
            row(
                $file,
                concat!($prefix, "coupling_justified"),
                &[reported(LATER)],
            ),
            row($file, concat!($prefix, "gap_closure"), &[reported(LATER)]),
            row($file, concat!($prefix, "agent_hint"), &[reported(LATER)]),
            row($file, concat!($prefix, "estimate"), &[reported(LATER)]),
            row($file, concat!($prefix, "user_setup"), &[reported(LATER)]),
            row(
                $file,
                concat!($prefix, "<objective>"),
                &[to("plan", "objective", Text)],
            ),
            row(
                $file,
                concat!($prefix, "Purpose:"),
                &[to("plan", "purpose", Text)],
            ),
            row(
                $file,
                concat!($prefix, "Output:"),
                &[to("plan", "output", Text)],
            ),
            row(
                $file,
                concat!($prefix, "<execution_context>"),
                &[reported(
                    "@-references to GSD's own files: `specforge gsd reference` serves them",
                )],
            ),
            row(
                $file,
                concat!($prefix, "<context>"),
                &[to("plan", "context_refs", List)],
            ),
            row(
                $file,
                concat!($prefix, "<context> @.planning/"),
                &[reported("GSD-internal: the spec replaces it")],
            ),
            row(
                $file,
                concat!($prefix, "<task type=\"auto|tdd|tracer\">"),
                &[
                    id("task"),
                    title("task"),
                    to("task", "plan", Ref),
                    to("task", "type", Word),
                    to("task", "files", List),
                    to("task", "read_first", List),
                    to("task", "action", Text),
                    to("task", "verify", Blocks),
                    to("task", "verify_text", Text),
                    to("task", "acceptance", List),
                    to("task", "done", Text),
                ],
            ),
            row(
                $file,
                concat!($prefix, "<task type=\"checkpoint:*\">"),
                &[reported(LATER)],
            ),
            row(
                $file,
                concat!($prefix, "<behavior>"),
                &[reported(
                    "a TDD task's expected behaviours: no field holds them, its truths and tests do",
                )],
            ),
            row(
                $file,
                concat!($prefix, "<verification>"),
                &[to("plan", "verification", List)],
            ),
            row(
                $file,
                concat!($prefix, "<success_criteria>"),
                &[to("plan", "success_criteria", List)],
            ),
            row(
                $file,
                concat!($prefix, "<output>"),
                &[reported("an instruction to the executor")],
            ),
            row(
                $file,
                concat!($prefix, "<threat_model> ## Trust Boundaries"),
                &[
                    id("trust_boundary"),
                    to("trust_boundary", "plan", Ref),
                    to("trust_boundary", "boundary", Text),
                    to("trust_boundary", "description", Text),
                ],
            ),
            row(
                $file,
                concat!($prefix, "<threat_model> ## STRIDE Threat Register"),
                &[
                    id("threat"),
                    to("threat", "plan", Ref),
                    to("threat", "category", Word),
                    to("threat", "component", Text),
                    to("threat", "severity", Word),
                    to("threat", "disposition", Word),
                    to("threat", "mitigation", Text),
                ],
            ),
        ]
    };
}

/// A phase summary's and a quick summary's keys and sections.
macro_rules! summary_rows {
    ($file:expr, $prefix:literal) => {
        &[
            row(
                $file,
                concat!($prefix, "file stem"),
                &[id("summary"), to("summary", "plan", Ref)],
            ),
            row(
                $file,
                concat!($prefix, "status"),
                &[to("summary", "status", Word)],
            ),
            row(
                $file,
                concat!($prefix, "one-liner"),
                &[to("summary", "one_liner", Text)],
            ),
            row(
                $file,
                concat!($prefix, "# title"),
                &[redundant("the plan's title names the summary")],
            ),
            row(
                $file,
                concat!($prefix, "tags"),
                &[to("summary", "tags", List)],
            ),
            row(
                $file,
                concat!($prefix, "provides"),
                &[to("summary", "provides", List)],
            ),
            row(
                $file,
                concat!($prefix, "affects"),
                &[to("summary", "affects", List)],
            ),
            row(
                $file,
                concat!($prefix, "tech-stack.added"),
                &[to("summary", "tech_added", List)],
            ),
            row(
                $file,
                concat!($prefix, "tech-stack.patterns"),
                &[to("summary", "tech_patterns", List)],
            ),
            row(
                $file,
                concat!($prefix, "coverage"),
                &[
                    id("deliverable"),
                    to("deliverable", "summary", Ref),
                    to("deliverable", "description", Text),
                    to("deliverable", "requirement", Ref),
                    to("deliverable", "human_judgment", Word),
                    to("deliverable", "rationale", Text),
                    to("deliverable", "checks.kind", Blocks),
                    to("deliverable", "checks.ref", Blocks),
                ],
            ),
            row(
                $file,
                concat!($prefix, "coverage verification status"),
                &[computed("deliverable", "checks.status")],
            ),
            row(
                $file,
                concat!($prefix, "requires"),
                &[to("summary", "requires", Blocks)],
            ),
            row(
                $file,
                concat!($prefix, "requires: [plan ids]"),
                &[reported(
                    "a list of plan ids names no `provides`, which a requires block needs",
                )],
            ),
            row($file, concat!($prefix, "key-decisions"), &[reported(LATER)]),
            row(
                $file,
                concat!($prefix, "key-files"),
                &[
                    computed("summary", "files_created"),
                    computed("summary", "files_modified"),
                ],
            ),
            row(
                $file,
                concat!($prefix, "duration"),
                &[computed("summary", "duration")],
            ),
            row($file, concat!($prefix, "started"), &[reported(HARNESS_TS)]),
            row(
                $file,
                concat!($prefix, "completed"),
                &[reported(HARNESS_TS)],
            ),
            row(
                $file,
                concat!($prefix, "## Task Commits"),
                &[reported(
                    "git history is the collector's: gsd-git writes commit entities",
                )],
            ),
            row(
                $file,
                concat!($prefix, "## Performance"),
                &[reported(PROSE)],
            ),
            row(
                $file,
                concat!($prefix, "## Accomplishments"),
                &[reported(PROSE)],
            ),
            row(
                $file,
                concat!($prefix, "## Issues Encountered"),
                &[reported(PROSE)],
            ),
            row(
                $file,
                concat!($prefix, "## Next Phase Readiness"),
                &[reported(LATER)],
            ),
            row(
                $file,
                concat!($prefix, "## Deviations from Plan"),
                &[reported(LATER)],
            ),
            row(
                $file,
                concat!($prefix, "## Threat Flags"),
                &[to("summary", "threat_flags", List)],
            ),
        ]
    };
}

const PL: &str = "NN-MM-PLAN.md";
#[rustfmt::skip]
pub const PLAN: &[Row] = &[
    row(PL, "file name", &[id("plan"), to("plan", "phase", Ref)]),
    row(PL, "phase, plan", &[redundant("the file name gives them; a mismatch is reported")]),
    row(PL, "wave", &[computed("plan", "wave"), to("plan", "wave0", Word)]),
    row(PL, "depends_on", &[to("plan", "depends_on", Ref)]),
    row(PL, "requirements", &[to("plan", "requirements", Ref)]),
];
pub const PLAN_SHARED: &[Row] = plan_rows!(PL, "");

const SM: &str = "NN-MM-SUMMARY.md";
#[rustfmt::skip]
pub const SUMMARY: &[Row] = &[
    row(SM, "phase, plan", &[redundant("the file name gives them; a mismatch is reported")]),
    row(SM, "subsystem", &[to("summary", "subsystem", Text)]),
    row(SM, "requirements-completed", &[to("summary", "requirements_completed", Ref)]),
];
pub const SUMMARY_SHARED: &[Row] = summary_rows!(SM, "");

const CX: &str = "NN-CONTEXT.md";
#[rustfmt::skip]
pub const CONTEXT: &[Row] = &[
    row(CX, "file stem", &[id("context"), to("context", "phase", Ref)]),
    row(CX, "# Phase N: Name - Context", &[title("context")]),
    row(CX, "**Gathered:**", &[to("context", "gathered", Text)]),
    row(CX, "**Status:**", &[computed("context", "status")]),
    row(CX, "## Phase Boundary", &[to("context", "boundary", Text)]),
    row(CX, "- **D-NN:**", &[
        id("decision"),
        to("decision", "phase", Ref),
        to("decision", "area", Text),
        to("decision", "text", Text),
        filled("decision", "origin", "context", "a decision of a phase's CONTEXT.md"),
    ]),
    row(CX, "### Claude's Discretion", &[to("context", "discretion", List)]),
    row(CX, "<specifics>", &[to("context", "specifics", Text)]),
    row(CX, "<canonical_refs>", &[to("context", "canonical_refs", Blocks)]),
    row(CX, "**Downstream agents MUST read these", &[Home::Ignored(INSTRUCTION)]),
    row(CX, "### Reusable Assets", &[to("context", "reusable_assets", List)]),
    row(CX, "### Established Patterns", &[to("context", "patterns", List)]),
    row(CX, "### Integration Points", &[to("context", "integration_points", List)]),
    row(CX, "<deferred>", &[
        id("deferred_idea"),
        to("deferred_idea", "phase", Ref),
        to("deferred_idea", "text", Text),
        to("deferred_idea", "why", Text),
        filled("deferred_idea", "origin", "discussion", "an idea deferred in a phase's discussion"),
    ]),
    row(CX, "footer", &[reported(PROSE)]),
];

const DL: &str = "NN-DISCUSSION-LOG.md";
#[rustfmt::skip]
pub const DISCUSSION_LOG: &[Row] = &[
    row(DL, "file stem", &[id("discussion"), to("discussion", "phase", Ref)]),
    row(DL, "# Phase N: Name - Discussion Log", &[title("discussion")]),
    row(DL, "> **Audit trail only.**", &[Home::Ignored(INSTRUCTION)]),
    row(DL, "**Date:**, **Phase:**, **Areas discussed:**", &[redundant("the directory and CONTEXT.md give them")]),
    row(DL, "## Area", &[
        to("discussion", "entries.area", Blocks),
        to("discussion", "entries.options", Blocks),
        to("discussion", "entries.chosen", Blocks),
        to("discussion", "entries.notes", Blocks),
        filled("discussion", "entries.mode", "interactive", "the only shape a discussion log has"),
    ]),
    row(DL, "| Description |", &[reported("no member of a discussion entry holds it")]),
    row(DL, "## Claude's Discretion", &[redundant(CONTEXT_MD)]),
    row(DL, "## Deferred Ideas", &[redundant(CONTEXT_MD)]),
    row(DL, "footer", &[reported(PROSE)]),
];

const RS: &str = "NN-RESEARCH.md";
#[rustfmt::skip]
pub const RESEARCH: &[Row] = &[
    row(RS, "file stem", &[id("research"), to("research", "phase", Ref)]),
    row(RS, "# Phase N: Name - Research", &[title("research")]),
    row(RS, "**Researched:**", &[to("research", "researched", Text)]),
    row(RS, "**Domain:**", &[to("research", "domain", Text)]),
    row(RS, "**Confidence:**", &[to("research", "confidence", Word)]),
    row(RS, "### Locked Decisions", &[to("research", "user_constraints", Ref)]),
    row(RS, "<user_constraints> ### Claude's Discretion", &[redundant(CONTEXT_MD)]),
    row(RS, "<user_constraints> ### Deferred Ideas", &[redundant(CONTEXT_MD)]),
    row(RS, "<architectural_responsibility_map>", &[to("research", "responsibility_map", Blocks)]),
    row(RS, "| Secondary Tier | Rationale |", &[reported("no member of a responsibility map holds it")]),
    row(RS, "## Summary", &[to("research", "summary", Text)]),
    row(RS, "**Primary recommendation:**", &[
        id("finding"), to("finding", "research", Ref), to("finding", "text", Text), to("finding", "confidence", Word),
    ]),
    row(RS, "## Standard Stack", &[
        id("stack_choice"),
        title("stack_choice"),
        to("stack_choice", "research", Ref),
        to("stack_choice", "name", Text),
        to("stack_choice", "version", Text),
        to("stack_choice", "purpose", Text),
        to("stack_choice", "tier", Word),
    ]),
    row(RS, "| Why Standard |", &[reported("no field of a stack choice holds it")]),
    row(RS, "### Pattern N: Name", &[
        id("pattern"),
        to("pattern", "research", Ref),
        to("pattern", "name", Text),
        to("pattern", "what", Text),
        to("pattern", "when", Text),
        to("pattern", "example", Text),
    ]),
    row(RS, "### Anti-Patterns to Avoid", &[
        id("pattern"),
        to("pattern", "research", Ref),
        to("pattern", "name", Text),
        to("pattern", "what", Text),
        to("pattern", "anti", Word),
    ]),
    row(RS, "## Don't Hand-Roll", &[
        id("dont_hand_roll"),
        to("dont_hand_roll", "research", Ref),
        to("dont_hand_roll", "problem", Text),
        to("dont_hand_roll", "instead", Text),
        to("dont_hand_roll", "why", Text),
    ]),
    row(RS, "| Don't Build |", &[reported("no field of a don't-hand-roll holds it")]),
    row(RS, "## Common Pitfalls", &[
        id("pitfall"),
        to("pitfall", "research", Ref),
        to("pitfall", "name", Text),
        to("pitfall", "goes_wrong", Text),
        to("pitfall", "why", Text),
        to("pitfall", "avoid", Text),
        to("pitfall", "warning_signs", Text),
    ]),
    row(RS, "## Open Questions", &[
        id("open_question"),
        to("open_question", "research", Ref),
        to("open_question", "question", Text),
        to("open_question", "known", Text),
        to("open_question", "unclear", Text),
        to("open_question", "recommendation", Text),
        to("open_question", "resolved_by", Ref),
    ]),
    row(RS, "## Sources", &[
        id("source"),
        to("source", "research", Ref),
        to("source", "url", Text),
        to("source", "tier", Word),
        to("source", "checked", Text),
    ]),
    row(RS, "package legitimacy", &[reported(LATER)]),
    row(RS, "footer", &[reported(PROSE)]),
];

const VF: &str = "NN-VERIFICATION.md";
#[rustfmt::skip]
pub const VERIFICATION: &[Row] = &[
    row(VF, "file stem", &[id("verification"), to("verification", "phase", Ref)]),
    row(VF, "status", &[computed("verification", "status")]),
    row(VF, "score", &[computed("verification", "score")]),
    row(VF, "verified", &[computed("verification", "verified")]),
    row(VF, "covered_files", &[computed("verification", "covered_files")]),
    row(VF, "covered_digest", &[computed("verification", "covered_digest")]),
    row(VF, "re_verification", &[computed("verification", "re_verification")]),
    row(VF, "anti_patterns", &[computed("verification", "anti_patterns")]),
    row(VF, "scan_partial", &[computed("verification", "scan_partial")]),
    row(VF, "behavior_unverified", &[reported("no field of a verification holds it")]),
    row(VF, "### Observable Truths", &[reported(VERDICT)]),
    row(VF, "### Required Artifacts", &[reported(VERDICT)]),
    row(VF, "### Key Link Verification", &[reported(VERDICT)]),
    row(VF, "### Requirements Coverage", &[redundant("the summaries' requirements-completed give it")]),
    row(VF, "## Gaps Summary", &[reported(LATER)]),
    row(VF, "No gaps found", &[Home::Ignored(EMPTY)]),
    row(VF, "## Human Verification Required", &[reported(LATER)]),
    row(VF, "override", &[reported(
        "a person's value: the host refuses an agent's write of a human field; write it in your editor",
    )]),
];

const UA: &str = "NN-UAT.md";
#[rustfmt::skip]
pub const UAT: &[Row] = &[
    row(UA, "file stem", &[id("uat_session"), to("uat_session", "phase", Ref)]),
    row(UA, "source", &[to("uat_session", "source", List)]),
    row(UA, "started", &[to("uat_session", "started", Text)]),
    row(UA, "status", &[computed("uat_session", "status")]),
    row(UA, "updated", &[reported("the session's last edit: no field holds it")]),
    row(UA, "## Current Test", &[reported("the session's position, a runtime state")]),
    row(UA, "## Summary", &[computed("uat_session", "totals")]),
    row(UA, "### N. Title", &[
        id("uat_test"),
        title("uat_test"),
        to("uat_test", "session", Ref),
        to("uat_test", "expected", Text),
        to("uat_test", "result", Word),
        to("uat_test", "reported", Text),
        to("uat_test", "severity", Word),
        to("uat_test", "reason", Text),
        to("uat_test", "blocked_by", Text),
    ]),
    row(UA, "## Gaps", &[
        id("uat_gap"),
        to("uat_gap", "test", Ref),
        to("uat_gap", "truth", Text),
        to("uat_gap", "reason", Text),
        to("uat_gap", "severity", Word),
        to("uat_gap", "root_cause", Text),
        to("uat_gap", "artifacts", List),
        to("uat_gap", "missing", List),
    ]),
    row(UA, "## Gaps status", &[computed("uat_gap", "status")]),
    row(UA, "## Gaps debug_session", &[reported("a gap's debug session is @gsd/ops's `uat_gap.debug`")]),
    row(UA, "## Deferred Follow-Ups", &[reported("uat_session.follow_ups is the harness field set_uat_result writes")]),
];

const QK: &str = "quick/*";
#[rustfmt::skip]
pub const QUICK: &[Row] = &[
    row(QK, "PLAN quick_id", &[id("plan")]),
    row(QK, "PLAN slug", &[to("plan", "slug", Text)]),
    row(QK, "PLAN title", &[title("plan")]),
    row(QK, "SUMMARY quick_id, slug", &[redundant("the quick plan gives them")]),
    row(QK, "SUMMARY subsystem", &[filled("summary", "subsystem", "quick", "a quick summary has no subsystem")]),
    row(QK, "SUMMARY requirements-completed", &[filled(
        "summary",
        "requirements_completed",
        "[]",
        "a quick summary lists no requirements",
    )]),
    row(QK, "CONTEXT", &[reported(LATER)]),
    row(QK, "RESEARCH", &[reported(LATER)]),
];
pub const QUICK_PLAN: &[Row] = plan_rows!(QK, "PLAN ");
pub const QUICK_SUMMARY: &[Row] = summary_rows!(QK, "SUMMARY ");

const SC: &str = "SECURITY.md";
#[rustfmt::skip]
pub const SECURITY: &[Row] = &[
    row(SC, "phase, slug", &[id("security_audit"), to("security_audit", "phase", Ref)]),
    row(SC, "asvs_level", &[to("security_audit", "asvs_level", Text)]),
    row(SC, "created", &[to("security_audit", "created", Text)]),
    row(SC, "status", &[computed("security_audit", "status")]),
    row(SC, "threats_open", &[computed("security_audit", "threats_open")]),
    row(SC, "## Trust Boundaries", &[to("trust_boundary", "data_crossing", Text)]),
    row(SC, "## Threat Register", &[to("threat", "status", Word)]),
    row(SC, "## Threat Register Mitigation", &[redundant("the plan's Mitigation Plan gives it; a difference is reported")]),
    row(SC, "## Accepted Risks Log", &[
        id("accepted_risk"),
        to("accepted_risk", "audit", Ref),
        to("accepted_risk", "threat", Ref),
        to("accepted_risk", "rationale", Text),
        to("accepted_risk", "accepted_by", Text),
        to("accepted_risk", "at", Text),
    ]),
    row(SC, "## Security Audit <date>", &[
        id("audit_run"),
        to("audit_run", "audit", Ref),
        filled("audit_run", "at", "<date>T00:00:00Z", "the heading gives a date only; the field is a timestamp"),
        to("audit_run", "total", Text),
        to("audit_run", "closed", Text),
        to("audit_run", "open", Text),
    ]),
    row(SC, "## Sign-Off", &[reported(PROSE)]),
    row(SC, "**Approval:**", &[reported(PROSE)]),
    row(SC, "status legend", &[reported(PROSE)]),
];
