//! `milestone_summary` and `extract_learnings` (D-r43-1 rows 16-17): a summary of a milestone shown to the person, and
//! the learnings of a phase recorded as `lesson` entities.

use specforge_extension_sdk::prelude::*;

use super::args::Arg;
use super::workflow::{self, Do, Slash, Spec, Step};

/// The summary is a report, not a file.
fn shown(_call: &PromptCall<'_>) -> String {
    "\n\n## Where the summary goes\n\n\
     The summary is shown to the user, never written: wherever the text above says it is written to a file or \
     committed, read that it is shown in the conversation. The files it names (`PROJECT.md`, `ROADMAP.md`, \
     `REQUIREMENTS.md`, and the `SUMMARY.md`, `VERIFICATION.md`, `CONTEXT.md` and `RESEARCH.md` of each phase) are the \
     `project`, `milestone`, `phase`, `requirement`, `summary`, `verification`, `context` and `research` entities, read \
     with the queries above.\n"
        .to_string()
}

/// A lesson has no phase and no title of its own.
fn recorded(_call: &PromptCall<'_>) -> String {
    "\n\n## Where the learnings go\n\n\
     Each learning is a `lesson` entity in `spec/lessons.spec`, written with `specforge new lesson`; its `kind` is the \
     category (`decision`, `lesson`, `pattern` or `surprise`), and `specforge check` checks it like any entity. A \
     `lesson` has no phase field and no counts: its id (`L-<phase>-<n>`) carries the phase, and the counts are the \
     lessons of each kind. `LEARNINGS.md` is not written. Nothing here removes a lesson: if an earlier run of the \
     phase recorded more lessons than this one, name the extra ids to the user.\n"
        .to_string()
}

const SUMMARY_STEPS: &[Step] = &[Step {
    id: "report",
    title: "Read the progress of the milestone (specforge gsd progress-report)",
    does: Do::Command("progress_report"),
}];

static MILESTONE_SUMMARY: Spec = Spec {
    id: "milestone_summary",
    title: "Summarise a milestone for onboarding",
    description: "Generate a comprehensive project summary from milestone artifacts for team onboarding and review",
    command: "milestone-summary",
    args: &[Arg::optional_reference("milestone", "milestone")],
    subject: None,
    gates: &[],
    steps: SUMMARY_STEPS,
    slash: Some(Slash {
        hint: "[--milestone <version>]",
        tools: &["read", "write", "shell", "search"],
        effort: None,
    }),
    reads_state: false,
    tail: Some(shown),
};

static EXTRACT_LEARNINGS: Spec = Spec {
    id: "extract_learnings",
    title: "Extract the learnings of a phase",
    description: "Extract decisions, lessons, patterns, and surprises from completed phase artifacts",
    command: "extract-learnings",
    args: &[Arg::reference("phase", "phase")],
    subject: Some("phase"),
    gates: &[],
    steps: &[],
    slash: Some(Slash {
        hint: "<phase>",
        tools: &["read", "write", "shell", "search", "agent"],
        effort: None,
    }),
    reads_state: false,
    tail: Some(recorded),
};

pub fn declare(c: &mut ContributionsBuilder) {
    workflow::declare(c, &MILESTONE_SUMMARY);
    workflow::declare(c, &EXTRACT_LEARNINGS);
}
