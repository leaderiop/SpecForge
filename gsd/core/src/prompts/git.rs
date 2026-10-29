//! `undo` and `pr_branch` (D-r43-1 rows 14-15): the two prompts that run `git` in the person's repository. Undo
//! reverts the commits the graph records for a plan or a phase; pr-branch rebuilds the branch without the spec.

use specforge_extension_sdk::prelude::*;

use super::args::Arg;
use super::workflow::{self, Slash, Spec};

/// A revert is `git`, not a host write: what it can take away.
fn revert(_call: &PromptCall<'_>) -> String {
    "\n\n## What a revert can take away\n\n\
     The commits to revert are the `commit` entities recorded under the plan or the phase (`specforge collect \
     --runner gsd-git` records them). A commit that was never collected is not found there: `--last N` selects from \
     `git log` instead.\n\n\
     `git revert` is not a SpecForge write: it bypasses the host's checks, so a reverted `.spec` hunk can remove a \
     value a person wrote (an override, a waiver). Before reverting a commit whose diff touches a `.spec` file, read \
     that diff, and ask the user before reverting one that removes a value a person wrote. After the revert, run \
     `specforge check` and report what it says.\n"
        .to_string()
}

/// The filter has one mode, and what it filters.
fn filtered(_call: &PromptCall<'_>) -> String {
    "\n\n## What the PR branch leaves out\n\n\
     The filter is always strict: every `*.spec` file and the `specforge-harness` directory are filtered out of the \
     cherry-picked commits, and a commit that touches nothing else is left out. `planning.pr_strict` and \
     `planning.sub_repos` are not settings here: a sub-repository is another SpecForge project, so there is nothing to \
     process in a second repository.\n"
        .to_string()
}

static UNDO: Spec = Spec {
    id: "undo",
    title: "Revert the commits of a phase or a plan",
    description: "Safe git revert. Roll back phase or plan commits using the phase manifest with dependency checks.",
    command: "undo",
    args: &[
        Arg::integer("last"),
        Arg::optional_reference("phase", "phase"),
        Arg::optional_reference("plan", "plan"),
    ],
    subject: None,
    gates: &[],
    steps: &[],
    slash: Some(Slash {
        hint: "--last N | --phase NN | --plan NN-MM",
        tools: &["read", "shell", "search", "ask_user"],
        effort: None,
    }),
    tail: Some(revert),
};

static PR_BRANCH: Spec = Spec {
    id: "pr_branch",
    title: "Create a clean branch for the pull request",
    description: "Create a clean PR branch by filtering out spec-only commits — ready for code review",
    command: "pr-branch",
    args: &[Arg::text("target")],
    subject: None,
    gates: &[],
    steps: &[],
    slash: Some(Slash {
        hint: "[--target <branch>]",
        tools: &["shell", "search", "read", "ask_user"],
        effort: None,
    }),
    tail: Some(filtered),
};

pub fn declare(c: &mut ContributionsBuilder) {
    workflow::declare(c, &UNDO);
    workflow::declare(c, &PR_BRANCH);
}
