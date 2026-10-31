//! `pause_work` and `resume_work` (D-r43-1 rows 8-9): the session handoff. Pausing writes a `handoff` entity and a
//! `pause` that names it; resuming reads both and `resume` removes them.

use specforge_extension_sdk::prelude::*;

use super::args::Arg;
use super::workflow::{self, Do, Slash, Spec, Step};

const PAUSE_STEPS: &[Step] = &[Step {
    id: "pause",
    title: "Record the pause: why, who and the handoff (specforge gsd pause <reason> --by <who> --handoff <handoff>)",
    does: Do::Command("pause"),
}];

const RESUME_STEPS: &[Step] = &[Step {
    id: "resume",
    title: "Remove the pause and its handoff once the work is resumed (specforge gsd resume <pause>)",
    does: Do::Command("resume"),
}];

static PAUSE_WORK: Spec = Spec {
    id: "pause_work",
    title: "Pause the work",
    description: "Create context handoff when pausing work mid-phase",
    command: "pause-work",
    args: &[Arg::flag("report")],
    subject: None,
    gates: &[],
    steps: PAUSE_STEPS,
    slash: Some(Slash {
        hint: "[--report]",
        tools: &["read", "write", "shell", "search"],
        effort: None,
    }),
    reads_state: false,
    reads_slash_commands: false,
    tail: None,
};

static RESUME_WORK: Spec = Spec {
    id: "resume_work",
    title: "Resume the work",
    description: "Resume work from previous session with full context restoration",
    command: "resume-work",
    args: &[],
    subject: None,
    gates: &[],
    steps: RESUME_STEPS,
    slash: Some(Slash {
        hint: "",
        tools: &["read", "shell", "search", "write", "ask_user", "skill"],
        effort: None,
    }),
    reads_state: true,
    reads_slash_commands: false,
    tail: Some(workflow::workstream_tail),
};

pub fn declare(c: &mut ContributionsBuilder) {
    workflow::declare(c, &PAUSE_WORK);
    workflow::declare(c, &RESUME_WORK);
}
