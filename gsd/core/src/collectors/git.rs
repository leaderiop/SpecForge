//! `gsd-git`: records, in the plan's own file, every commit whose subject names a plan by GSD's convention
//! `{type}({phase}-{plan}): {task}` (`gsd-core/references/git-integration.md:72`).
//!
//! It is a writing collector: the host runs `git log` with the user's consent and hands the captured output to
//! `collect`, with the graph. The answer is an edit plan that creates one `commit` entity per new commit, in the file
//! of the plan it names. It records nothing it already holds (the 12-hex name is the key), so a second run is a no-op.
//! GSD has no commit trailer, so a commit names its task by the subject alone: the description after `: ` equals the
//! task's title. A commit that names no plan (`feat(gsd): x`, `docs(pause): y`, a scope no plan has, or one that
//! several plans have) is skipped, as `/gsd-undo` skips it (`scope === id`).

mod subject;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::graph::find_kind;

const NAME: &str = "gsd-git";

/// One record separator (`%x1e`), and the unit separator between the fields of the header line (`%x1f`).
const RECORD: char = '\u{1e}';
const FIELD: char = '\u{1f}';

pub fn declare(c: &mut ContributionsBuilder) {
    c.collector(NAME, |k| {
        k.input_format("git-log")
            .run(&[
                "git",
                "-c",
                "core.quotepath=off",
                "log",
                "--no-merges",
                "--no-renames",
                "--numstat",
                "--summary",
                "--format=%x1e%H%x1f%at%x1f%s",
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

/// `seconds` since the epoch as `YYYY-MM-DDTHH:MM:SSZ` (the days-from-civil algorithm, proleptic Gregorian).
pub fn utc(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let rest = seconds.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// A path as `git log` prints it: C-quoted (`"a\tb"`, octal escapes for bytes) when it holds a tab, a quote or a
/// backslash.
fn unquote(path: &str) -> String {
    let Some(inner) = path.strip_prefix('"').and_then(|p| p.strip_suffix('"')) else {
        return path.to_string();
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
    String::from_utf8_lossy(&bytes).into_owned()
}

/// The commits of `git log --numstat --summary --format=%x1e%H%x1f%at%x1f%s`, newest first as git prints them. A record
/// that is not a header is dropped; a binary file counts 0 lines.
pub fn parse(log: &str) -> Vec<Commit> {
    log.split(RECORD).filter_map(record).collect()
}

fn record(text: &str) -> Option<Commit> {
    let mut lines = text.lines();
    let mut head = lines.next()?.splitn(3, FIELD);
    let (hash, at, subject) = (head.next()?, head.next()?, head.next()?);
    let mut commit = Commit {
        hash: hash.trim().to_string(),
        at: utc(at.trim().parse().ok()?),
        subject: subject.to_string(),
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
    (!commit.hash.is_empty()).then_some(commit)
}

/// `title` compared as GSD compares a task description: trimmed, case-folded, whitespace runs collapsed.
fn folded(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
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
    // Oldest first, so the entities land in the order the commits were made.
    for commit in parse(input.stdout.as_deref().unwrap_or("")).iter().rev() {
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
        let wanted = folded(header.description);
        let mut titled = nodes.iter().filter(|n| {
            n.kind == "task"
                && n.id
                    .strip_prefix(&plan.id)
                    .is_some_and(|rest| rest.starts_with('/') && !rest[1..].contains('/'))
                && n.title.as_deref().is_some_and(|t| folded(t) == wanted)
        });
        let task = match (titled.next(), titled.next()) {
            (Some(one), None) => Some(one.id.as_str()),
            _ => None,
        };
        builder.create(
            "commit",
            &name,
            &commit.subject,
            file,
            fields(commit, &header.kind, &plan.id, task),
            Vec::new(),
        );
        known.insert(name);
        recorded += 1;
    }
    let mut output = CollectOutput::default();
    if recorded > 0 {
        builder.title(&format!(
            "Record {recorded} commit{}",
            if recorded == 1 { "" } else { "s" }
        ));
        output.plan = Some(builder.finish());
    }
    Ok(output)
}
