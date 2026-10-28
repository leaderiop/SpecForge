//! What a hook is told that its sandbox cannot read (ADR 0292): the paths its tool names with what the disk says of
//! them, a shell tool's command as words, and the commit a `git commit` makes.

use std::path::{Path, PathBuf};

use serde_json::Value;
use specforge_protocol_types::{CommitFact, PathFact};

use crate::host::shell;

/// The paths a tool's input names: `file_path`, `path`, `notebook_path`, each of `paths` and each file a patch text
/// names.
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
    if let Some(patch) = super::patch::text_of(input) {
        out.extend(super::patch::paths(patch));
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

/// The `git commit` a command line makes: its message (`-m` or `--message`, alone or in a cluster of short options such
/// as `-am`) and whether it amends; none when no command of the line is a commit. A message the commit takes from
/// elsewhere (`-F`, `-C`, `-c`, `--fixup`, `--squash`, the editor) is none, not empty.
// shortcut: `git commit` is found by its words (global options and the wrappers `env`, `command`, `sudo`, `exec`,
// `nohup`, `time` skipped, `sh -c '…'` read); an alias, a script, `xargs` or `$(…)` is not seen.
pub fn commit(input: &Value) -> Option<CommitFact> {
    commit_of_line(input.get("command")?.as_str()?, 0)
}

fn commit_of_line(line: &str, depth: usize) -> Option<CommitFact> {
    let commands = shell::commands(line).ok()?;
    for words in &commands {
        if let Some(inner) = shell_c(words).filter(|_| depth < 3) {
            if let Some(fact) = commit_of_line(inner, depth + 1) {
                return Some(fact);
            }
        } else if let Some(args) = commit_args(words) {
            return Some(commit_fact(args));
        }
    }
    None
}

/// The script of `sh -c '…'` (or bash, zsh), the wrappers skipped.
fn shell_c(words: &[String]) -> Option<&str> {
    let mut it = words.iter().skip_while(|w| wrapper(w));
    let name = it.next()?.rsplit('/').next()?;
    if !matches!(name, "sh" | "bash" | "zsh") {
        return None;
    }
    let flag = |w: &String| w.starts_with('-') && !w.starts_with("--");
    let mut rest = it.skip_while(|w| flag(w) && !w.ends_with('c'));
    rest.next().filter(|w| flag(w) && w.ends_with('c'))?;
    rest.next().map(String::as_str)
}

fn wrapper(word: &str) -> bool {
    (word.contains('=') && !word.starts_with('-'))
        || matches!(
            word,
            "env" | "command" | "sudo" | "exec" | "nohup" | "time" | "nice"
        )
}

fn commit_fact(words: &[String]) -> CommitFact {
    let mut message: Vec<String> = Vec::new();
    let mut elsewhere = false;
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
            "--message" => message.extend(value(&mut at)),
            "--file" | "--reuse-message" | "--reedit-message" | "--fixup" | "--squash" => {
                elsewhere = true;
                at += 1;
            }
            w if w.starts_with("--message=") => message.push(w["--message=".len()..].to_string()),
            w if w.starts_with("--file=")
                || w.starts_with("--reuse-message=")
                || w.starts_with("--reedit-message=")
                || w.starts_with("--fixup=")
                || w.starts_with("--squash=") =>
            {
                elsewhere = true;
            }
            w if w.starts_with('-') && !w.starts_with("--") => {
                // A cluster of short options: the first that takes a value ends it.
                for (i, c) in w.char_indices().skip(1) {
                    match c {
                        'm' => {
                            let rest = &w[i + 1..];
                            if rest.is_empty() {
                                message.extend(value(&mut at));
                            } else {
                                message.push(rest.to_string());
                            }
                            break;
                        }
                        'F' | 'C' | 'c' => {
                            elsewhere = true;
                            if w[i + 1..].is_empty() {
                                at += 1;
                            }
                            break;
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        at += 1;
    }
    CommitFact {
        message: (!elsewhere && !message.is_empty()).then(|| message.join("\n\n")),
        amend,
    }
}

/// The words after `commit` when `words` is `git [global options] commit …`, the wrappers skipped.
fn commit_args(words: &[String]) -> Option<&[String]> {
    let start = words.iter().take_while(|w| wrapper(w)).count();
    let mut it = words.iter().enumerate().skip(start);
    if it.next().map(|(_, w)| w.as_str()) != Some("git") {
        return None;
    }
    while let Some((at, word)) = it.next() {
        match word.as_str() {
            "-C" | "-c" | "--git-dir" | "--work-tree" | "--namespace" => {
                it.next();
            }
            w if w.starts_with('-') => {}
            "commit" => return Some(&words[at + 1..]),
            _ => return None,
        }
    }
    None
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
        assert_eq!(
            (fact.message.as_deref(), fact.amend),
            (Some("feat(x): y"), true)
        );
        assert_eq!(
            commit_of("git -C sub commit --message=wip")
                .unwrap()
                .message
                .as_deref(),
            Some("wip")
        );
        assert_eq!(
            commit_of("cd x && git commit -m 'a b'")
                .unwrap()
                .message
                .as_deref(),
            Some("a b")
        );
        assert_eq!(
            commit_of("git commit -m a -m b")
                .unwrap()
                .message
                .as_deref(),
            Some("a\n\nb")
        );
    }

    #[test]
    fn clustered_options_wrappers_and_unread_messages_are_told_apart() {
        let message = |line: &str| commit_of(line).unwrap().message;
        for line in [
            "git commit -am 'wip'",
            "git commit -sm wip",
            "git commit -qam wip",
            "git commit -S -m wip",
            "git commit -amwip",
            "env GIT_X=1 git commit -m wip",
            "sudo git commit -m wip",
            "sh -c 'git add . && git commit -m wip'",
            "bash -lc \"git commit -am wip\"",
        ] {
            assert_eq!(message(line).as_deref(), Some("wip"), "{line}");
        }
        for line in [
            "git commit -F msg.txt",
            "git commit -C HEAD",
            "git commit --fixup=abc",
            "git commit -e",
            "git commit",
        ] {
            assert_eq!(message(line), None, "{line}");
        }
        assert!(commit_of("git commit --amend -m x").unwrap().amend);
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
