//! `gsd-git`: records, in the plan's own file, every commit whose subject names a plan by GSD's convention
//! `{type}({phase}-{plan}): {task}` (`gsd-core/references/git-integration.md:72`).
//!
//! It is a writing collector: the host runs `git log` with the user's consent and hands the captured output to
//! `collect`, with the graph. The answer is an edit plan that creates one `commit` entity per new commit, in the file
//! of the plan it names. It records nothing it already holds (the 12-hex name is the key), so a second run is a no-op.
//! GSD has no commit trailer, so a commit names its task by the subject alone: the description after `: ` equals the
//! task's title. A commit that names no plan (`feat(gsd): x`, `docs(pause): y`, a scope no plan has, or one that
//! several plans have) is skipped, as `/gsd-undo` skips it (`scope === id`).
//!
//! Everything taken from `git log` is untrusted. Records start at a NUL (`%x00`), which no commit message can hold, so
//! a subject cannot forge one; control characters in a subject or a path become U+FFFD; a record whose hash is not
//! hex or whose time is out of range is skipped and named in the plan's title. The host refuses a whole plan for one
//! value it cannot write, so one bad commit must never reach it.
//!
//! Also ports gsd-core bin/lib modules: git-base-branch.cjs.

mod subject;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::graph::find_kind;
use crate::time::utc;

const NAME: &str = "gsd-git";

/// One record separator (`%x00`), and the unit separator between the fields of the header line (`%x1f`).
const RECORD: char = '\0';
const FIELD: char = '\u{1f}';

pub fn declare(c: &mut ContributionsBuilder) {
    c.collector(NAME, |k| {
        k.input_format("git-log")
            .run(&[
                "git",
                "-c",
                "core.quotepath=off",
                "-c",
                "i18n.logOutputEncoding=UTF-8",
                "log",
                "--no-merges",
                "--no-renames",
                "--numstat",
                "--summary",
                "--format=%x00%H%x1f%at%x1f%s",
            ])
            .capture_stdout()
            .writes()
            .collect(collect);
    });
}

/// One commit of the log.
#[derive(Debug, PartialEq, Eq)]
pub struct Commit {
    pub hash: String,
    pub at: String,
    pub subject: String,
    pub files: Vec<String>,
    pub created: Vec<String>,
    pub insertions: i64,
    pub deletions: i64,
}

/// A path as `git log` prints it: C-quoted (`"a\tb"`, octal escapes for bytes) when it holds a tab, a quote or a
/// backslash.
fn unquote(path: &str) -> String {
    let Some(inner) = path.strip_prefix('"').and_then(|p| p.strip_suffix('"')) else {
        return clean(path);
    };
    let mut bytes = Vec::new();
    let mut chars = inner.bytes().peekable();
    while let Some(b) = chars.next() {
        if b != b'\\' {
            bytes.push(b);
            continue;
        }
        match chars.next() {
            Some(b'n') => bytes.push(b'\n'),
            Some(b't') => bytes.push(b'\t'),
            Some(b'r') => bytes.push(b'\r'),
            Some(b'a') => bytes.push(0x07),
            Some(b'b') => bytes.push(0x08),
            Some(b'f') => bytes.push(0x0c),
            Some(b'v') => bytes.push(0x0b),
            Some(digit @ b'0'..=b'7') => {
                let mut value = u32::from(digit - b'0');
                for _ in 0..2 {
                    match chars.peek() {
                        Some(next @ b'0'..=b'7') => {
                            value = value * 8 + u32::from(next - b'0');
                            chars.next();
                        }
                        _ => break,
                    }
                }
                bytes.push(u8::try_from(value & 0xff).unwrap_or(b'?'));
            }
            Some(other) => bytes.push(other),
            None => bytes.push(b'\\'),
        }
    }
    clean(&String::from_utf8_lossy(&bytes))
}

/// `text` with every control character (ANSI escapes, DEL, U+0085) and U+2028/2029 replaced by U+FFFD: the host
/// cannot write them back unchanged. A tab and a newline survive (the writer quotes them).
fn clean(text: &str) -> String {
    text.chars()
        .map(|c| {
            if (c.is_control() && c != '\t' && c != '\n') || matches!(c, '\u{2028}' | '\u{2029}') {
                '\u{fffd}'
            } else {
                c
            }
        })
        .collect()
}

/// A full object name: 40 (SHA-1) or 64 (SHA-256) lower-case hex digits.
fn is_hash(text: &str) -> bool {
    matches!(text.len(), 40 | 64) && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// The commits of `git log --numstat --summary --format=%x00%H%x1f%at%x1f%s`, newest first as git prints them, and
/// the hash (as far as it is printable) of each record that is not a valid commit: a hash that is not hex, a time that
/// is not in `0..=253402300799` (the years 1970 to 9999). A binary file counts 0 lines.
pub fn parse_all(log: &str) -> (Vec<Commit>, Vec<String>) {
    let (mut commits, mut skipped) = (Vec::new(), Vec::new());
    for text in log.split(RECORD).filter(|t| !t.trim().is_empty()) {
        match record(text) {
            Ok(commit) => commits.push(commit),
            Err(name) => skipped.push(name),
        }
    }
    (commits, skipped)
}

#[cfg(test)]
pub fn parse(log: &str) -> Vec<Commit> {
    parse_all(log).0
}

fn record(text: &str) -> Result<Commit, String> {
    let mut lines = text.lines();
    let mut head = lines.next().unwrap_or_default().splitn(3, FIELD);
    let (hash, at, subject) = (
        head.next().unwrap_or_default().trim(),
        head.next().unwrap_or_default().trim(),
        head.next().unwrap_or_default(),
    );
    let time = at
        .parse::<i64>()
        .ok()
        .filter(|t| (0..=253_402_300_799).contains(t));
    let (true, Some(time)) = (is_hash(hash), time) else {
        return Err(clean(&hash.chars().take(12).collect::<String>()));
    };
    let mut commit = Commit {
        hash: hash.to_string(),
        at: utc(time),
        subject: clean(subject),
        files: Vec::new(),
        created: Vec::new(),
        insertions: 0,
        deletions: 0,
    };
    for line in lines {
        if let Some(rest) = line.strip_prefix(" create mode ") {
            // `create mode 100644 path`: the mode, then the path (which may hold spaces).
            if let Some((_, path)) = rest.split_once(' ') {
                commit.created.push(unquote(path));
            }
            continue;
        }
        let mut columns = line.splitn(3, '\t');
        if let (Some(added), Some(deleted), Some(path)) =
            (columns.next(), columns.next(), columns.next())
        {
            commit.insertions += added.parse::<i64>().unwrap_or(0);
            commit.deletions += deleted.parse::<i64>().unwrap_or(0);
            commit.files.push(unquote(path));
        }
    }
    Ok(commit)
}

/// `title` compared as GSD compares a task description: trimmed, case-folded, whitespace runs collapsed.
fn folded(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// A task description as the executor words it (`gsd-executor.md:565`, `git-integration.md`): without a squash-merge's
/// ` (#12)`, and without the TDD verbs `add failing test for ` and `implement `.
fn described(description: &str) -> String {
    let mut text = folded(description);
    if let Some((head, tail)) = text.rsplit_once(" (#")
        && tail
            .strip_suffix(')')
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    {
        text = head.to_string();
    }
    for verb in ["add failing test for ", "implement "] {
        if let Some(rest) = text.strip_prefix(verb) {
            return rest.to_string();
        }
    }
    text
}

/// The one task of `plan` that `description` names: its title equals it, or the description holds the title (after
/// `described`). Two candidates name none.
fn task_of<'a>(nodes: &'a [GraphNode], plan: &str, description: &str) -> Option<&'a str> {
    let wanted = described(description);
    let mut titled = nodes.iter().filter(|n| {
        n.kind == "task"
            && n.id
                .strip_prefix(plan)
                .is_some_and(|rest| rest.starts_with('/') && !rest[1..].contains('/'))
            && n.title
                .as_deref()
                .map(folded)
                .is_some_and(|t| !t.is_empty() && (t == wanted || wanted.contains(&t)))
    });
    match (titled.next(), titled.next()) {
        (Some(one), None) => Some(one.id.as_str()),
        _ => None,
    }
}

/// The `commit` fields of `commit`, naming `plan` (and `task`, when its description is a task's title).
fn fields(commit: &Commit, kind: &str, plan: &str, task: Option<&str>) -> BTreeMap<String, Value> {
    let mut fields = BTreeMap::new();
    fields.insert("hash".to_string(), json!(commit.hash));
    if let Some(task) = task {
        fields.insert("task".to_string(), json!(task));
    }
    fields.insert("plan".to_string(), json!(plan));
    fields.insert("type".to_string(), json!(kind));
    fields.insert("subject".to_string(), json!(commit.subject));
    fields.insert("at".to_string(), json!(commit.at));
    if !commit.files.is_empty() {
        fields.insert("files".to_string(), json!(commit.files));
    }
    if !commit.created.is_empty() {
        fields.insert("created".to_string(), json!(commit.created));
    }
    // The host's plan op refuses a JSON number for a top-level integer field (`edit/write.rs`: "takes text") and
    // reads the decimal text, as `specforge set <id> insertions 10` does.
    fields.insert(
        "insertions".to_string(),
        json!(commit.insertions.to_string()),
    );
    fields.insert("deletions".to_string(), json!(commit.deletions.to_string()));
    fields
}

fn collect(input: &CollectInput) -> Result<CollectOutput, String> {
    let nodes = input.graph.as_ref().map_or(&[][..], |g| &g.nodes[..]);
    let mut known: BTreeSet<String> = nodes
        .iter()
        .filter(|n| n.kind == "commit")
        .map(|n| n.id.clone())
        .collect();
    let mut builder = PlanBuilder::for_collect(input);
    let mut recorded = 0;
    let (commits, mut skipped) = parse_all(input.stdout.as_deref().unwrap_or(""));
    // Oldest first, so the entities land in the order the commits were made.
    for commit in commits.iter().rev() {
        let name: String = commit.hash.chars().take(12).collect();
        if known.contains(&name) {
            continue;
        }
        let Some(header) = subject::parse(&commit.subject) else {
            continue;
        };
        let Some(plan) = header
            .scope
            .and_then(|scope| find_kind(nodes, scope, "plan"))
        else {
            continue;
        };
        let Some(file) = plan.file.as_deref() else {
            continue;
        };
        // A revert undoes the work: it is recorded as one, and names no task.
        let reverts = commit.subject.starts_with("Revert \"");
        let task = if reverts {
            None
        } else {
            task_of(nodes, &plan.id, header.description)
        };
        let kind = if reverts { "revert" } else { &header.kind };
        builder.create(
            "commit",
            &name,
            &commit.subject,
            file,
            fields(commit, kind, &plan.id, task),
            Vec::new(),
        );
        known.insert(name);
        recorded += 1;
    }
    skipped.dedup();
    let mut output = CollectOutput::default();
    if recorded > 0 {
        let note = if skipped.is_empty() {
            String::new()
        } else {
            format!(
                " (skipped {} with an unreadable header: {})",
                skipped.len(),
                skipped.join(", ")
            )
        };
        builder.title(&format!(
            "Record {recorded} commit{}{note}",
            if recorded == 1 { "" } else { "s" }
        ));
        output.plan = Some(builder.finish());
    }
    Ok(output)
}
