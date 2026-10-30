//! Ported from gsd-core v1.18.0 hooks/gsd-agent-isolation-guard.js and hooks/gsd-worktree-path-guard.js, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! `agent_isolation_guard` and `worktree_path_guard` (D-r44-10), both heuristics where upstream reads files and git.
//!
//! Upstream resolves the dispatch isolation from a sentinel file, the runtime registry and `.planning/config.json`, and
//! finds a GSD worktree from `git rev-parse` and the branch name. A hook here reads none of them, so:
//! - the isolation guard is Claude Code's, whose `Agent` tool takes the `isolation` parameter, and runs while
//!   `workflow.use_worktrees` is on (the hook's `when`, a no-op otherwise);
//! - the path guard knows a GSD worktree by where it is: a project under `<repo>/.claude/worktrees/agent-*` (Claude
//!   Code's own place for a subagent's worktree, whose branch is upstream's `worktree-agent-*`) or `wf_*`. It refuses
//!   an absolute path into `<repo>`, the main checkout, which is the bug it exists for (#260); upstream lets a path
//!   outside every repository through, and so does this one.

use specforge_extension_sdk::prelude::*;

use super::facts::Facts;

/// The subagent types that write code and so need their own checkout.
const EXECUTORS: [&str; 1] = ["gsd-executor"];

/// Refuses to start an executor in the checkout the orchestrator is in.
pub(super) fn agent_isolation_guard(f: &Facts) -> HookOutput {
    if f.host != "claude-code" || f.class != "agent" || !matches!(f.tool.as_str(), "Agent" | "Task")
    {
        return HookOutput::default();
    }
    let Some(kind) = f
        .input
        .get("subagent_type")
        .and_then(|v| v.as_str())
        .filter(|k| EXECUTORS.contains(k))
    else {
        return HookOutput::default();
    };
    if f.input.get("isolation").and_then(|v| v.as_str()) == Some("worktree") {
        return HookOutput::default();
    }
    HookOutput::deny(format!(
        "Agent isolation guard (gsd.agent_isolation_guard): this project's dispatch isolation is \"harness-worktree\" \
         (workflow.use_worktrees is on), but the Agent() dispatch for subagent_type=\"{kind}\" is missing \
         isolation=\"worktree\". Add isolation=\"worktree\" to the Agent() call so the executor runs in an isolated \
         worktree instead of the primary checkout (see the execute-phase prompt)."
    ))
}

/// A directory name that is a GSD agent's worktree: upstream's branch `worktree-agent-*` and `worktree-wf_*`.
fn is_agent_dir(name: &str) -> bool {
    let rest = name
        .strip_prefix("agent-")
        .or_else(|| name.strip_prefix("wf_"));
    rest.is_some_and(|r| {
        !r.is_empty()
            && r.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    })
}

/// `(main checkout, worktree)` when `cwd` is inside a GSD agent's worktree under `<main>/.claude/worktrees/`.
fn agent_worktree(cwd: &str) -> Option<(String, String)> {
    const MARK: &str = "/.claude/worktrees/";
    let cwd = cwd.replace('\\', "/");
    let at = cwd.find(MARK)?;
    let name = cwd[at + MARK.len()..].split('/').next()?;
    is_agent_dir(name).then(|| {
        let main = cwd[..at].to_string();
        let worktree = format!("{main}{MARK}{name}");
        (main, worktree)
    })
}

/// `path` with `.` and `..` resolved, as `path.resolve` does for an absolute path.
fn resolved(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            p => parts.push(p),
        }
    }
    format!("/{}", parts.join("/"))
}

fn under(path: &str, root: &str) -> bool {
    path == root || path.strip_prefix(root).is_some_and(|r| r.starts_with('/'))
}

/// Refuses an absolute path into the main checkout from inside an agent's worktree.
pub(super) fn worktree_path_guard(f: &Facts) -> HookOutput {
    if !matches!(f.class.as_str(), "write" | "edit") {
        return HookOutput::default();
    }
    let Some((main, worktree)) = agent_worktree(&f.cwd) else {
        return HookOutput::default();
    };
    for p in f
        .paths
        .iter()
        .filter(|p| !p.in_project && p.path.starts_with('/'))
    {
        let path = resolved(&p.path);
        if under(&path, &worktree) || !under(&path, &main) {
            continue;
        }
        return HookOutput::deny(format!(
            "Worktree path guard (gsd.worktree_path_guard): '{path}' resolves to git root '{main}' which differs from \
             the active worktree root '{worktree}'. This likely means an absolute path was derived from the \
             orchestrator's main repository instead of the active worktree. To fix: use a relative path, or \
             re-derive the base directory with `git rev-parse --show-toplevel` from within the worktree (hook cwd: \
             '{}').",
            f.cwd
        ));
    }
    HookOutput::default()
}

pub(super) fn declare(c: &mut ContributionsBuilder) {
    c.hook("agent_isolation_guard", HookEvent::PreTool, |h| {
        h.description("Refuses to start a gsd-executor outside a worktree while worktrees are on")
            .tools(&["agent"])
            .when("workflow.use_worktrees")
            .handler(|call| Ok(agent_isolation_guard(&Facts::of(call))));
    });
    c.hook("worktree_path_guard", HookEvent::PreTool, |h| {
        h.description(
            "Refuses a write from an agent's worktree to an absolute path in the main checkout",
        )
        .tools(&["write", "edit"])
        .handler(|call| Ok(worktree_path_guard(&Facts::of(call))));
    });
}

#[cfg(test)]
mod tests;
