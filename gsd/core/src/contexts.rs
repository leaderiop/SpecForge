//! The context routes `@gsd/core` declares (ADR 0356): for each role it spawns, what the role reads of the entity it
//! is spawned for, in order, core sections first and never cut, enrich sections served whole while the budget lasts.
//! Twelve are bound to their agent, whose installed body gains a `## Context` section naming the route; two,
//! `phase-context` and `quick-context`, are read by no agent: they bound the context a research, a spec or a
//! validation is written against (`spec/rules.md`, "Context routes"): a research, a spec or a validation records the
//! pack in its `context_pack`, and the host reports the record `gsd/W406` once that pack moves.
//!
//! gsd-core modules this file ports: gate-context-drift.cjs.

use specforge_extension_sdk::prelude::*;

use crate::judged::{INTEGRATION_INPUTS, PLAN_CHECK_INPUTS};

/// What every phase researcher reads first: the phase, its requirements, its context and the decisions it settled.
pub(crate) const PHASE_CORE: &[(&str, &str)] = &[
    ("phase", "self"),
    ("requirements", "self.requirements"),
    ("context", "self<context|deferred_idea.phase"),
    ("decisions", "self<decision[origin=context].phase"),
];

/// The findings, choices, patterns and the like a research holds.
pub(crate) const RESEARCH_ITEMS: &str =
    "research<finding|stack_choice|pattern|pitfall|dont_hand_roll|open_question.research";

/// The agent instructions of the project.
pub(crate) const INSTRUCTIONS: &str = "file(\"CLAUDE.md\", \"AGENTS.md\")";

/// A record a route keeps of its pack: the route, the `kind.field` it writes, the chain from that kind to the route's
/// subject, and what the stale record suggests.
pub struct Record {
    pub route: &'static str,
    pub field: &'static str,
    pub subject: &'static str,
    pub suggestion: &'static str,
}

/// Every context field `@gsd/core` declares, by the route that records it. A stale record raises the route's code.
pub const RECORDS: &[Record] = &[
    Record {
        route: "executor",
        field: "plan.context_pack",
        subject: "self",
        suggestion: "read the pack again (`specforge context {route} {subject}`), then run `specforge gsd start-plan \
                     {entity}`; the continuation does",
    },
    Record {
        route: "phase-context",
        field: "research.context_pack",
        subject: "self.phase",
        suggestion: "regenerate the research of {subject} (/gsd-plan-phase --research), then run `specforge gsd \
                     record-research {entity}`; do not hand-inject the newer decisions into a prompt",
    },
    Record {
        route: "phase-context",
        field: "phase_spec.context_pack",
        subject: "self.phase",
        suggestion: "regenerate or reconcile the spec against the current decisions, then run `specforge gsd \
                     record-spec {entity}`; do not hand-inject the newer decisions into a prompt",
    },
    Record {
        route: "phase-context",
        field: "validation.context_pack",
        subject: "self.phase",
        suggestion: "regenerate or reconcile the validation against the current decisions, then run `specforge gsd \
                     record-validation {entity}`; do not hand-inject the newer decisions into a prompt",
    },
    Record {
        route: "quick-context",
        field: "research.context_pack",
        subject: "self.plan",
        suggestion: "regenerate the research of quick task {subject} (/gsd-quick --research), then run `specforge \
                     gsd record-research {entity}`; do not hand-inject the newer decisions into a prompt",
    },
];

/// The code a stale record of each recording route raises.
const STALE: &[(&str, &str)] = &[
    ("executor", "W524"),
    ("phase-context", "W406"),
    ("quick-context", "W406"),
];

/// A route's sections, each `(name, selector, core)`.
type Sections<'a> = &'a [(&'a str, &'a str, bool)];

/// Declare `name` over `subject`, bound to `agent` when it names one, at `budget`, with `sections` in order, then what
/// `more` adds (its settings); its [`RECORDS`] and stale code when it records.
fn route(
    c: &mut ContributionsBuilder,
    name: &str,
    subject: &str,
    agent: Option<&str>,
    budget: u32,
    sections: Sections<'_>,
    more: impl FnOnce(&mut ContextBuilder),
) {
    c.context(name, |r| {
        r.subject(subject).budget(budget);
        if let Some(agent) = agent {
            r.agent(agent);
        }
        for (section, selector, core) in sections {
            if *core {
                r.core(section, selector);
            } else {
                r.enrich(section, selector);
            }
        }
        more(r);
        for record in RECORDS.iter().filter(|record| record.route == name) {
            r.record_suggesting(record.field, record.subject, record.suggestion);
        }
        if let Some((_, code)) = STALE.iter().find(|(route, _)| *route == name) {
            r.stale(code);
        }
    });
}

/// `rows`, every one core.
fn core<'a>(rows: &'a [(&'a str, &'a str)]) -> Vec<(&'a str, &'a str, bool)> {
    rows.iter()
        .map(|(name, selector)| (*name, *selector, true))
        .collect()
}

/// [`PHASE_CORE`], then the phase's milestone and its sibling phases, enrich.
fn researcher() -> Vec<(&'static str, &'static str, bool)> {
    let mut sections = core(PHASE_CORE);
    sections.extend([
        ("milestone", "self.milestone", false),
        ("siblings", "self.milestone<phase.milestone", false),
    ]);
    sections
}

const RESEARCH_ITEMS_OF_PHASE: &str = "self<research.phase<finding|stack_choice|pattern|pitfall|dont_hand_roll|open_question.research";

/// Declare every route.
pub(crate) fn declare(c: &mut ContributionsBuilder) {
    route(
        c,
        "executor",
        "plan",
        Some("gsd-executor"),
        6000,
        &[
            ("plan", "self", true),
            (
                "body",
                "self<task|checkpoint|truth|artifact|key_link|prohibition|user_setup.plan",
                true,
            ),
            ("phase", "self.phase", true),
            ("requirements", "self.requirements", true),
            ("decisions", "self.decisions", true),
            (
                "context",
                "self.phase<context|decision[origin=context]|deferred_idea.phase",
                false,
            ),
            (
                "quick",
                "self<context|decision[origin=context]|research.plan",
                false,
            ),
            ("prior", "self.depends_on<summary.plan", false),
            (
                "research",
                "self.phase<research.phase<pitfall|pattern|dont_hand_roll.research",
                false,
            ),
            ("project", "project", false),
            ("instructions", INSTRUCTIONS, false),
        ],
        |r| {
            r.settings(&[
                "workflow.auto_advance",
                "workflow.human_verify_mode",
                "workflow.tdd_mode",
                "git.allow_default_branch_commits",
            ]);
        },
    );
    route(
        c,
        "plan-checker",
        "plan",
        Some("gsd-plan-checker"),
        8000,
        &core(PLAN_CHECK_INPUTS),
        |_| {},
    );
    route(
        c,
        "integration-checker",
        "milestone_audit",
        Some("gsd-integration-checker"),
        8000,
        &core(INTEGRATION_INPUTS),
        |_| {},
    );
    route(
        c,
        "planner",
        "phase",
        Some("gsd-planner"),
        8000,
        &[
            ("phase", "self", true),
            ("requirements", "self.requirements", true),
            ("context", "self<context|deferred_idea.phase", true),
            ("decisions", "self<decision[origin=context].phase", true),
            ("research", "self<research.phase", false),
            ("research_items", RESEARCH_ITEMS_OF_PHASE, false),
            ("plans", "self<plan.phase", false),
            ("milestone", "self.milestone", false),
            (
                "gaps",
                "self<verification.phase<verify_gap.verification",
                false,
            ),
            (
                "uat_gaps",
                "self<uat_session.phase<uat_test.session<uat_gap.test",
                false,
            ),
            ("prior", "self.depends_on<plan.phase<summary.plan", false),
            ("project_decisions", "decision[origin=project]", false),
            ("project", "project", false),
            ("instructions", INSTRUCTIONS, false),
        ],
        |r| {
            r.settings(&["workflow.tdd_mode", "workflow.human_verify_mode"]);
        },
    );
    route(
        c,
        "verifier",
        "phase",
        Some("gsd-verifier"),
        8000,
        &[
            ("phase", "self", true),
            ("requirements", "self.requirements", true),
            (
                "must_haves",
                "self<plan.phase<truth|artifact|key_link.plan",
                true,
            ),
            ("plans", "self<plan.phase", true),
            ("summaries", "self<plan.phase<summary.plan", true),
            ("files", "file(self<plan.phase.files_modified)", false),
            (
                "context",
                "self<context|decision[origin=context]|deferred_idea.phase",
                false,
            ),
            ("pitfalls", "self<research.phase<pitfall.research", false),
        ],
        |_| {},
    );
    let mut phase_researcher = researcher();
    phase_researcher.extend([
        ("project", "project", false),
        ("instructions", INSTRUCTIONS, false),
    ]);
    route(
        c,
        "phase-researcher",
        "phase",
        Some("gsd-phase-researcher"),
        6000,
        &phase_researcher,
        |_| {},
    );
    for (name, agent) in [
        ("advisor-researcher", "gsd-advisor-researcher"),
        ("assumptions-analyzer", "gsd-assumptions-analyzer"),
    ] {
        route(c, name, "phase", Some(agent), 6000, &researcher(), |_| {});
    }
    route(
        c,
        "nyquist-auditor",
        "phase",
        Some("gsd-nyquist-auditor"),
        8000,
        &[
            ("phase", "self", true),
            ("plans", "self<plan.phase", true),
            ("tasks", "self<plan.phase<task.plan", true),
            ("validation", "self<validation.phase", true),
            (
                "rows",
                "self<validation.phase<validation_row.validation",
                true,
            ),
            ("requirements", "self.requirements", false),
            ("research", "self<research.phase", false),
            ("summaries", "self<plan.phase<summary.plan", false),
        ],
        |_| {},
    );
    route(
        c,
        "project-researcher",
        "project",
        Some("gsd-project-researcher"),
        8000,
        &[
            ("project", "self", true),
            ("requirements", "requirement", true),
            ("decisions", "decision[origin=project]", true),
            ("milestones", "milestone", false),
        ],
        |_| {},
    );
    route(
        c,
        "research-synthesizer",
        "project",
        Some("gsd-research-synthesizer"),
        8000,
        &[
            ("project", "self", true),
            ("research", "research", true),
            ("research_items", RESEARCH_ITEMS, true),
            ("requirements", "requirement", false),
        ],
        |_| {},
    );
    route(
        c,
        "roadmapper",
        "project",
        Some("gsd-roadmapper"),
        8000,
        &[
            ("project", "self", true),
            ("requirements", "requirement", true),
            ("decisions", "decision[origin=project]", true),
            ("research", "research", false),
            ("research_items", RESEARCH_ITEMS, false),
            ("milestones", "milestone", false),
        ],
        |_| {},
    );
    route(
        c,
        "phase-context",
        "phase",
        None,
        50000,
        &[
            ("context", "self<context.phase", true),
            ("decisions", "self<decision[origin=context].phase", true),
            ("deferred", "self<deferred_idea.phase", true),
        ],
        |_| {},
    );
    route(
        c,
        "quick-context",
        "plan",
        None,
        50000,
        &[
            ("context", "self<context.plan", true),
            ("decisions", "self<decision[origin=context].plan", true),
        ],
        |_| {},
    );
}
