//! What a hook is told that its sandbox cannot read (ADR 0292): the paths its tool names with what the disk says of
//! them, a shell tool's command as words, and the commit a `git commit` makes.

use std::path::{Path, PathBuf};

use serde_json::Value;
use specforge_protocol_types::{CommitFact, PathFact};

use crate::host::shell;

/// The paths a tool's input names: `file_path`, `path`, `notebook_path`, and each of `paths`.
pub fn path_names(input: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for key in ["file_path", "path", "notebook_path"] {
        if let Some(path) = input.get(key).and_then(Value::as_str) {
            out.push(path.to_string());
        }
    }
    if let Some(paths) = input.get("paths").and_then(Value::as_array) {
        out.extend(paths.iter().filter_map(Value::as_str).map(str::to_string));
    }
    out
}

/// `path` made absolute against `cwd`, with `.` and `..` resolved and, as far as the path exists, its links.
fn absolute(cwd: &Path, path: &str) -> PathBuf {
    let joined = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        cwd.join(path)
    };
    let mut lexical = PathBuf::new();
    for part in joined.components() {
        match part {
            std::path::Component::ParentDir => {
                lexical.pop();
            }
            std::path::Component::CurDir => {}
            other => lexical.push(other),
        }
    }
    if let Ok(real) = std::fs::canonicalize(&lexical) {
        return real;
    }
    match (
        lexical.parent().map(std::fs::canonicalize),
        lexical.file_name(),
    ) {
        (Some(Ok(parent)), Some(name)) => parent.join(name),
        _ => lexical,
    }
}

/// `path` relative to `root` with `/`, when it lies inside.
pub fn relative_to(root: &Path, cwd: &Path, path: &str) -> Option<String> {
    let real_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let inside = absolute(cwd, path);
    let rel = inside.strip_prefix(&real_root).ok()?;
    Some(rel.to_string_lossy().replace('\\', "/"))
}

/// The fact about each path `input` names. `spec_root` is the spec directory relative to `root` (empty: the root).
pub fn path_facts(root: &Path, cwd: &Path, spec_root: &str, input: &Value) -> Vec<PathFact> {
    let new_bytes = input
        .get("content")
        .and_then(Value::as_str)
        .map(|c| c.len() as u64);
    path_names(input)
        .into_iter()
        .map(|name| match relative_to(root, cwd, &name) {
            Some(rel) => {
                let meta = std::fs::metadata(root.join(&rel))
                    .ok()
                    .filter(|m| m.is_file());
                let in_spec_root = spec_root.is_empty()
                    || rel == spec_root
                    || rel.starts_with(&format!("{spec_root}/"));
                PathFact {
                    path: rel,
                    in_project: true,
                    in_spec_root,
                    exists: meta.is_some(),
                    bytes: meta.map(|m| m.len()),
                    new_bytes,
                }
            }
            None => PathFact {
                path: name,
                in_project: false,
                in_spec_root: false,
                exists: false,
                bytes: None,
                new_bytes,
            },
        })
        .collect()
}

/// The words of a shell tool's `command`; none when the tool has none or it cannot be read as shell.
pub fn shell_words(input: &Value) -> Option<Vec<String>> {
    shell::words(input.get("command")?.as_str()?).ok()
}

/// The `git commit` a command line makes: its message (`-m` or `--message`; a `-F` file is not read) and whether it amends; none when no command of the line is a commit.
// shortcut: `git commit` is found by its words (global options skipped); an alias or a script is not seen.
pub fn commit(input: &Value) -> Option<CommitFact> {
    let line = input.get("command")?.as_str()?;
    let commands = shell::commands(line).ok()?;
    let words = commands.iter().find(|c| is_commit(c))?;
    let mut message = Vec::new();
    let mut amend = false;
    let mut at = 0;
    while at < words.len() {
        let word = words[at].as_str();
        let value = |at: &mut usize| {
            *at += 1;
            words.get(*at).cloned()
        };
        match word {
            "--amend" => amend = true,
            "-m" | "--message" => message.extend(value(&mut at)),
            // The message file is not a project file: it is skipped, its message left empty.
            "-F" | "--file" => at += 1,
            w if w.starts_with("--message=") => message.push(w["--message=".len()..].to_string()),
            w if w.starts_with("-m") && w.len() > 2 && !w.starts_with("--") => {
                message.push(w[2..].to_string());
            }
            _ => {}
        }
        at += 1;
    }
    Some(CommitFact {
        message: message.join("\n\n"),
        amend,
    })
}

/// Whether `words` is `git [global options] commit …`.
fn is_commit(words: &[String]) -> bool {
    let mut it = words
        .iter()
        .skip_while(|w| w.contains('=') && !w.starts_with('-'));
    if it.next().map(String::as_str) != Some("git") {
        return false;
    }
    while let Some(word) = it.next() {
        match word.as_str() {
            "-C" | "-c" | "--git-dir" | "--work-tree" | "--namespace" => {
                it.next();
            }
            w if w.starts_with('-') => {}
            w => return w == "commit",
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn commit_of(command: &str) -> Option<CommitFact> {
        commit(&json!({"command": command}))
    }

    #[test]
    fn a_git_commit_is_found_with_its_message() {
        let fact = commit_of("git commit -m \"feat(x): y\" --amend").unwrap();
        assert_eq!((fact.message.as_str(), fact.amend), ("feat(x): y", true));
        assert_eq!(
            commit_of("git -C sub commit --message=wip")
                .unwrap()
                .message,
            "wip"
        );
        assert_eq!(
            commit_of("cd x && git commit -m 'a b'").unwrap().message,
            "a b"
        );
        assert_eq!(commit_of("git commit -m a -m b").unwrap().message, "a\n\nb");
    }

    #[test]
    fn what_is_no_commit_is_none() {
        for line in [
            "git status",
            "git log --grep commit",
            "echo git commit",
            "git",
        ] {
            assert!(commit_of(line).is_none(), "{line}");
        }
    }

    #[test]
    fn a_path_outside_the_project_is_named_as_given_and_never_looked_at() {
        let dir = tempfile::tempdir().unwrap();
        let facts = path_facts(
            dir.path(),
            dir.path(),
            "",
            &json!({"file_path": "/etc/hosts", "content": "x"}),
        );
        assert_eq!(facts.len(), 1);
        assert!(!facts[0].in_project && !facts[0].exists && facts[0].bytes.is_none());
        assert_eq!(facts[0].path, "/etc/hosts");
    }
}
