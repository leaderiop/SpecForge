//! `execute_phase` and `execute_plan` (D-r42-1 rows 9-10, D-r42-16): the execution loop. The phase prompt runs the
//! waves and spawns an executor per plan; `execute_plan` is upstream's execute-plan workflow, a prompt of its own with
//! no slash command (upstream has no `/gsd-execute-plan`).

use specforge_extension_sdk::prelude::*;

use super::args::Arg;
use super::workflow::{self, Do, Slash, Spec, Step};

const GATES: &[&str] = &["W318", "E516"];

const PHASE_ARGS: &[Arg] = &[
    Arg::reference("phase", "phase"),
    Arg::integer("wave"),
    Arg::flag("gaps_only"),
    Arg::flag("interactive"),
    Arg::flag("tdd"),
];

const PHASE_STEPS: &[Step] = &[
    Step {
        id: "execute",
        title: "Execute each plan of the wave (the prompt execute_plan)",
        does: Do::Prompt("execute_plan"),
    },
    Step {
        id: "verify",
        title: "Verify the phase goal",
        does: Do::Agent("gsd-verifier"),
    },
    Step {
        id: "record_verdict",
        title: "Record the verifier's verdicts (specforge gsd record-verdict <plan> <code> <outcome> <model> '<basis>')",
        does: Do::Command("record_verdict"),
    },
    Step {
        id: "record_window",
        title: "Record a window of the phase (specforge gsd record-window <kind> <phase> <description> ...)",
        does: Do::Command("record_window"),
    },
    Step {
        id: "next",
        title: "Continue with the verification of the built work (the prompt verify_work)",
        does: Do::Prompt("verify_work"),
    },
];

const PLAN_STEPS: &[Step] = &[
    Step {
        id: "start",
        title: "Open the plan and note the clock (specforge gsd start-plan <plan>)",
        does: Do::Command("start_plan"),
    },
    Step {
        id: "execute",
        title: "Execute the plan's tasks",
        does: Do::Agent("gsd-executor"),
    },
    Step {
        id: "checkpoint",
        title: "Ask the person to run `specforge gsd answer-checkpoint <checkpoint> ...` in their terminal (an agent's call is refused: it records what a person says)",
        does: Do::Command("answer_checkpoint"),
    },
    Step {
        id: "setup",
        title: "Ask the person to run `specforge gsd confirm-setup <user_setup> --by <who>` in their terminal (an agent's call is refused: it records what a person says)",
        does: Do::Command("confirm_setup"),
    },
    Step {
        id: "blocker",
        title: "Raise a blocker (specforge gsd raise-blocker <text>)",
        does: Do::Command("raise_blocker"),
    },
    Step {
        id: "complete",
        title: "Close the plan (specforge gsd complete-plan <summary> --started <time>)",
        does: Do::Command("complete_plan"),
    },
];

/// Worktree isolation is a capability @gsd/core does not have; the workflow's lines set `USE_WORKTREES` to `false`
/// whatever `workflow.use_worktrees` says, so the executors run one after another in this checkout.
fn sequential(_: &PromptCall<'_>) -> String {
    "\n\nGit worktree isolation is not part of @gsd/core: `USE_WORKTREES` is `false` in the shell blocks above whatever \
     `workflow.use_worktrees` says; run the executors one after another in this checkout.\n"
        .to_string()
}

static EXECUTE_PLAN: Spec = Spec {
    id: "execute_plan",
    title: "Execute one plan",
    description: "Execute a phase prompt (PLAN.md) and create the outcome summary (SUMMARY.md)",
    command: "execute-plan",
    args: &[Arg::reference("plan", "plan")],
    subject: Some("plan"),
    gates: GATES,
    steps: PLAN_STEPS,
    slash: None,
    reads_state: false,
    tail: Some(sequential),
};

static EXECUTE_PHASE: Spec = Spec {
    id: "execute_phase",
    title: "Execute a phase",
    description: "SDD phase execution — execute all plans in a phase with dependency-aware wave parallelization",
    command: "execute-phase",
    args: PHASE_ARGS,
    subject: Some("phase"),
    gates: &["W311", "W313", "W315", "W318", "E516"],
    steps: PHASE_STEPS,
    slash: Some(Slash {
        hint: "<phase> [--wave N] [--gaps-only] [--interactive] [--tdd]",
        tools: &[
            "read", "write", "edit", "search", "shell", "agent", "todo", "ask_user",
        ],
        effort: Some("max"),
    }),
    reads_state: false,
    tail: Some(sequential),
};

pub fn declare(c: &mut ContributionsBuilder) {
    workflow::declare(c, &EXECUTE_PLAN);
    workflow::declare(c, &EXECUTE_PHASE);
}
