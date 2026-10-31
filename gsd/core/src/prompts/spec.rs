//! `spec_phase` and `mvp_phase` (D-r42-1 rows 4-5). `spec_phase` is also the judge of `gsd/E202` and `gsd/W203`
//! (`judged.rs`, D-r40-11, D-r42-15): it renders upstream's spec-phase workflow and appends the judging section for
//! `<phase>-SPEC`, so a phase's spec is interviewed and judged by one prompt.

use specforge_extension_sdk::prelude::*;

use super::args::Arg;
use super::workflow::{self, Do, Slash, Spec, Step};

/// The ambiguity gates of `gsd/E202`, in percent: `(score, bound, is_a_maximum)` (upstream spec-phase: ambiguity <= 0.20
/// and each dimension at its minimum). `record_verdict` enforces them on a `passed`.
pub const GATES: [(&str, i64, bool); 5] = [
    ("goal", 75, false),
    ("boundary", 70, false),
    ("constraint", 65, false),
    ("acceptance", 70, false),
    ("ambiguity", 20, true),
];

fn thresholds() -> String {
    let gate = |(name, bound, max): &(&str, i64, bool)| {
        format!("{name} {} {bound}", if *max { "<=" } else { ">=" })
    };
    GATES.iter().map(gate).collect::<Vec<_>>().join(", ")
}

/// The id of a phase's spec: `v1.0/02` is judged as `v1.0/02/02-SPEC`.
fn spec_of(phase: &str) -> String {
    let number = phase.rsplit('/').next().unwrap_or(phase);
    format!("{phase}/{number}-SPEC")
}

fn judging(call: &PromptCall<'_>) -> String {
    let spec = spec_of(call.str("phase").unwrap_or_default());
    let thresholds = thresholds();
    format!(
        "\n\n## Judging the spec\n\n\
         Judge the phase spec `{spec}`.\n\n\
         Two rules need your verdict. Read the inputs of each with `specforge query <entity> --inputs gsd/<code> \
         --format json`, judge, then record the outcome with `specforge gsd record-verdict <entity> <code> <outcome> \
         <model> '<basis>'`, where `<basis>` is the `basis` object that query printed.\n\n\
         - `gsd/E202` on `{spec}`: does the spec meet its ambiguity gates? Score it in percent ({thresholds}). \
         Record `passed` or `failed` and pass the five scores as `--values '{{\"scores\": {{\"goal\": 0, \"boundary\": 0, \
         \"constraint\": 0, \"acceptance\": 0, \"ambiguity\": 0}}}}'` (your own numbers).\n\
         - `gsd/W203` on each locked requirement of the spec (`{spec}/LR1`, ...): is its acceptance falsifiable, \
         something a test can fail? Record `passed` or `failed`.\n\n\
         Judge what the inputs show, not what the spec hopes. Name each problem in `--findings`.\n"
    )
}

const SPEC_ARGS: &[Arg] = &[
    Arg::reference("phase", "phase"),
    Arg::flag("auto"),
    Arg::flag("text"),
];

const SPEC_STEPS: &[Step] = &[
    Step {
        id: "record_spec",
        title: "Record the spec's digest once it is written (specforge gsd record-spec <phase_spec>)",
        does: Do::Command("record_spec"),
    },
    Step {
        id: "discuss",
        title: "Discuss the phase next (discuss_phase)",
        does: Do::Prompt("discuss_phase"),
    },
];

static SPEC_PHASE: Spec = Spec {
    id: "spec_phase",
    title: "Specify a phase",
    description: "Clarify WHAT a phase delivers with ambiguity scoring; produces a SPEC.md before discuss-phase.",
    command: "spec-phase",
    args: SPEC_ARGS,
    subject: Some("phase"),
    gates: &[],
    steps: SPEC_STEPS,
    slash: Some(Slash {
        hint: "<phase> [--auto] [--text]",
        tools: &["read", "write", "shell", "search", "ask_user"],
        effort: None,
    }),
    reads_state: false,
    tail: Some(judging),
};

const MVP_ARGS: &[Arg] = &[Arg::reference("phase", "phase")];

const MVP_STEPS: &[Step] = &[Step {
    id: "plan",
    title: "Plan the phase, in MVP mode (plan_phase --mvp)",
    does: Do::Prompt("plan_phase"),
}];

static MVP_PHASE: Spec = Spec {
    id: "mvp_phase",
    title: "Plan a phase as an MVP slice",
    description: "Plan a phase as a vertical MVP slice — user story, SPIDR splitting, then plan-phase",
    command: "mvp-phase",
    args: MVP_ARGS,
    subject: Some("phase"),
    gates: &["E516"],
    steps: MVP_STEPS,
    slash: Some(Slash {
        hint: "<phase-number>",
        tools: &["read", "write", "shell", "search", "agent", "ask_user"],
        effort: None,
    }),
    reads_state: false,
    tail: None,
};

pub fn declare(c: &mut ContributionsBuilder) {
    workflow::declare(c, &SPEC_PHASE);
    workflow::declare(c, &MVP_PHASE);
}
