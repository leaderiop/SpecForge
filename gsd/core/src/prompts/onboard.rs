//! `ingest_docs` and `onboard` (D-r43-1 rows 18-19): the spec built from ADRs, PRDs and SPECs, and the route that
//! leads a repository to it. `ingest_docs` is declared first because `onboard` has a step that runs it.

use specforge_extension_sdk::prelude::*;

use super::args::Arg;
use super::workflow::{self, Do, Slash, Spec, Step};

/// Ingestion writes entities, not the files GSD wrote.
fn ingested(_call: &PromptCall<'_>) -> String {
    "\n\n## Where the ingest goes\n\n\
     The project is read from the directory `--dir` names (`--path` is the host's own option and does not reach this \
     prompt). The classifier and the synthesizer write their staging files to a scratch directory outside the project: \
     the intel files and the conflict report are not part of the spec, and the report is shown to the user. Wherever \
     the text above says `PROJECT.md`, `REQUIREMENTS.md`, `ROADMAP.md` or `STATE.md` is written, read that the \
     roadmapper writes the `project`, `requirement`, `milestone` and `phase` entities (`specforge new`, `specforge \
     set`), and that the project state is computed. `--resolve interactive` is reserved, as upstream's.\n"
        .to_string()
}

/// The summary is shown, and mapping is another extension's.
fn onboarding(_call: &PromptCall<'_>) -> String {
    "\n\n## What onboarding does here\n\n\
     `/gsd-map-codebase` is served by `@gsd/codebase`, which is not installed here: the route always takes upstream's \
     \"Skip mapping\" path. The onboarding summary is shown, not written, and the project state is computed, so there \
     is no `STATE.md` to be present or missing.\n"
        .to_string()
}

static INGEST_DOCS: Spec = Spec {
    id: "ingest_docs",
    title: "Ingest ADRs, PRDs and SPECs into the spec",
    description: "Bootstrap or merge a SpecForge spec setup from existing ADRs, PRDs, SPECs, and docs in a repo.",
    command: "ingest-docs",
    args: &[
        Arg::text("dir"),
        Arg::one_of("mode", &["new", "merge"]),
        Arg::text("manifest"),
        Arg::one_of("resolve", &["auto", "interactive"]),
    ],
    subject: None,
    gates: &[],
    steps: &[
        Step {
            id: "classify",
            title: "Classify each document in parallel (gsd-doc-classifier)",
            does: Do::Agent("gsd-doc-classifier"),
        },
        Step {
            id: "synthesize",
            title: "Synthesize the classifications and detect conflicts (gsd-doc-synthesizer)",
            does: Do::Agent("gsd-doc-synthesizer"),
        },
        Step {
            id: "roadmap",
            title: "Write the project, requirements and roadmap (gsd-roadmapper)",
            does: Do::Agent("gsd-roadmapper"),
        },
    ],
    slash: Some(Slash {
        hint: "[--dir <path>] [--mode new|merge] [--manifest <file>] [--resolve auto|interactive]",
        tools: &[
            "read", "write", "edit", "shell", "search", "ask_user", "agent",
        ],
        effort: None,
    }),
    tail: Some(ingested),
};

static ONBOARD: Spec = Spec {
    id: "onboard",
    title: "Onboard an existing repository",
    description: "Guide existing codebase onboarding through mapping, doc ingest, and planning setup",
    command: "onboard",
    args: &[Arg::flag("fast"), Arg::flag("text")],
    subject: None,
    gates: &[],
    steps: &[
        Step {
            id: "ingest",
            title: "Ingest the existing documents",
            does: Do::Prompt("ingest_docs"),
        },
        Step {
            id: "new_project",
            title: "Start the project",
            does: Do::Prompt("new_project"),
        },
        Step {
            id: "manager",
            title: "Hand over to the manager",
            does: Do::Prompt("manager"),
        },
    ],
    slash: Some(Slash {
        hint: "[--fast] [--text]",
        tools: &["read", "shell", "write", "search", "agent", "ask_user"],
        effort: None,
    }),
    tail: Some(onboarding),
};

pub fn declare(c: &mut ContributionsBuilder) {
    workflow::declare(c, &INGEST_DOCS);
    workflow::declare(c, &ONBOARD);
}
