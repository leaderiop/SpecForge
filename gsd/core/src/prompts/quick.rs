//! `quick`, `quick_batch` and `fast` (D-r43-1 rows 4-6): the prompts for work too small for a phase. A quick task is a
//! plan with no phase, named by its quick id; `fast` makes no plan at all. None has an entry gate: every guard of this
//! extension is scoped to a phase.
//!
//! Also ports gsd-core bin/lib modules: quick-batch*.

use specforge_extension_sdk::prelude::*;

use super::args::Arg;
use super::workflow::{self, Do, Slash, Spec, Step};

const QUICK_STEPS: &[Step] = &[
    Step {
        id: "research",
        title: "Research how to implement the task",
        does: Do::Agent("gsd-phase-researcher"),
    },
    Step {
        id: "plan",
        title: "Create the quick plan",
        does: Do::Agent("gsd-planner"),
    },
    Step {
        id: "check",
        title: "Check the quick plan",
        does: Do::Agent("gsd-plan-checker"),
    },
    Step {
        id: "record_verdict",
        title: "Record the plan checker's verdict (specforge gsd record-verdict <plan> <code> <outcome> <model> <basis>)",
        does: Do::Command("record_verdict"),
    },
    Step {
        id: "start",
        title: "Open the plan at the host's clock (specforge gsd start-plan <plan>)",
        does: Do::Command("start_plan"),
    },
    Step {
        id: "execute",
        title: "Execute the quick plan",
        does: Do::Agent("gsd-executor"),
    },
    Step {
        id: "complete",
        title: "Complete the plan: both times of its summary (specforge gsd complete-plan <summary> --started <ts>)",
        does: Do::Command("complete_plan"),
    },
    Step {
        id: "verify",
        title: "Verify the quick plan's truths",
        does: Do::Agent("gsd-verifier"),
    },
];

/// How a quick plan is made, started and completed through the typed tools: the id is `<yymmdd>-<3 base-36>`, which
/// `gsd/W323` checks, and the plan has no phase.
fn quick_plan(_call: &PromptCall<'_>) -> String {
    quick_plan_text("the one the init block above made: a second one would name another plan")
}

/// The batch has no init block: each item of its manifest already carries the id.
fn quick_batch_plan(_call: &PromptCall<'_>) -> String {
    quick_plan_text("each manifest item's `quickId`: a second one would name another plan")
}

fn quick_plan_text(quick_id_is: &str) -> String {
    format!(
        "\n\n## The quick plan\n\n\
     A quick task is a plan with no phase, named by its quick id: the day as `yymmdd`, a dash and three base-36 \
     characters (`261029-a1b`). `gsd/W323` checks the name; `gsd/E302` and `gsd/E401` do not apply to a plan with no \
     phase. Write the plan, its truths and its artifacts through the host, not by hand:\n\n\
     ```bash\n\
     # $quick_id is {quick_id_is}\n\
     specforge new plan \"$quick_id\" \"<title>\" --in spec/quick.spec --type execute --files_modified <paths> --objective \"<objective>\"\n\
     specforge new truth TR1 \"<what must be true>\" --in spec/quick.spec --plan \"$quick_id\" --text \"<what must be true>\"\n\
     # an artifact carries a `path` field, which the CLI reads as its own --path: write it through the MCP tool `specforge.edit`\n\
     STARTED=$(specforge gsd start-plan \"$quick_id\" --format json | jq -r .started)\n\
     specforge new summary \"$quick_id-SUMMARY\" \"<title>\" --in spec/quick.spec --plan \"$quick_id\" --status complete --one_liner \"<one line>\" --subsystem <area> --requirements_completed \"\"\n\
     specforge gsd complete-plan \"$quick_id/$quick_id-SUMMARY\" --started \"$STARTED\"\n\
     ```\n\n\
     `start-plan` writes nothing and answers the time to keep; the summary is written first and `complete-plan` stamps \
     both of its times. The project state is computed, so no task row is written. \
     `gsd-code-reviewer` is not part of `@gsd/core`: skip the code review of `--full` and say so.\n"
    )
}

const QUICK_ARGS: &[Arg] = &[
    Arg::flag("full"),
    Arg::flag("validate"),
    Arg::flag("discuss"),
    Arg::flag("research"),
    Arg::rest("task"),
];

static QUICK: Spec = Spec {
    id: "quick",
    title: "Run a quick task",
    description: "Execute a quick task with GSD guarantees (atomic commits, state tracking) but skip optional agents",
    command: "quick",
    args: QUICK_ARGS,
    subject: None,
    gates: &[],
    steps: QUICK_STEPS,
    slash: Some(Slash {
        hint: "[list | status <slug> | resume <slug> | --full] [--validate] [--discuss] [--research] [task description]",
        tools: &[
            "read", "write", "edit", "search", "shell", "agent", "ask_user",
        ],
        effort: None,
    }),
    reads_state: false,
    tail: Some(quick_plan),
};

static QUICK_BATCH: Spec = Spec {
    id: "quick_batch",
    title: "Run a batch of quick tasks",
    description: "Batch several `/gsd:quick`-shaped tasks together — planned, dispatched, and merged as one run",
    command: "quick-batch",
    args: &[
        Arg::text("file"),
        Arg::text("jobs"),
        Arg::flag("validate"),
        Arg::flag("research"),
        Arg::text("resume"),
        Arg::rest("tasks"),
    ],
    subject: None,
    gates: &[],
    steps: QUICK_STEPS,
    slash: Some(Slash {
        hint: "[--file <path>] [--jobs auto|N] [--validate] [--research] [--resume <batch-id>] [task list]",
        tools: &["read", "write", "edit", "search", "shell", "agent"],
        effort: None,
    }),
    reads_state: false,
    tail: Some(quick_batch_plan),
};

static FAST: Spec = Spec {
    id: "fast",
    title: "Run a trivial task inline",
    description: "Execute a trivial task inline — no subagents, no planning overhead",
    command: "fast",
    args: &[Arg::rest("task")],
    subject: None,
    gates: &[],
    steps: &[],
    slash: Some(Slash {
        hint: "[task description]",
        tools: &["read", "write", "edit", "shell", "search"],
        effort: None,
    }),
    reads_state: false,
    tail: None,
};

pub fn declare(c: &mut ContributionsBuilder) {
    workflow::declare(c, &QUICK);
    workflow::declare(c, &QUICK_BATCH);
    workflow::declare(c, &FAST);
}
