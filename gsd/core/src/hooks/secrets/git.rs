//! Ported from gsd-core v1.18.0 hooks/gsd-secret-read-guard.js (the git, container and copy readers of `findSecretRead`), MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! What git, a container runtime and `cp`/`mv` take that is only a name: a pathspec, an `--env-file` value, a copy's
//! target. And the command-valued `git -c` keys (`alias.x=!…`, `diff.external`), whose value is a script to scan.

use super::scan::find_secret_read;
use super::shell::Token;

const CONTAINER_RUNTIMES: [&str; 4] = ["docker", "docker-compose", "podman", "nerdctl"];
const COPY_NAME_ONLY_FLAGS: [&str; 8] = [
    "-f",
    "--force",
    "-i",
    "--interactive",
    "-n",
    "--no-clobber",
    "-v",
    "--verbose",
];
const GIT_GLOBAL_VALUE_OPTIONS: [&str; 8] = [
    "-C",
    "-c",
    "--git-dir",
    "--work-tree",
    "--namespace",
    "--config-env",
    "--attr-source",
    "--shallow-file",
];
const GIT_GLOBAL_EQUALS_FORM_OPTIONS: [&str; 6] = [
    "--git-dir",
    "--work-tree",
    "--namespace",
    "--config-env",
    "--attr-source",
    "--exec-path",
];
const GIT_GLOBAL_FLAGS: [&str; 13] = [
    "-p",
    "--paginate",
    "-P",
    "--no-pager",
    "--no-replace-objects",
    "--no-lazy-fetch",
    "--no-optional-locks",
    "--no-advice",
    "--bare",
    "--literal-pathspecs",
    "--glob-pathspecs",
    "--noglob-pathspecs",
    "--icase-pathspecs",
];
const CHECK_IGNORE_FLAGS: &[&str] = &[
    "-q",
    "--quiet",
    "-v",
    "--verbose",
    "--stdin",
    "-z",
    "-n",
    "--non-matching",
    "--no-index",
    "--index",
];
const LS_FILES_FLAGS: &[&str] = &[
    "-z",
    "-t",
    "-v",
    "-f",
    "-c",
    "--cached",
    "-d",
    "--deleted",
    "-m",
    "--modified",
    "-o",
    "--others",
    "-i",
    "--ignored",
    "-s",
    "--stage",
    "-k",
    "--killed",
    "-u",
    "--unmerged",
    "--directory",
    "--eol",
    "--no-empty-directory",
    "--resolve-undo",
    "--exclude-standard",
    "--full-name",
    "--recurse-submodules",
    "--error-unmatch",
    "--abbrev",
    "--debug",
    "--deduplicate",
    "--sparse",
];
const RM_FLAGS: &[&str] = &[
    "--cached",
    "-f",
    "--force",
    "-n",
    "--dry-run",
    "-q",
    "--quiet",
    "-r",
    "--ignore-unmatch",
    "--sparse",
];

/// `(sub, configs)`: the index of the git subcommand (none when an unknown option hides it) and every `-c` value
/// before it.
fn walk_git_globals(operands: &[&Token]) -> (Option<usize>, Vec<String>) {
    let mut configs = Vec::new();
    let mut k = 0;
    while k < operands.len() {
        let t = operands[k].text.as_str();
        if !t.starts_with('-') {
            return (Some(k), configs);
        }
        if GIT_GLOBAL_VALUE_OPTIONS.contains(&t) {
            if t == "-c" && k + 1 < operands.len() {
                configs.push(operands[k + 1].text.clone());
            }
            k += 2;
            continue;
        }
        if GIT_GLOBAL_FLAGS.contains(&t) {
            k += 1;
            continue;
        }
        if t.starts_with("--")
            && t.split_once('=')
                .is_some_and(|(o, _)| GIT_GLOBAL_EQUALS_FORM_OPTIONS.contains(&o))
        {
            k += 1;
            continue;
        }
        let mut j = k + 1;
        while j + 1 < operands.len() {
            if operands[j].text == "-c" {
                j += 1;
                configs.push(operands[j].text.clone());
            }
            j += 1;
        }
        return (None, configs);
    }
    (None, configs)
}

pub(super) fn scan_git_config(operands: &[&Token], depth: usize) -> Option<String> {
    for config in walk_git_globals(operands).1 {
        let Some((key, value)) = config.split_once('=') else {
            continue;
        };
        let key = key.to_lowercase();
        let script = if key.starts_with("alias.") && value.starts_with('!') {
            &value[1..]
        } else if key == "diff.external" {
            value
        } else {
            continue;
        };
        if let Some(hit) = find_secret_read(script, depth + 1) {
            return Some(hit);
        }
    }
    None
}

fn is_listed_flag(text: &str, flags: &[&str]) -> bool {
    if text.starts_with("--") {
        return flags.contains(&text);
    }
    text.len() > 1
        && text[1..].chars().all(|c| c.is_ascii_alphabetic())
        && text.starts_with('-')
        && text[1..]
            .chars()
            .all(|c| flags.contains(&format!("-{c}").as_str()))
}

/// The indices of the operands that are only names: a container's `--env-file` value, a git pathspec, a copy's target.
pub(super) fn name_only(base: &str, operands: &[&Token]) -> Vec<usize> {
    if CONTAINER_RUNTIMES.contains(&base) {
        let mut out = Vec::new();
        let mut k = 0;
        while k < operands.len() {
            let t = &operands[k].text;
            if t == "--env-file" || t.starts_with("--env-file=") {
                out.push(k);
                if !t.contains('=') {
                    k += 1;
                    out.push(k);
                }
            }
            k += 1;
        }
        out
    } else if base == "git" {
        git_pathspecs(operands)
    } else if base == "cp" || base == "mv" {
        let mut positional = Vec::new();
        let mut end_of_options = false;
        for (k, o) in operands.iter().enumerate() {
            let t = o.text.as_str();
            if end_of_options || !t.starts_with('-') || t == "-" {
                positional.push(k);
            } else if !positional.is_empty() {
                return Vec::new();
            } else if t == "--" {
                end_of_options = true;
            } else if !is_listed_flag(t, &COPY_NAME_ONLY_FLAGS) {
                return Vec::new();
            }
        }
        if positional.len() == 2 {
            vec![positional[1]]
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    }
}

fn git_pathspecs(operands: &[&Token]) -> Vec<usize> {
    let Some(sub) = walk_git_globals(operands).0 else {
        return Vec::new();
    };
    let (flags, requires): (&[&str], Option<&str>) = match operands[sub].text.as_str() {
        "check-ignore" => (CHECK_IGNORE_FLAGS, None),
        "ls-files" => (LS_FILES_FLAGS, None),
        "rm" => (RM_FLAGS, Some("--cached")),
        _ => return Vec::new(),
    };
    let mut pathspecs = Vec::new();
    let mut required = requires.is_none();
    let mut end_of_options = false;
    for (k, o) in operands.iter().enumerate().skip(sub + 1) {
        let t = o.text.as_str();
        if end_of_options || !t.starts_with('-') || t == "-" {
            pathspecs.push(k);
        } else if t == "--" {
            end_of_options = true;
        } else if !is_listed_flag(t, flags) {
            return Vec::new();
        } else if Some(t) == requires {
            required = true;
        }
    }
    if required { pathspecs } else { Vec::new() }
}
