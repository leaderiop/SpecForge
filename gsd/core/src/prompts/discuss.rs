//! `discuss_phase` (D-r42-1 row 3): gathers a phase's context. The command routes in prose between the discuss and
//! assumptions workflows and the nine modes, which are read on demand with `specforge gsd reference`.

use specforge_extension_sdk::prelude::*;

use super::args::Arg;
use super::workflow::{self, Do, Slash, Spec, Step};

const ARGS: &[Arg] = &[
    Arg::reference("phase", "phase"),
    Arg::flag("all"),
    Arg::flag("auto"),
    Arg::flag("chain"),
    Arg::flag("batch"),
    Arg::flag("analyze"),
    Arg::flag("text"),
    Arg::flag("power"),
    Arg::flag("assumptions"),
];

const STEPS: &[Step] = &[
    Step {
        id: "analyze_assumptions",
        title: "Analyze the codebase for the phase's assumptions (gsd-assumptions-analyzer), in assumptions mode",
        does: Do::Agent("gsd-assumptions-analyzer"),
    },
    Step {
        id: "research_gray_areas",
        title: "Research a gray area for the advisor table (gsd-advisor-researcher), in advisor mode",
        does: Do::Agent("gsd-advisor-researcher"),
    },
];

static DISCUSS_PHASE: Spec = Spec {
    id: "discuss_phase",
    title: "Discuss a phase",
    description: "Gather phase context through adaptive questioning before planning.",
    command: "discuss-phase",
    args: ARGS,
    subject: Some("phase"),
    gates: &[],
    steps: STEPS,
    slash: Some(Slash {
        hint: "<phase> [--all] [--auto] [--chain] [--batch] [--analyze] [--text] [--power] [--assumptions]",
        tools: &[
            "read",
            "write",
            "shell",
            "search",
            "ask_user",
            "agent",
            "mcp:context7/resolve-library-id",
            "mcp:context7/query-docs",
        ],
        effort: None,
    }),
    tail: None,
};

pub fn declare(c: &mut ContributionsBuilder) {
    workflow::declare(c, &DISCUSS_PHASE);
}
