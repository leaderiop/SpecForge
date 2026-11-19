//! `plan_phase` (D-r42-1 row 6): the largest workflow. It researches the phase, maps patterns, plans, checks the
//! plans and revises them until the checker passes. Its gate is the entry gate of planning.

use specforge_extension_sdk::prelude::*;

use super::args::Arg;
use super::ns::served;
use super::workflow::{Do, Slash, Spec, Step};

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
        id: "thread",
        title: "Check the plans' order with specforge trace --plan and fix an ok: false result before the check (specforge gsd thread-plan <phase> --format json)",
        does: Do::Command("thread_plan"),
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
    inputs: &[specforge_extension_sdk::Input::SlashCommands],
    tail: Some(tail),
};

/// The sections after the workflow: how this install serves `/gsd-graphify`, then how the `thread` step checks the
/// plans' order.
fn tail(call: &PromptCall<'_>) -> String {
    graph_context(call) + THREADING
}

/// What the `thread` step runs, what fails it and where its result goes (ADR 0378).
const THREADING: &str = "\n## Threading the plans\n\n\
     After the planner writes the phase's plans, and before the plan checker runs, check their order against the \
     graph:\n\n\
     1. Write the answer of `specforge gsd thread-plan <phase> --format json` (the MCP tool \
     `specforge.gsd.thread_plan`) to a file. Its entries are the phase's plans in strict-wave order, each followed by \
     its tasks.\n\
     2. Run `specforge trace --plan <file> --format json`.\n\n\
     A result with `ok: false` is a finding the planner fixes before the check: an `unresolved_entity` gap (E003) is \
     an entry that names no entity, and an `ordering` gap is an entry before a plan or task it depends on, such as a \
     coupling peer that runs later. Fix the plan's `depends_on`, its coupling peer or the order, then run the step \
     again. `missing_plan_entry` gaps are warnings over every testable entity of the project: they do not fail the \
     thread.\n\n\
     Pass the final result to the plan checker as its `**Thread:**` line: its own section reads that line, and \
     without it the checker runs both commands itself. \
     The order of the tasks inside a plan is not checked: a task carries no `depends_on`.\n";

/// How this install serves `/gsd-graphify`: the planner's graph-context gate, read from the served slash commands
/// rather than a PATH probe or a graph-path setting.
fn graph_context(call: &PromptCall<'_>) -> String {
    format!(
        "\n\n## Graph context on this install\n\n- `/gsd-graphify`: {}\n\n\
         Pass this line to the planner as its `**Graph context:**`.\n",
        served(call, "graphify")
    )
}

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
    // autonomous --converge's dispatch carries it to bypass the feature setting (#4600); not in the hint
    Arg::flag("override_gate"),
];

const CONVERGENCE_STEPS: &[Step] = &[
    Step {
        id: "replan",
        title: "Replan with the reviewers' feedback (the prompt plan_phase, with the reviewers' concerns)",
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
    inputs: &[],
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
    inputs: &[],
    tail: None,
};

/// The workflows of this family, in the order they are declared.
pub(super) static SPECS: &[&Spec] = &[&PLAN_PHASE, &PLAN_REVIEW_CONVERGENCE, &ULTRAPLAN_PHASE];
