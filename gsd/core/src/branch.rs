//! The one place for branch names and rules: which branches are an agent's and which are protected, for the hooks and
//! `branch_check`.

use serde_json::Value;
use specforge_extension_sdk::prelude::*;

/// Whether `branch` is a GSD executor's: upstream's `^((worktree-)?agent-|worktree-wf_)[A-Za-z0-9._/-]+$`.
pub fn agent_branch(branch: &str) -> bool {
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

/// The branches a commit needs a reason to land on, whatever the settings add.
pub const PROTECTED: &[&str] = &["main", "master", "develop", "trunk"];

/// What the settings add to [`PROTECTED`].
///
/// shortcut: origin/HEAD is not read (the git port answers no remotes); upgrade when it does.
#[derive(Debug, Default)]
pub struct Protection<'a> {
    /// `git.protected_branches`.
    pub listed: Vec<&'a str>,
    /// `git.base_branch`, when set.
    pub base: Option<&'a str>,
}

impl<'a> Protection<'a> {
    pub fn of(settings: &'a Settings) -> Self {
        Protection {
            listed: settings
                .get("git.protected_branches")
                .and_then(Value::as_array)
                .map(|names| names.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default(),
            base: settings.string("git.base_branch").filter(|b| !b.is_empty()),
        }
    }

    /// Whether `branch` is protected: one of [`PROTECTED`], under `release/`, listed, or the base. A detached `HEAD`
    /// (empty) is none of them.
    pub fn covers(&self, branch: &str) -> bool {
        !branch.is_empty()
            && (PROTECTED.contains(&branch)
                || branch
                    .strip_prefix("release/")
                    .is_some_and(|rest| !rest.is_empty())
                || self.listed.contains(&branch)
                || self.base == Some(branch))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_agent_namespace_is_one_rule() {
        for branch in [
            "agent-1",
            "worktree-agent-1",
            "worktree-wf_run1-2",
            "agent-a/b.c_d",
        ] {
            assert!(agent_branch(branch), "{branch}");
        }
        for branch in [
            "agent-",
            "worktree-agent-",
            "worktree-wf_",
            "agent-a b",
            "main",
            "",
            "x-agent-1",
        ] {
            assert!(!agent_branch(branch), "{branch}");
        }
    }
}
