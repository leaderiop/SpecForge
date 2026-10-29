//! `review` and `review_backlog` (D-r43-1 rows 10-11): cross-AI review of a phase's plans, and the review of the
//! backlog phases. The reviewers are external CLIs the shell runs; a backlog phase is promoted or removed by `phase`.

use specforge_extension_sdk::prelude::*;

use super::args::Arg;
use super::workflow::{self, Do, Slash, Spec, Step};

/// Upstream's order: the reviewer lanes, then `--all`.
const REVIEW_ARGS: &[Arg] = &[
    Arg::reference("phase", "phase"),
    Arg::flag("claude"),
    Arg::flag("codex"),
    Arg::flag("coderabbit"),
    Arg::flag("opencode"),
    Arg::flag("qwen"),
    Arg::flag("cursor"),
    Arg::flag("antigravity"),
    Arg::flag("agy"),
    Arg::flag("ollama"),
    Arg::flag("lm_studio"),
    Arg::flag("llama_cpp"),
    Arg::flag("kimi_code"),
    Arg::flag("all"),
];

/// Where the reviews go: nowhere on disk.
fn shown(_call: &PromptCall<'_>) -> String {
    "\n\n## Where the reviews go\n\n\
     The reviews are shown to the user, not stored: wherever the text above says `REVIEWS.md` is written or \
     committed, read that the combined review is shown in the conversation. `/gsd-plan-phase <phase> --reviews` \
     reads it from there, and its dispositions are recorded as `disposition` rows.\n\n\
     The reviewers are the CLIs of the flags (`claude`, `codex`, `coderabbit`, `opencode`, `qwen`, `cursor`, `agy`, \
     `kimi`) and the local servers (`ollama`, `lm-studio`, `llama-cpp`), run through the shell. Their invocation is \
     the CLI's own: there is no lane manifest, no prompt budget and no lane runner here, so read each CLI's `--help` \
     for the flag that takes a prompt.\n"
        .to_string()
}

static REVIEW: Spec = Spec {
    id: "review",
    title: "Ask other AI CLIs to review the phase plans",
    description: "Request cross-AI peer review of phase plans from external AI CLIs",
    command: "review",
    args: REVIEW_ARGS,
    subject: Some("phase"),
    gates: &[],
    steps: &[],
    slash: Some(Slash {
        hint: "<phase> [--claude] [--codex] [--opencode] [--qwen] [--cursor] [--agy] [--all]",
        tools: &["read", "write", "shell", "search"],
        effort: None,
    }),
    tail: Some(shown),
};

const BACKLOG_STEPS: &[Step] = &[Step {
    id: "phase",
    title: "Promote a backlog phase into the milestone, or remove it (specforge gsd phase)",
    does: Do::Command("phase"),
}];

static REVIEW_BACKLOG: Spec = Spec {
    id: "review_backlog",
    title: "Review the backlog",
    description: "Review and promote backlog items to active milestone",
    command: "review-backlog",
    args: &[],
    subject: None,
    gates: &[],
    steps: BACKLOG_STEPS,
    slash: Some(Slash {
        hint: "",
        tools: &["read", "write", "shell", "search", "ask_user"],
        effort: None,
    }),
    tail: None,
};

pub fn declare(c: &mut ContributionsBuilder) {
    workflow::declare(c, &REVIEW);
    workflow::declare(c, &REVIEW_BACKLOG);
}
