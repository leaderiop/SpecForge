//! The judged rules: one table, read by `declare` and by `record_verdict`. Each rule names the agent or prompt
//! that judges it, the outcomes a verdict may record, and what it reads of each target (`spec/rules.md`, "Judged
//! rules' inputs").
//!
//! Also ports gsd-core bin/lib modules: package-legitimacy.cjs.

use specforge_extension_sdk::prelude::*;

use gsd_kit::judged::{Judged, Owed, Target};

use crate::codes::{CODES, Severity};

/// What the plan checks read of a plan; the `gsd/plan-checker` route serves the same sections. Of the decisions: its
/// phase's, a quick plan's own, the project's, those that resolve a question of its phase's research, and those a task
/// of the plan names as settling its one-way door, never every decision of every phase.
pub(crate) const PLAN_CHECK_INPUTS: &[(&str, &str)] = &[
    ("plan", "self"),
    (
        "body",
        "self<task|checkpoint|truth|artifact|key_link|prohibition|user_setup.plan",
    ),
    ("phase", "self.phase"),
    ("requirements", "self.requirements"),
    ("phase_decisions", "self.phase<decision.phase"),
    ("quick_decisions", "self<decision.plan"),
    ("project_decisions", "decision[origin=project]"),
    (
        "resolved_decisions",
        "self.phase<research.phase<open_question.research.resolved_by",
    ),
    (
        "settled_decisions",
        "self<task.plan.reversibility.settled_by",
    ),
    (
        "context",
        "self.phase<context|deferred_idea[origin=discussion].phase",
    ),
    ("research", "self.phase<research.phase"),
    (
        "research_items",
        "self.phase<research.phase<finding|stack_choice|pattern|pitfall|dont_hand_roll|open_question.research",
    ),
    ("siblings", "self.phase<plan.phase"),
    ("instructions", "file(\"CLAUDE.md\", \"AGENTS.md\")"),
];

/// What the verifier's verdict on a truth reads: the truth, its plan, its phase and the tests that name it. The
/// stale-truths pass's delta reads its entity inputs.
pub(crate) const TRUTH_INPUTS: &[(&str, &str)] = &[
    ("truth", "self"),
    ("plan", "self.plan"),
    ("phase", "self.plan.phase"),
    ("tests", "test(self<test.names)"),
];

/// What E518 reads of a milestone audit; the `gsd/integration-checker` route serves the same sections.
pub(crate) const INTEGRATION_INPUTS: &[(&str, &str)] = &[
    ("audit", "self"),
    ("phases", "self.milestone<phase.milestone"),
    (
        "verifications",
        "self.milestone<phase.milestone<verification.phase",
    ),
    (
        "requirements",
        "self.milestone<phase.milestone.requirements",
    ),
];

// A plan check is owed while its plan is open: it judges a plan before it runs, and an executed plan's check is history
// (ADR 0377). A must-have is owed once its plan is complete: the verifier judges what was built.
const PLAN_CHECK: &[Target] = &[Target {
    kind: "plan",
    inputs: PLAN_CHECK_INPUTS,
    unless_set: &[],
    judged_while: Some(Owed::Open),
}];

// A truth's verdict reads the files and tests it cites (the host digests every verdict's `cited`, ADR 0377), so an
// edit stales only the truths that cite the file; an artifact and a key link read the files they name (ADR 0304). The
// plan checks read none: they judge the plan before its files exist, and a code edit must not stale an in-flight check.
const E501: &[Target] = &[
    Target {
        kind: "truth",
        inputs: TRUTH_INPUTS,
        unless_set: &[],
        judged_while: Some(Owed::Complete),
    },
    Target {
        kind: "artifact",
        inputs: &[
            ("artifact", "self"),
            ("plan", "self.plan"),
            ("file", "file(self.path)"),
        ],
        unless_set: &[],
        judged_while: Some(Owed::Complete),
    },
    // A key link with a `pattern` is not judged: a pass searches the pattern (`passes::scan`).
    Target {
        kind: "key_link",
        inputs: &[
            ("link", "self"),
            ("plan", "self.plan"),
            ("from", "file(self.from)"),
            ("to", "file(self.to)"),
        ],
        unless_set: &["pattern"],
        judged_while: Some(Owed::Complete),
    },
];

// A prohibition that names its enforcing `test` is E319's and the test's; one that names none is judged, upstream's
// fail-closed rule for a judgment-tier item. A model's verdict is not authoritative, so its rule is a warning. A
// prohibition judges its plan's whole change ("no code of the plan does X"), so its verdict reads the plan's files and,
// as a truth's, what it cites (`record_verdict` refuses one that cites no code): an edit anywhere in the plan's change,
// or to another file the verifier read, stales it.
const W525: &[Target] = &[Target {
    kind: "prohibition",
    inputs: &[
        ("prohibition", "self"),
        ("plan", "self.plan"),
        ("phase", "self.plan.phase"),
        ("files", "file(self.plan.files_modified)"),
    ],
    unless_set: &["test"],
    judged_while: Some(Owed::Complete),
}];

const fn plan_check(code: &'static str, message: &'static str) -> Judged {
    Judged {
        code,
        message,
        judge: "gsd-plan-checker",
        outcomes: &["passed", "issues"],
        passing: &["passed"],
        targets: PLAN_CHECK,
        values: &[],
        outcome_fields: &[],
    }
}

pub const JUDGED: &[Judged] = &[
    plan_check(
        "W311",
        "plan '{id}' is not checked against its phase goal, scope and settled decisions",
    ),
    plan_check("W313", "plan '{id}' is not checked for wired key links"),
    plan_check(
        "W315",
        "plan '{id}' is not checked for matching data contracts",
    ),
    plan_check(
        "W316",
        "plan '{id}' is not checked against the agent instructions and mapped patterns",
    ),
    Judged {
        code: "E501",
        message: "{kind} '{id}' has no fresh verifier verdict",
        judge: "gsd-verifier",
        outcomes: &[
            "verified",
            "verified_coincidental",
            "failed",
            "uncertain",
            "present_behavior_unverified",
        ],
        passing: &["verified", "verified_coincidental"],
        targets: E501,
        values: &[],
        outcome_fields: &[],
    },
    Judged {
        code: "W525",
        message: "{kind} '{id}' has no fresh verifier verdict",
        judge: "gsd-verifier",
        outcomes: &["held", "violated", "unverified"],
        passing: &["held"],
        targets: W525,
        values: &[],
        outcome_fields: &[],
    },
    Judged {
        code: "E518",
        message: "milestone audit '{id}' has no fresh integration verdict",
        judge: "gsd-integration-checker",
        outcomes: &["passed", "issues"],
        passing: &["passed"],
        targets: &[Target {
            kind: "milestone_audit",
            inputs: INTEGRATION_INPUTS,
            unless_set: &[],
            judged_while: None,
        }],
        values: &[],
        outcome_fields: &["milestone_audit.integration"],
    },
    Judged {
        code: "E202",
        message: "phase spec '{id}' is not judged against its ambiguity gates",
        judge: "prompt:spec_phase",
        outcomes: &["passed", "failed"],
        passing: &["passed"],
        targets: &[Target {
            kind: "phase_spec",
            inputs: &[
                ("spec", "self"),
                ("items", "self<locked_req|interview.spec"),
                ("phase", "self.phase"),
            ],
            unless_set: &[],
            judged_while: None,
        }],
        values: &["phase_spec.scores"],
        outcome_fields: &[],
    },
    Judged {
        code: "W203",
        message: "locked requirement '{id}' is not judged for a falsifiable acceptance",
        judge: "prompt:spec_phase",
        outcomes: &["passed", "failed"],
        passing: &["passed"],
        targets: &[Target {
            kind: "locked_req",
            inputs: &[("req", "self"), ("traces", "self.traces")],
            unless_set: &[],
            judged_while: None,
        }],
        values: &[],
        outcome_fields: &[],
    },
    Judged {
        code: "W205",
        message: "package '{id}' has no legitimacy verdict",
        judge: "gsd-phase-researcher",
        outcomes: &["legitimate", "suspicious", "unknown"],
        passing: &["legitimate"],
        targets: &[Target {
            kind: "package_audit",
            inputs: &[("audit", "self")],
            unless_set: &[],
            judged_while: None,
        }],
        values: &[],
        outcome_fields: &["package_audit.legitimacy"],
    },
];

/// The judged rule with this local code.
pub fn get(code: &str) -> Option<&'static Judged> {
    gsd_kit::judged::get(JUDGED, code)
}

fn severity(code: &str) -> ValidationSeverity {
    match CODES.iter().find(|k| k.id == code).map(|k| k.severity) {
        Some(Severity::Error) => ValidationSeverity::Error,
        Some(Severity::Info) => ValidationSeverity::Info,
        _ => ValidationSeverity::Warning,
    }
}

pub fn declare(c: &mut ContributionsBuilder) {
    gsd_kit::judged::declare(c, JUDGED, severity);
}
