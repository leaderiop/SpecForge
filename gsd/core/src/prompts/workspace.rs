//! `workspace` (D-r43-1 row 20): the three workspace workflows behind one command. A workspace is another checkout,
//! which the model does not hold, so the text works in the shell.

use specforge_extension_sdk::prelude::*;

use super::args::Arg;
use super::workflow::{self, Slash, Spec};

/// What the model of a project does not hold, and the arguments the workflows read.
fn checkout(_call: &PromptCall<'_>) -> String {
    "\n\n## What a workspace is here\n\n\
     A workspace is a directory of git worktrees or clones beside this project, with a `WORKSPACE.md` that the agent \
     writes: it is another checkout, so the spec does not model it and there is no `init.*-workspace` query. The text \
     above does its work with the shell (`ls`, `git worktree list`). A new workspace gets its own spec: \
     `specforge init`, `specforge add @gsd/core`, then `specforge install <host>` in the workspace directory.\n\n\
     The arguments are read as the workflow's parse says (`--name` is the bare words after the options): \
     `--repos` the comma-separated repository paths or names, `--path` the target directory (default \
     `~/gsd-workspaces/<name>`), `--strategy worktree|clone` (default `worktree`), `--branch` the branch name (default \
     `workspace/<name>`), and `--auto` skips the questions.\n"
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
        Arg::text("repos"),
        Arg::text("path"),
        Arg::one_of("strategy", &["worktree", "clone"]),
        Arg::text("branch"),
        Arg::flag("auto"),
        Arg::rest("name"),
    ],
    subject: None,
    gates: &[],
    steps: &[],
    slash: Some(Slash {
        hint: "[--new | --list | --remove] [name]",
        tools: &["read", "write", "shell", "search", "ask_user"],
        effort: None,
    }),
    reads_state: false,
    tail: Some(checkout),
};

pub fn declare(c: &mut ContributionsBuilder) {
    workflow::declare(c, &WORKSPACE);
}
