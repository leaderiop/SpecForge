//! Ported from gsd-core v1.18.0 hooks/gsd-agent-isolation-guard.js and hooks/gsd-worktree-path-guard.js, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! `worktree_path_guard` (D-r44-10), a heuristic where upstream reads files and git. There is no `agent_isolation_guard`.
//!
//! Upstream resolves the dispatch isolation from a sentinel file, the runtime registry and `.planning/config.json`, and
//! finds a GSD worktree from `git rev-parse` and the branch name. A hook here reads none of them, so:
//! - there is no isolation guard. Upstream denies an executor without `isolation:"worktree"` only when the isolation is
//!   `harness-worktree`, and in sequential mode (`USE_WORKTREES=false`) the sentinel says `none` and it allows.
//!   @gsd/core has no worktree isolation: its execute workflows set `USE_WORKTREES="false"` and tell the agent to omit
//!   `isolation`, so every dispatch it makes is the sequential one (REVIEW-r44 C1). A hook that always allows would
//!   cost every `Agent`/`Task` dispatch a process, so none is installed (r44 post-fix N7) until the `specforged`
//!   harness runs executors in worktrees;
//! - the path guard knows a GSD worktree by where it is: a project under `<repo>/.claude/worktrees/agent-*` (Claude
//!   Code's own place for a subagent's worktree, whose branch is upstream's `worktree-agent-*`) or `wf_*`. It refuses
//!   a path into `<repo>`, the main checkout, which is the bug it exists for (#260), in any case and as a relative
//!   path that climbs into it (stricter than upstream, which allows every relative path: ACCEPTANCE's divergences); upstream lets a path outside every repository through, and so does this one. A path that reaches `<repo>`
//!   through a symlink is not seen (the kernel hands a hook the name, not the real path: K20), where upstream asks
//!   git for the toplevel of the target's real directory.

use specforge_extension_sdk::prelude::*;

use super::facts::Facts;

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

/// Whether `path` is `root` or inside it, ignoring case: APFS and NTFS name one directory with several spellings.
fn under(path: &str, root: &str) -> bool {
    let (path, root) = (path.to_lowercase(), root.to_lowercase());
    path == root || path.strip_prefix(&root).is_some_and(|r| r.starts_with('/'))
}

/// Refuses an absolute path into the main checkout from inside an agent's worktree.
pub(super) fn worktree_path_guard(f: &Facts) -> HookOutput {
    if !matches!(f.class.as_str(), "write" | "edit") {
        return HookOutput::default();
    }
    let Some((main, worktree)) = agent_worktree(&f.cwd) else {
        return HookOutput::default();
    };
    for p in f.paths.iter().filter(|p| !p.in_project) {
        // A relative path leaves the project from the host's directory (`../../../src/a.ts`).
        let named = if p.path.starts_with('/') {
            p.path.clone()
        } else {
            format!("{}/{}", f.cwd.replace('\\', "/"), p.path)
        };
        let path = resolved(&named);
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
