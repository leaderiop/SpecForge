//! Ported from gsd-core v1.18.0 hooks/gsd-workflow-guard.js, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! `workflow_marker` and `workflow_guard` (D-r44-11).
//!
//! Upstream's guard warns on an edit made outside a GSD command, and learns it is outside one from nothing: it warns
//! on every edit that is not under `.planning/`. Here a `user_prompt` hook marks the session when the user starts a
//! `/gsd-<command>` (or `$gsd-<command>`, Codex's spelling), and the guard stays quiet in a session that has one. The
//! marker lasts the session and is replaced by the next command (U51). Cursor's `beforeSubmitPrompt` carries it too
//! (ADR 0322).
//!
//! The one block (#3504): a shell command that force-adds (`git add -f`, `--force`, `-Af`) on an agent branch
//! (`agent-*`, `worktree-agent-*`) is refused. The kernel splits the command at its control operators and tells the
//! hook the branch (ADR 0320); the walk past git's global options is `git-cmd.js`'s `skipToSubcommand`. Not ported:
//! the node hook's fail-closed posture on an internal error (a hook here does not crash into a policy), and the
//! branch of a `git -C <dir>` target (the branch is the hook's cwd's). Two of upstream's three exemptions are other facts here: the spec root is `.planning/`, and a subagent is
//! `tool.input.is_subagent` (the kernel carries no `session_type`). The advisory names this port's commands and,
//! where upstream says STATE.md and SUMMARY.md, the project's state and a summary.

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::facts::{Facts, truthy};

/// Names the user typed that start a GSD command: `/gsd-plan-phase 2` and `$gsd-plan-phase 2`.
fn command_of(prompt: &str) -> Option<String> {
    let word = prompt.split_whitespace().next()?;
    let name = word.strip_prefix(['/', '$'])?;
    let tail = name.strip_prefix("gsd-")?;
    (!tail.is_empty()
        && tail
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
    .then(|| format!("/{name}"))
}

/// Marks the session as inside a GSD command; any other prompt changes nothing.
pub(super) fn workflow_marker(f: &Facts) -> HookOutput {
    let Some(command) = f.prompt.as_deref().and_then(command_of) else {
        return HookOutput::default();
    };
    HookOutput {
        state: Some(StatePatch {
            session: Some(json!({"workflow": {"command": command, "at": f.now}})),
            project: None,
        }),
        ..HookOutput::default()
    }
}

/// Upstream's exemptions by file name: the ignore file, `.env*`, and the agent and settings files.
fn is_exempt(file: &str) -> bool {
    file.ends_with(".gitignore")
        || file.contains(".env")
        || ["CLAUDE.md", "AGENTS.md", "GEMINI.md", "settings.json"]
            .iter()
            .any(|n| file.ends_with(n))
}

/// Git's global options that take an argument (`git-cmd.js` `ARGUMENT_TAKING_FLAGS`).
const ARGUMENT_FLAGS: [&str; 11] = [
    "-C",
    "-c",
    "--git-dir",
    "--work-tree",
    "--namespace",
    "--super-prefix",
    "--exec-path",
    "--html-path",
    "--man-path",
    "--info-path",
    "--list-cmds",
];

/// Git's global flags that take none (`git-cmd.js` `BOOLEAN_FLAGS`).
const BOOLEAN_FLAGS: [&str; 14] = [
    "-p",
    "--paginate",
    "--no-pager",
    "--no-replace-objects",
    "--bare",
    "--literal-pathspecs",
    "--glob-pathspecs",
    "--noglob-pathspecs",
    "--icase-pathspecs",
    "--no-optional-locks",
    "-P",
    "--no-lazy-fetch",
    "--version",
    "--help",
];

/// `NAME=value`: a leading environment assignment.
fn is_assignment(word: &str) -> bool {
    word.split_once('=').is_some_and(|(name, _)| {
        name.chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    })
}

/// `git-cmd.js` `skipToSubcommand`: the index of the subcommand word, or none when the command is not a git call.
fn subcommand_index(words: &[String]) -> Option<usize> {
    let mut i = words.iter().take_while(|w| is_assignment(w)).count();
    if Facts::base(words.get(i)?) != "git" {
        return None;
    }
    i += 1;
    while let Some(word) = words.get(i) {
        let name = word.split_once('=').map_or(word.as_str(), |(n, _)| n);
        if ARGUMENT_FLAGS.contains(&name) {
            i += if name.len() == word.len() { 2 } else { 1 };
        } else if word
            .strip_prefix("-c")
            .and_then(|r| r.split_once('='))
            .is_some_and(|(key, _)| !key.contains(char::is_whitespace))
            || BOOLEAN_FLAGS.contains(&word.as_str())
        {
            i += 1;
        } else {
            break;
        }
    }
    Some(i)
}

/// A force flag: `--force`, `-f`, or a short cluster holding `f` (`-Af`).
fn is_force(word: &str) -> bool {
    word == "--force"
        || word.strip_prefix('-').is_some_and(|c| {
            !c.is_empty() && c.chars().all(|c| c.is_ascii_alphabetic()) && c.contains('f')
        })
}

/// Whether one of the commands is a `git add` with a force flag before any `--`.
fn force_adds(commands: &[Vec<String>]) -> bool {
    commands.iter().any(|words| {
        let Some(i) = subcommand_index(words).filter(|&i| words.get(i).is_some_and(|w| w == "add"))
        else {
            return false;
        };
        words[i + 1..]
            .iter()
            .take_while(|w| *w != "--")
            .any(|w| is_force(w))
    })
}

/// The branches of the agents (`isAgentBranch`).
fn on_agent_branch(f: &Facts) -> bool {
    f.git
        .as_ref()
        .is_some_and(|g| g.branch.starts_with("agent-") || g.branch.starts_with("worktree-agent-"))
}

/// Warns of an edit made in a session that has started no GSD command.
pub(super) fn workflow_guard(f: &Facts) -> HookOutput {
    if f.class == "shell" {
        return if force_adds(&f.commands) && on_agent_branch(f) {
            HookOutput::deny(
                "agent/worktree-agent branches must not run git add -f or git add --force. Respect the SDK \
                 skipped_gitignored/skipped_commit_docs_false contract and leave gitignored files untracked.",
            )
        } else {
            HookOutput::default()
        };
    }
    if !matches!(f.class.as_str(), "write" | "edit")
        || f.input.get("is_subagent").is_some_and(truthy)
    {
        return HookOutput::default();
    }
    let file = ["file_path", "path"]
        .iter()
        .find_map(|k| f.input_str(k).filter(|s| !s.is_empty()))
        .or_else(|| f.paths.first().map(|p| p.path.as_str()));
    let Some(file) = file else {
        return HookOutput::default();
    };
    let in_spec_root = !f.paths.is_empty() && f.paths.iter().all(|p| p.in_spec_root);
    if in_spec_root
        || file.contains(".planning/")
        || file.contains(".planning\\")
        || is_exempt(file)
    {
        return HookOutput::default();
    }
    let marked = f
        .session_state
        .as_ref()
        .and_then(|s| s.get("workflow"))
        .is_some_and(|w| w.get("command").is_some_and(truthy));
    if marked {
        return HookOutput::default();
    }
    HookOutput::allow().with_context(format!(
        "\u{26a0}\u{fe0f} WORKFLOW ADVISORY: You're editing {} directly without a GSD command. \
         This edit will not be tracked in the project's state or produce a summary. \
         Consider using /gsd-fast for trivial fixes or /gsd-quick for larger changes \
         to maintain project state tracking. \
         If this is intentional (e.g., user explicitly asked for a direct edit), proceed normally.",
        Facts::base(file)
    ))
}

pub(super) fn declare(c: &mut ContributionsBuilder) {
    c.hook("workflow_marker", HookEvent::UserPrompt, |h| {
        h.description("Marks the session when the user starts a /gsd- command")
            .handler(|call| Ok(workflow_marker(&Facts::of(call))));
    });
    c.hook("workflow_guard", HookEvent::PreTool, |h| {
        h.description("Warns of an edit made outside a GSD command, and refuses a force-add on an agent branch")
            .tools(&["write", "edit", "shell"])
            .when("hooks.workflow_guard")
            .reads_state()
            .reads_git()
            .handler(|call| Ok(workflow_guard(&Facts::of(call))));
    });
}

#[cfg(test)]
mod tests;
