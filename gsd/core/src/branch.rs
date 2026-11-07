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

/// That the settings the manifest's text writes for this extension state what [`Protection`] reads, by its key. The
/// host sends a setting of another type as unset (E086, which only `validate` and doctor report), so the manifest is
/// the one place a broken override shows; read as no override, it would leave what it names unprotected.
pub fn check_written_protection(manifest: &str) -> Result<(), String> {
    let manifest: Value =
        serde_json::from_str(manifest).map_err(|e| format!("specforge.json: {e}"))?;
    let written = |key: &str| {
        manifest
            .pointer(&format!("/settings/@gsd~1core/{key}"))
            .filter(|value| !value.is_null())
    };
    if let Some(value) = written("git.protected_branches") {
        let names = value
            .as_array()
            .filter(|names| names.iter().all(Value::is_string));
        if names.is_none() {
            return Err(format!(
                "the setting git.protected_branches is {value}, not a list of branch names"
            ));
        }
    }
    if let Some(value) = written("git.base_branch").filter(|value| !value.is_string()) {
        return Err(format!(
            "the setting git.base_branch is {value}, not a branch name"
        ));
    }
    Ok(())
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
    /// (empty) is none of them. Case does not count: on a case-insensitive file system `MAIN` is `main`'s ref, and a
    /// commit on it moves `main`.
    pub fn covers(&self, branch: &str) -> bool {
        let same = |name: &&str| name.eq_ignore_ascii_case(branch);
        let release = "release/";
        !branch.is_empty()
            && (PROTECTED.iter().any(same)
                || branch.len() > release.len()
                    && branch
                        .get(..release.len())
                        .is_some_and(|start| start.eq_ignore_ascii_case(release))
                || self.listed.iter().any(same)
                || self.base.iter().any(same))
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
    fn a_written_protection_is_a_list_of_names_and_a_name() {
        let manifest = |settings: &str| format!(r#"{{"settings": {{"@gsd/core": {settings}}}}}"#);
        for fine in [
            r#"{}"#,
            r#"{"git.protected_branches": ["a", "b"]}"#,
            r#"{"git.protected_branches": []}"#,
            r#"{"git.protected_branches": null, "git.base_branch": null}"#,
            r#"{"git.base_branch": "stable"}"#,
        ] {
            assert_eq!(check_written_protection(&manifest(fine)), Ok(()), "{fine}");
        }
        for (broken, key) in [
            (
                r#"{"git.protected_branches": "feat"}"#,
                "git.protected_branches",
            ),
            (
                r#"{"git.protected_branches": ["feat", 1]}"#,
                "git.protected_branches",
            ),
            (
                r#"{"git.protected_branches": {"feat": true}}"#,
                "git.protected_branches",
            ),
            (r#"{"git.base_branch": 1}"#, "git.base_branch"),
        ] {
            let why = check_written_protection(&manifest(broken)).unwrap_err();
            assert!(why.contains(key), "{broken}: {why}");
        }
        assert_eq!(check_written_protection("{}"), Ok(()));
    }

    #[test]
    fn the_protected_set_compares_without_case() {
        let protection = Protection {
            listed: vec!["Shared/X"],
            base: Some("Stable"),
        };
        for branch in [
            "main",
            "MAIN",
            "Master",
            "TRUNK",
            "release/1.0",
            "RELEASE/1.0",
            "shared/x",
            "SHARED/X",
            "stable",
        ] {
            assert!(protection.covers(branch), "{branch}");
        }
        for branch in ["", "release/", "RELEASE/", "mainline", "feature/MAIN", "é"] {
            assert!(!protection.covers(branch), "{branch}");
        }
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
