//! The one place for branch names and rules: which branches are an agent's, for the hooks and `branch_check`.

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
