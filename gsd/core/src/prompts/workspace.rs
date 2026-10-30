//! `workspace` (D-r43-1 row 20): the three workspace workflows behind one command. A workspace is another checkout,
//! which the model does not hold, so the text works in the shell.

use specforge_extension_sdk::prelude::*;

use super::args::Arg;
use super::workflow::{self, Slash, Spec};

/// What the model of a project does not hold.
fn checkout(_call: &PromptCall<'_>) -> String {
    "\n\n## What a workspace is here\n\n\
     A workspace is a directory of git worktrees or clones beside this project, with a `WORKSPACE.md` that the agent \
     writes: it is another checkout, so the spec does not model it and there is no `init.*-workspace` query. The text \
     above does its work with the shell (`ls`, `git worktree list`). A new workspace gets its own spec: \
     `specforge init`, `specforge add @gsd/core`, then `specforge install <host>` in the workspace directory. Only \
     `--name` is an option here; `--repos`, `--strategy` and `--branch` are asked for, and the workspace goes to \
     `~/gsd-workspaces/<name>`. The usage examples above that name `--repos` or `--auto` are not accepted by the \
     slash command: do not show them as commands to type, ask the questions instead (there is no unattended mode).\n"
        .to_string()
}

static WORKSPACE: Spec = Spec {
    id: "workspace",
    title: "Create, list or remove a workspace",
    description: "Manage GSD workspaces — create, list, or remove isolated workspace environments",
    command: "workspace",
    args: &[
        Arg::flag("new"),
        Arg::flag("list"),
        Arg::flag("remove"),
        Arg::text("name"),
    ],
    subject: None,
    gates: &[],
    steps: &[],
    slash: Some(Slash {
        hint: "[--new | --list | --remove] [--name <name>]",
        tools: &["read", "write", "shell", "search", "ask_user"],
        effort: None,
    }),
    tail: Some(checkout),
};

pub fn declare(c: &mut ContributionsBuilder) {
    workflow::declare(c, &WORKSPACE);
}
