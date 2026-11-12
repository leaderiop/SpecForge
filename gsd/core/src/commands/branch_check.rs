//! `branch_check`: whether the branch `HEAD` is on may take a commit, from the git port and the settings, over
//! the one rule in [`crate::branch`]. Read-only; git reads inside the project need no approval (ADR 0335 D2.4).
//!
//! - `[agent-namespace-one-rule]`: isolation's agent namespace, with `worktree-wf_`, for both hooks and
//!   this answer (upstream's workflow guard has two prefixes).
//! - `[protected-set]`: four names plus `release/…`, `git.protected_branches` and `git.base_branch`. The
//!   override changes `blocks_commit`, not `protected`, so ship's and the `none` strategy's warnings still show on
//!   `main`.
//!
//! A host that refuses the git port (a ceiling's `effects`) answers `unreadable` and `blocks_commit`. What to say about
//! a blocked commit is each asset's.

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::support::{answer, run};
use crate::branch::{Protection, agent_branch, check_written_protection};

/// The answer for the branch `HEAD` is on (empty: detached).
#[derive(Debug, Default, PartialEq)]
struct Judged {
    branch: String,
    detached: bool,
    protected: bool,
    agent: bool,
    /// The host could not say which branch `HEAD` is on: what the executor's guard tells from a protected one.
    unreadable: bool,
    blocks_commit: bool,
}

fn judge(branch: &str, protection: &Protection<'_>, allow: bool) -> Judged {
    let detached = branch.is_empty();
    let protected = protection.covers(branch);
    Judged {
        branch: branch.to_string(),
        detached,
        protected,
        agent: agent_branch(branch),
        unreadable: false,
        blocks_commit: detached || (protected && !allow),
    }
}

fn branch_check(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let manifest = call.files("manifest").and_then(|read| read.facts.first());
    if let Some(fact) = manifest {
        match &fact.text {
            Some(text) => check_written_protection(text)?,
            None => return Err("specforge.json cannot be read for the protected set".to_string()),
        }
    }
    let (judged, human) = match specforge_extension_sdk::host::git::branch() {
        Ok(branch) => {
            let settings = Settings::current();
            let allow = settings
                .bool("git.allow_default_branch_commits")
                .unwrap_or(false);
            let judged = judge(&branch, &Protection::of(&settings), allow);
            let human = match (judged.detached, judged.blocks_commit) {
                (true, _) => "blocks a commit: HEAD is detached".to_string(),
                (false, true) => format!("blocks a commit: {branch} is protected"),
                (false, false) => format!("ok: {branch}"),
            };
            (judged, human)
        }
        Err(error) => (
            Judged {
                unreadable: true,
                blocks_commit: true,
                ..Judged::default()
            },
            format!("blocks a commit: the branch cannot be read: {error:?}"),
        ),
    };
    Ok(answer(
        call,
        json!({
            "branch": judged.branch,
            "detached": judged.detached,
            "protected": judged.protected,
            "agent": judged.agent,
            "unreadable": judged.unreadable,
            "blocks_commit": judged.blocks_commit,
        }),
        human,
        None,
    ))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.capability(
        Capability::git("repo")
            .why("read the branch HEAD is on, and the history gsd-git records commits from"),
    );
    c.command("branch_check", |cmd| {
        cmd.description("Answer whether the branch HEAD is on may take a commit: protected, an agent's, overridden. Read-only")
            .input(Input::files("manifest", FileReadSpec::glob("specforge.json").text()))
            .handler(|call| run(call, branch_check));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none() -> Protection<'static> {
        Protection::default()
    }

    #[test]
    fn a_protected_branch_blocks_a_commit_unless_overridden() {
        let refused = judge("main", &none(), false);
        assert!(refused.protected && refused.blocks_commit);
        let allowed = judge("main", &none(), true);
        assert!(allowed.protected && !allowed.blocks_commit);
        let free = judge("feature/x", &none(), false);
        assert!(!free.protected && !free.blocks_commit);
    }

    #[test]
    fn the_listed_and_base_branches_are_protected() {
        let protection = Protection {
            listed: vec!["shared/x"],
            base: Some("stable"),
        };
        for branch in ["shared/x", "stable", "trunk", "release/1.0"] {
            assert!(judge(branch, &protection, false).protected, "{branch}");
        }
        assert!(!judge("shared/y", &protection, false).protected);
    }

    #[test]
    fn detached_always_blocks() {
        for allow in [false, true] {
            let judged = judge("", &none(), allow);
            assert!(judged.detached && judged.blocks_commit);
        }
    }

    #[test]
    fn the_agent_namespace_is_answered_beside_the_rest() {
        for branch in ["agent-1", "worktree-wf_run-1"] {
            assert!(judge(branch, &none(), false).agent, "{branch}");
        }
        // A free branch is no agent's, and the override does not open the namespace.
        assert!(!judge("feature/x", &none(), false).agent);
        assert!(!judge("main", &none(), true).agent);
    }
}
