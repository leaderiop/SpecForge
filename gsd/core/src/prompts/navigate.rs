//! `next`, `progress` and `manager` (D-r43-1 rows 1-3): the prompts that read the project's situation and send the
//! person to the next prompt. `next` routes from the command `smart_entry`, `progress` and `manager` from
//! `progress_report`; neither carries an entry gate, because each step into a lifecycle prompt is gated there.

use specforge_extension_sdk::prelude::*;

use super::args::Arg;
use super::ns::served;
use super::workflow::{self, Do, Slash, Spec, Step};

const NEXT_STEPS: &[Step] = &[Step {
    id: "detect",
    title: "Detect the situation and its next actions (specforge gsd smart-entry)",
    does: Do::Command("smart_entry"),
}];

const PROGRESS_STEPS: &[Step] = &[Step {
    id: "report",
    title: "Report the phases, their plans and summaries, and the percent done (specforge gsd progress-report)",
    does: Do::Command("progress_report"),
}];

const MANAGER_STEPS: &[Step] = &[
    Step {
        id: "dashboard",
        title: "Read the phases and what each needs next (specforge gsd progress-report)",
        does: Do::Command("progress_report"),
    },
    Step {
        id: "discuss",
        title: "Discuss a phase inline",
        does: Do::Prompt("discuss_phase"),
    },
    Step {
        id: "plan",
        title: "Plan a phase",
        does: Do::Prompt("plan_phase"),
    },
    Step {
        id: "execute",
        title: "Execute a phase",
        does: Do::Prompt("execute_phase"),
    },
    Step {
        id: "verify",
        title: "Verify a phase's work",
        does: Do::Prompt("verify_work"),
    },
    Step {
        id: "complete",
        title: "Complete the milestone once every phase is done",
        does: Do::Prompt("complete_milestone"),
    },
];

/// The commands `smart_entry` can recommend that this project may not serve: said, so the person is not sent to a
/// command this install does not have (D-r43-20).
fn not_installed(call: &PromptCall<'_>) -> String {
    let lines: Vec<String> = ["debug", "code-review", "capture", "map-codebase"]
        .iter()
        .map(|stem| format!("- `/gsd-{stem}`: {}", served(call, stem)))
        .collect();
    format!(
        "\n\n## Commands this install may not serve\n\n\
         The menu may name these commands; this is how the project serves each:\n\n{}\n\n\
         When the recommended action is one that is not served on this project, say so and offer the next action \
         that is: `specforge gsd resolve-blocker` or `/gsd-verify-work` for a blocked project, the next phase \
         prompt otherwise.\n",
        lines.join("\n")
    )
}

static NEXT: Spec = Spec {
    id: "next",
    title: "Route to the next action",
    description: "Smart entry — detect project state and route to the right next GSD action.",
    command: "next",
    args: &[Arg::flag("text")],
    subject: None,
    gates: &[],
    steps: NEXT_STEPS,
    slash: Some(Slash {
        hint: "",
        tools: &["read", "shell", "search", "skill", "ask_user"],
        effort: Some("low"),
    }),
    reads_state: false,
    reads_slash_commands: true,
    tail: Some(not_installed),
};

static PROGRESS: Spec = Spec {
    id: "progress",
    title: "Check progress",
    description: "Check progress, advance workflow, or dispatch freeform intent — the unified GSD situational command",
    command: "progress",
    args: &[
        Arg::flag("forensic"),
        Arg::flag("next"),
        Arg::flag("auto"),
        Arg::flag("converge"),
        Arg::text("do"),
    ],
    subject: None,
    gates: &[],
    steps: PROGRESS_STEPS,
    slash: Some(Slash {
        hint: "[--forensic | --next [--auto] [--converge] | --do \"task description\"]",
        tools: &["read", "shell", "search", "skill", "ask_user"],
        effort: Some("low"),
    }),
    reads_state: true,
    reads_slash_commands: false,
    tail: Some(workflow::workstream_tail),
};

static MANAGER: Spec = Spec {
    id: "manager",
    title: "Manage the phases",
    description: "Interactive command center for managing multiple phases from one terminal",
    command: "manager",
    args: &[Arg::flag("analyze_deps")],
    subject: None,
    gates: &[],
    steps: MANAGER_STEPS,
    slash: Some(Slash {
        hint: "[--analyze-deps]",
        tools: &[
            "read", "write", "shell", "search", "ask_user", "skill", "agent",
        ],
        effort: None,
    }),
    reads_state: false,
    reads_slash_commands: false,
    tail: None,
};

pub fn declare(c: &mut ContributionsBuilder) {
    workflow::declare(c, &NEXT);
    workflow::declare(c, &PROGRESS);
    workflow::declare(c, &MANAGER);
}
