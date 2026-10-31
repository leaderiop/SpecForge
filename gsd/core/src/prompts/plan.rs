//! `plan_phase` (D-r42-1 row 6): the largest workflow. It researches the phase, maps patterns, plans, checks the
//! plans and revises them until the checker passes. Its gate is the entry gate of planning.

use specforge_extension_sdk::prelude::*;

use super::args::Arg;
use super::workflow::{self, Do, Slash, Spec, Step};

const ARGS: &[Arg] = &[
    Arg::subject("phase", "phase"),
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
        title: "Research how to implement the phase",
        does: Do::Agent("gsd-phase-researcher"),
    },
    Step {
        id: "record_research",
        title: "Record the research's digest (specforge gsd record-research <research>)",
        does: Do::Command("record_research"),
    },
    Step {
        id: "patterns",
        title: "Map the closest existing patterns",
        does: Do::Agent("gsd-pattern-mapper"),
    },
    Step {
        id: "plan",
        title: "Create the phase's plans",
        does: Do::Agent("gsd-planner"),
    },
    Step {
        id: "check",
        title: "Check the plans against the phase goal",
        does: Do::Agent("gsd-plan-checker"),
    },
    Step {
        id: "record_verdict",
        title: "Record the plan checker's verdicts (specforge gsd record-verdict <plan> <code> <outcome> <model> '<basis>')",
        does: Do::Command("record_verdict"),
    },
    Step {
        id: "execute",
        title: "Continue with the execution of the phase (the prompt execute_phase)",
        does: Do::Prompt("execute_phase"),
    },
];

/// Upstream's hint.
const HINT: &str = "[phase] [--auto] [--research] [--skip-research] [--research-phase <N>] [--view] [--gaps] [--skip-verify] [--skip-ui] [--prd <file>] [--ingest <path-or-glob>] [--ingest-format <auto|nygard|madr|narrative>] [--reviews] [--text] [--bounce] [--skip-bounce] [--chunked] [--tdd] [--mvp] [--granularity <coarse|standard|fine>] [--no-tracer] [--no-reversibility-gates]";

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
    reads_state: false,
    reads_slash_commands: false,
    tail: None,
};

/// Upstream's order: the reviewer lanes, then `--text`, `--ws`, `--all`, `--max-cycles`.
const CONVERGENCE_ARGS: &[Arg] = &[
    Arg::reference("phase", "phase"),
    Arg::flag("claude"),
    Arg::flag("codex"),
    Arg::flag("coderabbit"),
    Arg::flag("opencode"),
    Arg::flag("qwen"),
    Arg::flag("cursor"),
    Arg::flag("antigravity"),
    Arg::flag("agy"),
    Arg::flag("ollama"),
    Arg::flag("lm_studio"),
    Arg::flag("llama_cpp"),
    Arg::flag("kimi_code"),
    Arg::flag("text"),
    Arg::text("ws"),
    Arg::flag("all"),
    Arg::integer("max_cycles"),
];

const CONVERGENCE_STEPS: &[Step] = &[
    Step {
        id: "replan",
        title: "Replan with the reviewers' feedback (the prompt plan_phase, --reviews)",
        does: Do::Prompt("plan_phase"),
    },
    Step {
        id: "check",
        title: "Check the revised plans against the phase goal",
        does: Do::Agent("gsd-plan-checker"),
    },
];

const CONVERGENCE_HINT: &str = "<phase> [--claude] [--codex] [--coderabbit] [--opencode] [--qwen] [--cursor] [--antigravity] [--agy] [--ollama] [--lm-studio] [--llama-cpp] [--kimi-code] [--text] [--ws <name>] [--all] [--max-cycles N]";

static PLAN_REVIEW_CONVERGENCE: Spec = Spec {
    id: "plan_review_convergence",
    title: "Replan until the reviewers' concerns are resolved",
    description: "Cross-AI plan convergence - replan until review concerns are resolved.",
    command: "plan-review-convergence",
    args: CONVERGENCE_ARGS,
    subject: Some("phase"),
    gates: &["E201", "W204", "E206", "W406", "E516"],
    steps: CONVERGENCE_STEPS,
    slash: Some(Slash {
        hint: CONVERGENCE_HINT,
        tools: &[
            "read", "write", "shell", "search", "agent", "skill", "ask_user",
        ],
        effort: None,
    }),
    reads_state: false,
    reads_slash_commands: false,
    tail: None,
};

const ULTRAPLAN_STEPS: &[Step] = &[
    Step {
        id: "check",
        title: "Check the imported plans against the phase goal",
        does: Do::Agent("gsd-plan-checker"),
    },
    Step {
        id: "record_verdict",
        title: "Record the plan checker's verdicts (specforge gsd record-verdict <plan> <code> <outcome> <model> '<basis>')",
        does: Do::Command("record_verdict"),
    },
];

static ULTRAPLAN_PHASE: Spec = Spec {
    id: "ultraplan_phase",
    title: "Offload planning to Claude Code's ultraplan cloud",
    description: "[BETA] Offload plan phase to Claude Code's ultraplan cloud; review in browser and import back.",
    command: "ultraplan-phase",
    args: &[Arg::reference("phase", "phase")],
    subject: Some("phase"),
    gates: &["E201", "W204", "E206", "W406", "E516"],
    steps: ULTRAPLAN_STEPS,
    slash: Some(Slash {
        hint: "<phase>",
        tools: &["read", "shell", "search"],
        effort: None,
    }),
    reads_state: false,
    reads_slash_commands: false,
    tail: None,
};

pub fn declare(c: &mut ContributionsBuilder) {
    workflow::declare(c, &PLAN_PHASE);
    workflow::declare(c, &PLAN_REVIEW_CONVERGENCE);
    workflow::declare(c, &ULTRAPLAN_PHASE);
}
