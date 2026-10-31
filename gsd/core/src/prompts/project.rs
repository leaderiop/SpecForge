//! `new_project` and `new_milestone` (D-r42-1 rows 1-2): the workflows that start a project and a milestone. Both
//! research with the project researcher, synthesize, and hand the roadmap to the roadmapper.

use specforge_extension_sdk::prelude::*;

use super::args::Arg;
use super::workflow::{self, Do, Slash, Spec, Step};

const STEPS: &[Step] = &[
    Step {
        id: "research",
        title: "Research the domain with the project researcher, one agent per dimension",
        does: Do::Agent("gsd-project-researcher"),
    },
    Step {
        id: "synthesize",
        title: "Synthesize the research",
        does: Do::Agent("gsd-research-synthesizer"),
    },
    Step {
        id: "roadmap",
        title: "Create the roadmap",
        does: Do::Agent("gsd-roadmapper"),
    },
    Step {
        id: "record_research",
        title: "Record each research entity's digest (specforge gsd record-research <research>)",
        does: Do::Command("record_research"),
    },
    Step {
        id: "discuss",
        title: "Discuss the first phase before planning it (discuss_phase)",
        does: Do::Prompt("discuss_phase"),
    },
];

const NEW_PROJECT_ARGS: &[Arg] = &[Arg::flag("auto"), Arg::text("idea")];

const NEW_MILESTONE_ARGS: &[Arg] = &[
    Arg::text("name"),
    Arg::text("ws"),
    Arg::flag("reset_phase_numbers"),
];

fn idea(call: &PromptCall<'_>) -> String {
    match call.str("idea").filter(|i| !i.is_empty()) {
        Some(_) => "\n\n`--idea` is the person's own first description of the project: take it as their answer \
                    to the opening question of the questioning step, and go on from there.\n"
            .to_string(),
        None => String::new(),
    }
}

fn name(call: &PromptCall<'_>) -> String {
    match call.str("name").filter(|n| !n.is_empty()) {
        Some(_) => "\n\nThe text after `--name` is the milestone name (upstream's free-text argument); \
                    `--ws` names a workstream and `--reset-phase-numbers` restarts phase numbering at 1.\n"
            .to_string(),
        None => String::new(),
    }
}

static NEW_PROJECT: Spec = Spec {
    id: "new_project",
    title: "Start a new project",
    description: "Initialize a new project with deep context gathering and PROJECT.md",
    command: "new-project",
    args: NEW_PROJECT_ARGS,
    subject: None,
    gates: &[],
    steps: STEPS,
    slash: Some(Slash {
        hint: "[--auto]",
        tools: &["read", "shell", "search", "write", "agent", "ask_user"],
        effort: None,
    }),
    reads_state: false,
    tail: Some(idea),
};

static NEW_MILESTONE: Spec = Spec {
    id: "new_milestone",
    title: "Start a new milestone",
    description: "Start a new milestone cycle — update PROJECT.md and route to requirements",
    command: "new-milestone",
    args: NEW_MILESTONE_ARGS,
    subject: None,
    gates: &[],
    steps: STEPS,
    slash: Some(Slash {
        hint: "[--name \"...\"] [--ws <name>] [--reset-phase-numbers]",
        tools: &["read", "write", "shell", "search", "agent", "ask_user"],
        effort: None,
    }),
    reads_state: false,
    tail: Some(name),
};

pub fn declare(c: &mut ContributionsBuilder) {
    workflow::declare(c, &NEW_PROJECT);
    workflow::declare(c, &NEW_MILESTONE);
}
