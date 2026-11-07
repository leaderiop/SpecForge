//! Where a verify command runs, read from its text and never executed: a port of `resolveVerifyCommandTarget`
//! (`src/verify-command-grounding.cts`) up to the point where it touches the disk. The guest has no disk, so it only
//! names the paths to look at; the host reads them and the check pass judges the facts.
//!
//! shortcut: the guest does not know the project root, so an absolute target, or one that climbs out of the project,
//! is `Outside` whether or not it exists (GSD probes a climb that names a sibling, and lets an absolute path inside
//! the root through); upgrade when the host sends the root.
//!
//! gsd-core modules this file ports: verify-command-grounding.cjs.

use crate::relocation::{Event, dynamic, events, missing_sentinel};

/// What a command's text says about where it runs.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    /// No `cd` or `--prefix` target, an empty command, or the `MISSING` sentinel.
    Skip,
    /// The target holds `$`, a backtick, `*`, `?`, a newline or a leading `~`.
    Dynamic,
    /// An absolute target, or one that climbs out of the project.
    Outside,
    At(Target),
}

/// A directory the command runs in, and the manifest its tool needs there.
#[derive(Debug, PartialEq, Eq)]
pub struct Target {
    /// Project-relative, lexically normalised; empty is the project root.
    pub dir: String,
    pub need: Need,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Need {
    Nothing,
    /// `package.json`, and the script `npm run <script>` names.
    Package(Option<String>),
    Makefile,
}

impl Target {
    /// The manifest's project-relative path.
    pub fn manifest(&self) -> Option<String> {
        let name = match self.need {
            Need::Nothing => return None,
            Need::Package(_) => "package.json",
            Need::Makefile => "Makefile",
        };
        Some(if self.dir.is_empty() {
            name.to_string()
        } else {
            format!("{}/{name}", self.dir)
        })
    }

    /// Every path the host must read for this target, the directory first.
    pub fn paths(&self) -> Vec<String> {
        let dir = (!self.dir.is_empty()).then(|| self.dir.clone());
        dir.into_iter().chain(self.manifest()).collect()
    }
}

pub fn resolve(command: &str) -> Verdict {
    let trimmed = command.trim();
    if trimmed.is_empty() || missing_sentinel(trimmed) {
        return Verdict::Skip;
    }
    // Words the shell could not read name no directory that can be checked.
    let Ok(events) = events(trimmed) else {
        return Verdict::Dynamic;
    };
    let (cds, rest) = leading_cds(&events);
    let raw = if cds.is_empty() {
        let prefix = events.iter().find_map(|event| match event {
            Event::Flag { flag, target } if flag == "--prefix" => Some(target.clone()),
            _ => None,
        });
        match prefix {
            Some(prefix) => prefix,
            None => return Verdict::Skip,
        }
    } else {
        fold_cd(&cds)
    };
    if dynamic(&raw) || raw.contains('\n') {
        return Verdict::Dynamic;
    }
    let Some(dir) = lexical(&to_slash(&raw)) else {
        return Verdict::Outside;
    };
    let first = rest.iter().find_map(|event| match event {
        Event::Run(words) => Some(words.as_slice()),
        _ => None,
    });
    Verdict::At(Target {
        dir,
        need: first.map_or(Need::Nothing, need),
    })
}

/// The `cd`s a command starts with, and the events after them. A group before a `cd` moves nothing here (`echo
/// "$(pwd)"; cd x` does not, and neither does `cd "$(pwd)/x"`'s substitution).
fn leading_cds(events: &[Event]) -> (Vec<&str>, &[Event]) {
    let mut cds = Vec::new();
    let mut rest = events;
    loop {
        match rest.first() {
            Some(Event::Cd(target)) => {
                cds.push(target.as_str());
                rest = &rest[1..];
            }
            Some(Event::Enter) => {
                let after = past_group(rest);
                if !matches!(after.first(), Some(Event::Cd(_))) {
                    return (cds, rest);
                }
                rest = after;
            }
            _ => return (cds, rest),
        }
    }
}

/// `events` past the group its first event opens.
fn past_group(events: &[Event]) -> &[Event] {
    let mut open = 0usize;
    for (at, event) in events.iter().enumerate() {
        match event {
            Event::Enter => open += 1,
            Event::Leave => {
                open -= 1;
                if open == 0 {
                    return &events[at + 1..];
                }
            }
            _ => {}
        }
    }
    &[]
}

fn need(words: &[String]) -> Need {
    let tool = words
        .first()
        .map_or("", |word| word.rsplit('/').next().unwrap_or(word));
    if ["npm", "npx", "pnpm", "yarn", "bun"].contains(&tool) {
        Need::Package(npm_script(tool, &words[1..]))
    } else if tool == "make" {
        Need::Makefile
    } else {
        Need::Nothing
    }
}

fn to_slash(text: &str) -> String {
    text.replace('\\', "/")
}

/// Chained `cd`s left to right: a relative segment appends, an absolute one starts over.
fn fold_cd(args: &[&str]) -> String {
    args.iter().fold(String::new(), |acc, raw| {
        if acc.is_empty() || to_slash(raw).starts_with('/') {
            (*raw).to_string()
        } else {
            format!("{acc}/{raw}")
        }
    })
}

/// `path` without `.` and `//`, `..` folded; `None` when it is absolute or climbs out of the project.
fn lexical(path: &str) -> Option<String> {
    if path.starts_with('/') {
        return None;
    }
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

/// The script of `npm run <script>`, with `--prefix` flags before the `run` read past.
fn npm_script(tool: &str, args: &[String]) -> Option<String> {
    if tool != "npm" {
        return None;
    }
    let mut args = args.iter().map(String::as_str);
    let mut word = args.next()?;
    while word == "--prefix" || word.starts_with("--prefix=") {
        if word == "--prefix" {
            args.next()?;
        }
        word = args.next()?;
    }
    if word != "run" {
        return None;
    }
    let script: String = args
        .next()?
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || "_:@./-".contains(*c))
        .collect();
    (!script.is_empty()).then_some(script)
}

#[cfg(test)]
mod tests;
