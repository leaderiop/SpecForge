//! Ported from gsd-core v1.18.0 hooks/gsd-workflow-guard.js, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! `workflow_marker` and `workflow_guard` (D-r44-11).
//!
//! Upstream's guard warns on an edit made outside a GSD command, and learns it is outside one from nothing: it warns
//! on every edit that is not under `.planning/`. Here a `user_prompt` hook marks the session when the user starts a
//! `/gsd-<command>` (or `$gsd-<command>`, Codex's spelling), and the guard stays quiet in a session that has one. The
//! marker lasts the session and is replaced by the next command (U51); Cursor has no prompt event, so there the guard
//! is upstream's, always advisory.
//!
//! Dropped: the `git add -f` block on an agent branch, which needs a shell and git (K19 git facts). Two of
//! upstream's three exemptions are other facts here: the spec root is `.planning/`, and a subagent is
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

/// Warns of an edit made in a session that has started no GSD command.
pub(super) fn workflow_guard(f: &Facts) -> HookOutput {
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
        h.description("Warns of an edit made outside a GSD command")
            .tools(&["write", "edit"])
            .when("hooks.workflow_guard")
            .reads_state()
            .handler(|call| Ok(workflow_guard(&Facts::of(call))));
    });
}

#[cfg(test)]
mod tests;
