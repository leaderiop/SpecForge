//! `autonomous` (D-r43-1 row 7, D-r43-17): upstream runs every remaining phase unattended. Here the agent follows the
//! workflow in this conversation: each phase is a prompt step whose own gate refuses a phase that is not ready. The
//! harness that would run it unattended is not part of `@gsd/core`.

use specforge_extension_sdk::prelude::*;

use super::args::Arg;
use super::workflow::{self, Do, Slash, Spec, Step};

const STEPS: &[Step] = &[
    Step {
        id: "discuss",
        title: "Discuss the phase",
        does: Do::Prompt("discuss_phase"),
    },
    Step {
        id: "plan",
        title: "Plan the phase",
        does: Do::Prompt("plan_phase"),
    },
    Step {
        id: "converge",
        title: "Replan until the cross-AI review concerns are resolved (--converge)",
        does: Do::Prompt("plan_review_convergence"),
    },
    Step {
        id: "execute",
        title: "Execute the phase",
        does: Do::Prompt("execute_phase"),
    },
    Step {
        id: "verify",
        title: "Verify the phase's work",
        does: Do::Prompt("verify_work"),
    },
    Step {
        id: "audit",
        title: "Audit the milestone once every phase is done",
        does: Do::Prompt("audit_milestone"),
    },
    Step {
        id: "complete",
        title: "Complete the milestone",
        does: Do::Prompt("complete_milestone"),
    },
    Step {
        id: "cleanup",
        title: "Archive the closed milestone's phases (specforge gsd cleanup)",
        does: Do::Command("cleanup"),
    },
];

/// What this install does not have: the unattended harness, its settings, the code review and the UI gate.
fn unattended(_call: &PromptCall<'_>) -> String {
    "\n\n## What runs here and what does not\n\n\
     This workflow runs in this conversation, one phase after another, through the prompt steps above. Each step keeps \
     its own gate: a phase that is not ready is refused there, and you stop and report it.\n\n\
     - The unattended harness (`specforged`) is not part of `@gsd/core`. Nothing starts this workflow by itself or \
     resumes it after the conversation ends.\n\
     - `claude_orchestration.*` and `external_job.*` are declared settings with no effect here, even when the settings \
     block above shows them set.\n\
     - `/gsd-code-review` and the UI gate (`check ui-plan-gate`) belong to extensions that are not installed: skip \
     the code review and the UI design contract, and say so.\n"
        .to_string()
}

static AUTONOMOUS: Spec = Spec {
    id: "autonomous",
    title: "Run the remaining phases",
    description: "Run all remaining phases autonomously — discuss→plan→execute per phase",
    command: "autonomous",
    args: &[
        Arg::optional_reference("from", "phase"),
        Arg::optional_reference("to", "phase"),
        Arg::optional_reference("only", "phase"),
        Arg::flag("interactive"),
        Arg::flag("converge"),
    ],
    subject: None,
    gates: &[],
    steps: STEPS,
    slash: Some(Slash {
        hint: "[--from N] [--to N] [--only N] [--interactive] [--converge]",
        tools: &["read", "write", "shell", "search", "ask_user", "agent"],
        effort: Some("max"),
    }),
    tail: Some(unattended),
};

pub fn declare(c: &mut ContributionsBuilder) {
    workflow::declare(c, &AUTONOMOUS);
}
