//! The judged rules (K7): one table, read by `declare` and by `record_verdict`. Each rule names the agent or prompt
//! that judges it, the outcomes a verdict may record, and what it reads of each target (`spec/rules.md`, "Judged
//! rules' inputs").

use specforge_extension_sdk::prelude::*;

use crate::codes::{CODES, Severity};

/// What a rule reads of the entities of one kind.
pub struct Target {
    pub kind: &'static str,
    /// `(name, selector)`, the named inputs of the verdict.
    pub inputs: &'static [(&'static str, &'static str)],
    /// An entity that writes any of these fields is not judged.
    pub unless_set: &'static [&'static str],
}

/// One judged rule.
pub struct Judged {
    /// The local code: `W311`.
    pub code: &'static str,
    /// The diagnostic's message; the host adds `: no verdict`, `: stale …` or the verdict's outcome.
    pub message: &'static str,
    /// An agent of this extension, or `prompt:<id>`.
    pub judge: &'static str,
    pub outcomes: &'static [&'static str],
    pub passing: &'static [&'static str],
    pub targets: &'static [Target],
    /// `kind.field`s set from the verdict's `values`.
    pub values: &'static [&'static str],
    /// `kind.field`s that hold the verdict's outcome.
    pub outcome_fields: &'static [&'static str],
}

const PLAN_CHECK_INPUTS: &[(&str, &str)] = &[
    ("plan", "self"),
    (
        "body",
        "self<task|checkpoint|truth|artifact|key_link|prohibition|user_setup.plan",
    ),
    ("phase", "self.phase"),
    ("requirements", "self.requirements"),
    ("decisions", "self.phase<decision.phase"),
    ("all_decisions", "decision"),
    ("context", "self.phase<context|deferred_idea.phase"),
    ("research", "self.phase<research.phase"),
    (
        "research_items",
        "self.phase<research.phase<finding|stack_choice|pattern|pitfall|dont_hand_roll|open_question.research",
    ),
    ("siblings", "self.phase<plan.phase"),
    ("instructions", "file(\"CLAUDE.md\", \"AGENTS.md\")"),
];

const PLAN_CHECK: &[Target] = &[Target {
    kind: "plan",
    inputs: PLAN_CHECK_INPUTS,
    unless_set: &[],
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
];

/// The judged rule with this local code.
pub fn get(code: &str) -> Option<&'static Judged> {
    JUDGED.iter().find(|j| j.code == code)
}

fn severity(code: &str) -> ValidationSeverity {
    match CODES.iter().find(|k| k.id == code).map(|k| k.severity) {
        Some(Severity::Error) => ValidationSeverity::Error,
        Some(Severity::Info) => ValidationSeverity::Info,
        _ => ValidationSeverity::Warning,
    }
}

pub fn declare(c: &mut ContributionsBuilder) {
    for j in JUDGED {
        c.rule(j.code, |r| {
            r.severity(severity(j.code))
                .message_template(j.message)
                .judged(j.judge, |b| {
                    b.outcomes(j.outcomes).passing(j.passing);
                    for t in j.targets {
                        b.target(t.kind, |tb| {
                            for (name, selector) in t.inputs {
                                tb.input(name, selector);
                            }
                            if !t.unless_set.is_empty() {
                                tb.unless_set(t.unless_set);
                            }
                        });
                    }
                    if !j.values.is_empty() {
                        b.values(j.values);
                    }
                    if !j.outcome_fields.is_empty() {
                        b.outcome_fields(j.outcome_fields);
                    }
                });
        });
    }
}
