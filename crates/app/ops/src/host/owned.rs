//! The files install may write (ADR 0292): the four host layouts, project-local, inside the root. Nothing else is
//! ever written, moved or removed by install, uninstall, or the rollback of an interrupted one.

use std::path::{Component, Path};

/// The config files install merges entries into.
const CONFIGS: [&str; 6] = [
    ".mcp.json",
    ".claude/settings.json",
    ".codex/config.toml",
    ".codex/hooks.json",
    ".cursor/mcp.json",
    ".cursor/hooks.json",
];

/// A file name an extension's id gives: kebab-case words.
fn name(word: &str) -> bool {
    !word.is_empty()
        && !word.starts_with('-')
        && word
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Whether `path` is one of the config files install merges into: files a host runs code from.
pub fn config_path(path: &str) -> bool {
    CONFIGS.contains(&path)
}

/// Whether `path` (project-relative, `/`-separated) is a file of one of the host layouts, with no `..`, no absolute
/// root and no empty segment.
pub fn owned_path(path: &str) -> bool {
    let relative = Path::new(path);
    if relative.has_root()
        || !relative
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
        || path.contains('\\')
        || path.ends_with('/')
        || path.contains("//")
    {
        return false;
    }
    if CONFIGS.contains(&path) {
        return true;
    }
    let parts: Vec<&str> = path.split('/').collect();
    let agent = |dir: &str, ext: &str| {
        parts.len() == 3
            && format!("{}/{}", parts[0], parts[1]) == dir
            && parts[2].strip_suffix(ext).is_some_and(name)
    };
    let skill = |dir: &str| {
        parts.len() == 4
            && format!("{}/{}", parts[0], parts[1]) == dir
            && name(parts[2])
            && parts[3] == "SKILL.md"
    };
    agent(".claude/agents", ".md")
        || agent(".codex/agents", ".toml")
        || agent(".cursor/agents", ".md")
        || skill(".claude/skills")
        || skill(".codex/skills")
        || skill(".cursor/skills")
}

/// The first component of `relative` below `root` that is a link (a dangling one too), if any: install, uninstall and
/// the state store never write or delete through one, since a link can lead outside the project (a `.claude` that is
/// the user's dotfiles directory).
pub fn link_in(root: &Path, relative: &str) -> Option<std::path::PathBuf> {
    let mut at = root.to_path_buf();
    for part in relative.split('/').filter(|p| !p.is_empty()) {
        at.push(part);
        match std::fs::symlink_metadata(&at) {
            Ok(meta) if meta.file_type().is_symlink() => return Some(at),
            Ok(_) => {}
            // Nothing there, so nothing below it is a link.
            Err(_) => return None,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::owned_path;

    #[test]
    fn only_the_host_layouts_are_owned() {
        for owned in [
            ".mcp.json",
            ".claude/settings.json",
            ".claude/agents/t-planner.md",
            ".claude/skills/t-ns-context/SKILL.md",
            ".codex/agents/t-planner.toml",
            ".codex/skills/x/SKILL.md",
            ".cursor/agents/a.md",
            ".cursor/hooks.json",
        ] {
            assert!(owned_path(owned), "{owned}");
        }
        for not in [
            "",
            "m.spec",
            "../.mcp.json",
            "/abs/.mcp.json",
            ".claude/agents/../settings.json",
            ".claude/agents/T.md",
            ".claude/agents/a/b.md",
            ".claude/skills/x/OTHER.md",
            ".claude/agents/x.toml",
            ".codex/agents/x.md",
            ".claude/commands/x.md",
            ".claude//agents/x.md",
            ".specforge/write.journal",
            "specforge.json",
        ] {
            assert!(!owned_path(not), "{not}");
        }
    }
}
