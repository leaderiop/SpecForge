//! The one place for branch names and rules: which branches are an agent's and which are protected, for the hooks and
//! `branch_check`; the slug and the template a branch is named from.

use serde_json::Value;
use specforge_extension_sdk::prelude::*;

/// `text` as a slug: lower case, runs of other characters as one dash. A phase's directory and a branch share it.
pub fn slug(text: &str) -> String {
    let mut slug = String::new();
    for c in text.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    slug.trim_end_matches('-').to_string()
}

/// `template` with each `{key}` of `pairs` replaced; any other brace is kept as written.
pub fn fill(template: &str, pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .fold(template.to_string(), |filled, (key, value)| {
            filled.replace(&format!("{{{key}}}"), value)
        })
}

/// Whether `branch` is a GSD executor's: `agent-`, `worktree-agent-` or `worktree-wf_`, then `[A-Za-z0-9._/-]+`.
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
    fn the_slug_is_planning_configs() {
        assert_eq!(slug("User Auth & Login!"), "user-auth-login");
        assert_eq!(slug("--Déjà vu 2.0--"), "d-j-vu-2-0");
        assert_eq!(slug(""), "");
    }

    #[test]
    fn fill_replaces_each_key_and_keeps_the_rest() {
        let pairs = [("phase", "03"), ("slug", "auth")];
        assert_eq!(
            fill("gsd/phase-{phase}-{slug}/{phase}", &pairs),
            "gsd/phase-03-auth/03"
        );
        assert_eq!(fill("gsd/{other}-{slug}", &pairs), "gsd/{other}-auth");
        assert_eq!(fill("", &pairs), "");
    }

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
