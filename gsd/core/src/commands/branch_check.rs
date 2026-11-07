//! `branch_check [--agent]`: whether the branch `HEAD` is on may take a commit, from the git port and the settings, over
//! the one rule in [`crate::branch`]. Read-only; git reads inside the project need no approval (ADR 0335 D2.4).
//!
//! - `[agent-namespace-one-rule]`: isolation's agent namespace, with `worktree-wf_`, for both hooks and
//!   this answer (upstream's workflow guard has two prefixes).
//! - `[protected-set]`: four names plus `release/…`, `git.protected_branches` and `git.base_branch`. The
//!   override changes `blocks_commit`, not `protected`, so ship's and the `none` strategy's warnings still show on
//!   `main`.
//!
//! A host that refuses the git port (a ceiling's `effects`) answers `blocks_commit: true` with its error as `why`.

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
    overridden: bool,
    /// The host could not say which branch `HEAD` is on: what the executor's guard tells from a protected one.
    unreadable: bool,
    blocks_commit: bool,
    why: Option<String>,
}

fn judge(branch: &str, protection: &Protection<'_>, allow: bool, agent_mode: bool) -> Judged {
    let detached = branch.is_empty();
    let protected = protection.covers(branch);
    let agent = agent_branch(branch);
    let overridden = protected && allow;
    let blocks_commit = detached
        || if agent_mode {
            !agent
        } else {
            protected && !overridden
        };
    let why = blocks_commit.then(|| {
        if detached {
            "refusing to commit — HEAD is detached.".to_string()
        } else if agent_mode {
            format!(
                "refusing to commit — worktree HEAD '{branch}' is not in the agent-* / worktree-agent-* / worktree-wf_* namespace."
            )
        } else {
            format!("refusing to commit — HEAD is on '{branch}' (protected/default branch).")
        }
    });
    Judged {
        branch: branch.to_string(),
        detached,
        protected,
        agent,
        overridden,
        unreadable: false,
        blocks_commit,
        why,
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
    let judged = match specforge_extension_sdk::host::git::branch() {
        Ok(branch) => {
            let settings = Settings::current();
            let allow = settings
                .bool("git.allow_default_branch_commits")
                .unwrap_or(false);
            judge(
                &branch,
                &Protection::of(&settings),
                allow,
                call.flag("agent"),
            )
        }
        Err(error) => Judged {
            unreadable: true,
            blocks_commit: true,
            why: Some(format!(
                "refusing to commit — the branch cannot be read: {error:?}"
            )),
            ..Judged::default()
        },
    };
    let human = judged
        .why
        .clone()
        .unwrap_or_else(|| format!("ok: {}", judged.branch));
    Ok(answer(
        call,
        json!({
            "branch": judged.branch,
            "detached": judged.detached,
            "protected": judged.protected,
            "agent": judged.agent,
            "overridden": judged.overridden,
            "unreadable": judged.unreadable,
            "blocks_commit": judged.blocks_commit,
            "why": judged.why,
        }),
        human,
        None,
    ))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.capability(Capability::git("repo").why("read the branch HEAD is on"));
    c.command("branch_check", |cmd| {
        cmd.description("Answer whether the branch HEAD is on may take a commit: protected, an agent's, overridden. Read-only")
            .arg("agent", |a| {
                a.flag()
                    .description("this is an agent's worktree: only the agent namespace may commit");
            })
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
        let refused = judge("main", &none(), false, false);
        assert!(refused.protected && refused.blocks_commit && !refused.overridden);
        assert!(refused.why.expect("why").contains("'main'"));
        let allowed = judge("main", &none(), true, false);
        assert!(allowed.protected && allowed.overridden && !allowed.blocks_commit);
        assert_eq!(allowed.why, None);
        let free = judge("feature/x", &none(), false, false);
        assert!(!free.protected && !free.blocks_commit);
    }

    #[test]
    fn the_listed_and_base_branches_are_protected() {
        let protection = Protection {
            listed: vec!["shared/x"],
            base: Some("stable"),
        };
        for branch in ["shared/x", "stable", "trunk", "release/1.0"] {
            assert!(
                judge(branch, &protection, false, false).protected,
                "{branch}"
            );
        }
        assert!(!judge("shared/y", &protection, false, false).protected);
    }

    #[test]
    fn detached_always_blocks() {
        for (allow, agent_mode) in [(false, false), (true, false), (true, true)] {
            let judged = judge("", &none(), allow, agent_mode);
            assert!(judged.detached && judged.blocks_commit);
            assert!(judged.why.expect("why").contains("detached"));
        }
    }

    #[test]
    fn the_agent_mode_requires_the_namespace() {
        assert!(!judge("agent-1", &none(), false, true).blocks_commit);
        assert!(!judge("worktree-wf_run-1", &none(), false, true).blocks_commit);
        // A free branch is no agent's, and the override does not open the namespace.
        assert!(judge("feature/x", &none(), false, true).blocks_commit);
        assert!(judge("main", &none(), true, true).blocks_commit);
    }
}
