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

/// The most bytes of one path component of a branch name: a file name's 255, less the `.lock` git writes it beside.
const COMPONENT_MAX: usize = 250;

/// The most characters of a slug in a branch name: a title is prose, and a name has [`COMPONENT_MAX`].
const SLUG_MAX: usize = 64;

/// `slug` cut to what a branch name takes, without a dash or a dot at its end.
pub fn shortened(slug: &str) -> &str {
    let end = slug
        .char_indices()
        .nth(SLUG_MAX)
        .map_or(slug.len(), |(at, _)| at);
    match slug[..end].trim_end_matches(['-', '.']) {
        trimmed if end < slug.len() => trimmed,
        _ => slug,
    }
}

/// Why git refuses `name` as a new branch, `None` when it takes it: the rule of `git check-ref-format --branch`, and a
/// file name's limit, which that rule leaves to the switch that fails on it.
///
/// shortcut: restates git's rule, as no ref validator is reachable from this workspace (gix stays behind the host's
/// git port); upgrade when that port answers it.
pub fn refused(name: &str) -> Option<&'static str> {
    if name.is_empty() {
        return Some("it is empty");
    }
    if name.starts_with('-') {
        return Some("it starts with a dash, which a command reads as an option");
    }
    if name == "HEAD" || name == "@" {
        return Some("git keeps that name");
    }
    if name.contains("..") || name.contains("@{") || name.ends_with('.') {
        return Some("it holds `..` or `@{`, or ends with a dot");
    }
    let forbidden = |c: char| c.is_ascii_control() || " ~^:?*[\\".contains(c);
    if name.contains(forbidden) {
        return Some("it holds a space, a control character or one of ~^:?*[\\");
    }
    name.split('/').find_map(|component| {
        if component.is_empty() {
            Some("it has an empty path component")
        } else if component.starts_with('.') || component.ends_with(".lock") {
            Some("a path component starts with a dot or ends with `.lock`")
        } else if component.len() > COMPONENT_MAX {
            Some("a path component is too long for a file name")
        } else {
            None
        }
    })
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

    /// Git's own answer when it is on the machine, so the rule cannot drift from it unseen.
    fn git_takes(name: &str) -> Option<bool> {
        std::process::Command::new("git")
            .args(["check-ref-format", "--branch", name])
            .output()
            .ok()
            .map(|out| out.status.success())
    }

    #[test]
    fn the_name_rule_is_gits() {
        for name in [
            "gsd/phase-01-core",
            "feature/x",
            "v1.0",
            "a.b",
            "x;touch",
            "a-",
            "a@b",
            "déjà",
            "{slug}",
        ] {
            assert_eq!(refused(name), None, "{name}");
            assert_ne!(git_takes(name), Some(false), "git refuses {name}");
        }
        for name in [
            "x;touch pwned",
            "a..b",
            "y.lock",
            "y.lock/z",
            "-evil01",
            "--help",
            "a b",
            "a\tb",
            "a\u{7f}b",
            "a~b",
            "a^b",
            "a:b",
            "a?b",
            "a*b",
            "a[b",
            "a\\b",
            "/a",
            "a/",
            "a//b",
            ".a",
            "a/.b",
            "a.",
            "a/b.",
            "a@{b",
            "HEAD",
            "",
        ] {
            assert!(refused(name).is_some(), "{name:?}");
            assert_ne!(git_takes(name), Some(true), "git takes {name:?}");
        }
        // `@` is `HEAD`'s other name: git's answer reads the checkout it runs in.
        assert!(refused("@").is_some());
        // Git's rule has no length: a file name has, and a ref is one, with `.lock` beside it while it is written.
        assert_eq!(refused(&"a".repeat(250)), None);
        assert!(refused(&"a".repeat(251)).is_some());
        assert_eq!(refused(&format!("{0}/{0}", "a".repeat(250))), None);
    }

    #[test]
    fn a_long_slug_shortens() {
        assert_eq!(shortened("core"), "core");
        assert_eq!(shortened(""), "");
        let long = "a".repeat(400);
        assert_eq!(shortened(&long).len(), 64);
        let dash = format!("{}-b", "a".repeat(63));
        assert_eq!(shortened(&dash), "a".repeat(63), "no trailing dash");
        let dot = format!("{}.b", "a".repeat(63));
        assert_eq!(shortened(&dot), "a".repeat(63), "no trailing dot");
        assert_eq!(shortened(&"é".repeat(100)).chars().count(), 64);
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
