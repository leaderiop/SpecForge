//! Ported from gsd-core v1.18.0 hooks/gsd-agent-isolation-guard.js and hooks/gsd-worktree-path-guard.js, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! `worktree_path_guard` (D-r44-10), upstream's guard on the git facts the kernel hands a hook (ADR 0320).
//!
//! There is no `agent_isolation_guard`. Upstream resolves the dispatch isolation from a sentinel file, the runtime
//! registry and `.planning/config.json`, and denies an executor without `isolation:"worktree"` only when the isolation
//! is `harness-worktree`; in sequential mode (`USE_WORKTREES=false`) it allows. @gsd/core has no worktree isolation: its
//! execute workflows set `USE_WORKTREES="false"` and tell the agent to omit `isolation`, so every dispatch it makes is
//! the sequential one (REVIEW-r44 C1). A hook that always allows would cost every `Agent`/`Task` dispatch a process,
//! so none is installed (r44 post-fix N7) until the `specforge` harness runs executors in worktrees.
//!
//! The path guard reads git's answer as upstream does: the cwd's linked worktree (its git directory holds
//! `/.git/worktrees/`) on an agent branch (`^((worktree-)?agent-|worktree-wf_)[A-Za-z0-9._/-]+$`), each target's real
//! path and the repository it is in. A target inside a `.git` directory that is no work tree is refused, and so is one
//! in a repository other than the worktree's (the main checkout is the bug it exists for, #260); a target in no
//! repository is let through. A relative path that leaves the project is judged by where it lands, which upstream
//! does not (it allows every relative path: ACCEPTANCE's `[stricter-relative-path]`).

use specforge_extension_sdk::prelude::*;

use super::facts::Facts;

/// Whether `branch` is a GSD executor's: upstream's `^((worktree-)?agent-|worktree-wf_)[A-Za-z0-9._/-]+$`.
fn agent_branch(branch: &str) -> bool {
    let rest = branch
        .strip_prefix("worktree-agent-")
        .or_else(|| branch.strip_prefix("agent-"))
        .or_else(|| branch.strip_prefix("worktree-wf_"));
    rest.is_some_and(|r| {
        !r.is_empty()
            && r.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '/' | '-'))
    })
}

/// Whether `git_dir` is a linked worktree's: it holds `/.git/worktrees/` as components, whichever the separator.
fn linked_worktree(git_dir: &str) -> bool {
    git_dir.replace('\\', "/").contains("/.git/worktrees/")
}

/// Refuses a write from an agent's worktree to a path in another repository, or inside a `.git` directory.
pub(super) fn worktree_path_guard(f: &Facts) -> HookOutput {
    if !matches!(f.class.as_str(), "write" | "edit") {
        return HookOutput::default();
    }
    let Some(git) = f
        .git
        .as_ref()
        .filter(|g| linked_worktree(&g.git_dir) && agent_branch(&g.branch))
    else {
        return HookOutput::default();
    };
    for p in f.paths.iter().filter(|p| !p.in_project) {
        match &p.toplevel {
            None if p.in_git_dir => {
                return HookOutput::deny(format!(
                    "Worktree path guard (gsd.worktree_path_guard): '{}' is inside a git internal (.git) directory, \
                     not the active worktree at '{}'. Writing to repository internals via an absolute path is not \
                     permitted from an isolated executor worktree. Use a relative path.",
                    p.real, git.toplevel
                ));
            }
            Some(top) if *top != git.toplevel => {
                return HookOutput::deny(format!(
                    "Worktree path guard (gsd.worktree_path_guard): '{}' resolves to git root '{top}' which differs \
                     from the active worktree root '{}'. This likely means an absolute path was derived from the \
                     orchestrator's main repository instead of the active worktree. To fix: use a relative path, or \
                     re-derive the base directory with `git rev-parse --show-toplevel` from within the worktree (hook \
                     cwd: '{}').",
                    p.real, git.toplevel, f.cwd
                ));
            }
            _ => {}
        }
    }
    HookOutput::default()
}

pub(super) fn declare(c: &mut ContributionsBuilder) {
    c.hook("worktree_path_guard", HookEvent::PreTool, |h| {
        h.description("Refuses a write from an agent's worktree to a path in another repository")
            .tools(&["write", "edit"])
            .reads_git()
            .handler(|call| Ok(worktree_path_guard(&Facts::of(call))));
    });
}

#[cfg(test)]
mod tests;
