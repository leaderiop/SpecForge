//! `plan_phase` (D-r42-1 row 6): the largest workflow. It researches the phase, maps patterns, plans, checks the
//! plans and revises them until the checker passes. Its gate is the entry gate of planning.

use specforge_extension_sdk::prelude::*;

use super::args::Arg;
use super::workflow::{self, Do, Slash, Spec, Step};

const ARGS: &[Arg] = &[
    Arg::reference("phase", "phase"),
    Arg::flag("auto"),
    Arg::flag("research"),
    Arg::flag("skip_research"),
    Arg::text("research_phase"),
    Arg::flag("view"),
    Arg::flag("gaps"),
    Arg::flag("skip_verify"),
    Arg::flag("skip_ui"),
    Arg::text("prd"),
    Arg::text("ingest"),
    Arg::one_of("ingest_format", &["auto", "nygard", "madr", "narrative"]),
    Arg::flag("reviews"),
    Arg::flag("text"),
    Arg::flag("bounce"),
    Arg::flag("skip_bounce"),
    Arg::flag("chunked"),
    Arg::flag("tdd"),
    Arg::flag("mvp"),
    Arg::one_of("granularity", &["coarse", "standard", "fine"]),
    Arg::flag("no_tracer"),
    Arg::flag("no_reversibility_gates"),
];

const STEPS: &[Step] = &[
    Step {
        id: "research",
        title: "Research how to implement the phase (gsd-phase-researcher)",
        does: Do::Agent("gsd-phase-researcher"),
    },
    Step {
        id: "record_research",
        title: "Record the research's digest (specforge gsd record-research <research>)",
        does: Do::Command("record_research"),
    },
    Step {
        id: "patterns",
        title: "Map the closest existing patterns (gsd-pattern-mapper)",
        does: Do::Agent("gsd-pattern-mapper"),
    },
    Step {
        id: "plan",
        title: "Create the phase's plans (gsd-planner)",
        does: Do::Agent("gsd-planner"),
    },
    Step {
        id: "check",
        title: "Check the plans against the phase goal (gsd-plan-checker)",
        does: Do::Agent("gsd-plan-checker"),
    },
    Step {
        id: "record_verdict",
        title: "Record the plan checker's verdicts (specforge gsd record-verdict <plan> <code> <outcome> <model> '<basis>')",
        does: Do::Command("record_verdict"),
    },
];

/// Upstream's hint, with the phase required (the subject scopes the gates).
const HINT: &str = "<phase> [--auto] [--research] [--skip-research] [--research-phase <N>] [--view] [--gaps] [--skip-verify] [--skip-ui] [--prd <file>] [--ingest <path-or-glob>] [--ingest-format <auto|nygard|madr|narrative>] [--reviews] [--text] [--bounce] [--skip-bounce] [--chunked] [--tdd] [--mvp] [--granularity <coarse|standard|fine>] [--no-tracer] [--no-reversibility-gates]";

static PLAN_PHASE: Spec = Spec {
    id: "plan_phase",
    title: "Plan a phase",
    description: "Create detailed phase plan (PLAN.md) with verification loop",
    command: "plan-phase",
    args: ARGS,
    subject: Some("phase"),
    gates: &["E201", "W204", "E206", "W406", "E516"],
    steps: STEPS,
    slash: Some(Slash {
        hint: HINT,
        tools: &[
            "read",
            "write",
            "shell",
            "search",
            "agent",
            "ask_user",
            "web_fetch",
            "mcp:context7/*",
        ],
        effort: Some("max"),
    }),
    tail: None,
};

pub fn declare(c: &mut ContributionsBuilder) {
    workflow::declare(c, &PLAN_PHASE);
}
