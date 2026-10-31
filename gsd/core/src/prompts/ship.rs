//! `complete_milestone` and `ship` (D-r42-1 rows 16-17, D-r42-9): the two prompts gated by their guards. The
//! milestone prompt carries `close_milestone`'s codes, so both refuse together; `ship` adds the broken-window guard.

use specforge_extension_sdk::prelude::*;

use super::args::Arg;
use super::workflow::{self, Do, Slash, Spec, Step};

const COMPLETE_STEPS: &[Step] = &[Step {
    id: "close",
    title: "Close the milestone: its completion and its retrospective (specforge gsd close-milestone <milestone> --built <json>)",
    does: Do::Command("close_milestone"),
}];

const SHIP_STEPS: &[Step] = &[
    Step {
        id: "fix_window",
        title: "Record that a broken window was fixed (specforge gsd fix-window <window>)",
        does: Do::Command("fix_window"),
    },
    Step {
        id: "waive_window",
        title: "Ask the person to run `specforge gsd waive-window <window> <reason> <who>` in their terminal (an agent's call is refused: it records what a person says)",
        does: Do::Command("waive_window"),
    },
];

static COMPLETE_MILESTONE: Spec = Spec {
    id: "complete_milestone",
    title: "Complete a milestone",
    description: "Archive completed milestone and prepare for next version",
    command: "complete-milestone",
    args: &[Arg::reference("milestone", "milestone")],
    subject: Some("milestone"),
    gates: &["E106", "E511", "E516"],
    steps: COMPLETE_STEPS,
    slash: Some(Slash {
        hint: "<milestone>",
        tools: &["read", "write", "shell", "search"],
        effort: None,
    }),
    tail: None,
};

static SHIP: Spec = Spec {
    id: "ship",
    title: "Ship a phase",
    description: "Create PR, run review, and prepare for merge after verification passes",
    command: "ship",
    args: &[Arg::reference("phase", "phase")],
    subject: Some("phase"),
    gates: &["E511", "E512", "E516"],
    steps: SHIP_STEPS,
    slash: Some(Slash {
        hint: "<phase>",
        tools: &["read", "shell", "search", "write", "ask_user"],
        effort: None,
    }),
    tail: None,
};

pub fn declare(c: &mut ContributionsBuilder) {
    workflow::declare(c, &COMPLETE_MILESTONE);
    workflow::declare(c, &SHIP);
}
